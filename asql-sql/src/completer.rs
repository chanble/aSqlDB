use std::sync::Arc;

use crate::db_type::DatabaseType;
use crate::parser::{AliasMap, CompletionContext, CompletionRequest};
use crate::provider::SchemaProvider;
use crate::suggestion::Suggestion;

/// Maximum number of table suggestions returned in a single completion call.
const TABLE_LIMIT: usize = 200;

/// Keywords to suggest after a completed table name in FROM/JOIN clause.
const KEYWORDS_AFTER_TABLE: &[&str] = &[
    "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "CROSS", "OUTER", "FULL",
    "ON", "GROUP", "ORDER", "HAVING", "LIMIT", "OFFSET",
    "AS", "UNION", "ALL", "AND", "OR",
];

pub struct Completer<P: SchemaProvider> {
    db_type: DatabaseType,
    provider: Arc<P>,
}

impl<P: SchemaProvider> Completer<P> {
    pub fn new(db_type: DatabaseType, provider: Arc<P>) -> Self {
        Self { db_type, provider }
    }

    pub async fn get_suggestions(&self, request: &CompletionRequest) -> Vec<Suggestion> {
        let ctx = request.detect_context();
        let input = request.prefix_for_completion();
        let input_lower = input.to_lowercase();
        let aliases = request.extract_aliases();


        let mut suggestions = match ctx {
            CompletionContext::Start => self.suggest_keywords(input),

            CompletionContext::AfterSelect => self.suggest_select_context(&aliases, input).await,

            CompletionContext::AfterFrom | CompletionContext::AfterJoin => {
                if input_lower.is_empty() {
                    // After a space → check what came before it.
                    if self.is_table_introducer(request.text_before_cursor()) {
                        // User just typed "FROM " or "JOIN " → suggest tables
                        let mut s = self.suggest_tables("").await;
                        s.extend(self.suggest_columns_with_aliases(&aliases, "").await);
                        s.extend(self.suggest_functions(""));
                        s.extend(self.suggest_keywords(""));
                        s
                    } else {
                        // User typed "table_name " → only keywords after table
                        self.suggest_keywords_after_table("")
                    }
                } else {
                    let text = request.text_before_cursor();
                    let stripped = &text[..text.len().saturating_sub(input.len())];
                    if self.is_table_introducer(stripped) {
                        self.suggest_tables(input).await
                    } else {
                        self.suggest_keywords_after_table(input)
                    }
                }
            }

            CompletionContext::AfterWhere
            | CompletionContext::AfterOrderBy
            | CompletionContext::AfterGroupBy
            | CompletionContext::AfterHaving => self.suggest_where_context(&aliases, input).await,

            CompletionContext::AfterLimit => self.suggest_keywords(input),

            // SET context without a known table — fall back to keyword-only.
            CompletionContext::AfterSet => self.suggest_keywords(input),

            CompletionContext::AfterInsertInto => self.suggest_tables(input).await,

            CompletionContext::AfterDot { ref prefix } => {
                self.suggest_columns_for_prefix(prefix, &aliases).await
            }

            CompletionContext::InsideFunction | CompletionContext::Unknown => {
                self.suggest_general_context(&aliases, input).await
            }

            CompletionContext::Subquery(inner) => {
                self.suggest_subquery(inner.as_deref(), &aliases, input).await
            }

            CompletionContext::AfterCreateTable => self.suggest_tables(input).await,

            CompletionContext::AfterAlterTable => self.suggest_tables(input).await,

            CompletionContext::AfterDrop => {
                let mut s = self.suggest_tables(input).await;
                s.extend(self.suggest_keywords(input));
                s
            }

            CompletionContext::CreateTableColumnName => {
                let mut s = Vec::new();
                s.extend(self.suggest_column_types(input));
                s.extend(self.suggest_constraint_keywords());
                s
            }

            CompletionContext::CreateTableColumnType => {
                let mut s = Vec::new();
                s.extend(self.suggest_column_types(input));
                s.extend(self.suggest_constraint_keywords());
                s
            }

            CompletionContext::CreateTableConstraint => {
                let mut s = Vec::new();
                s.extend(self.suggest_constraint_keywords());
                s.extend(self.suggest_column_types(input));
                s
            }

            CompletionContext::AfterAddColumn => self.suggest_column_types(input),

            CompletionContext::AfterDropColumn | CompletionContext::AfterModifyColumn => {
                let mut s = self.suggest_tables(input).await;
                s.extend(self.suggest_keywords(input));
                s
            }
        };

        if !input_lower.is_empty() {
            suggestions.retain(|s| s.text.to_lowercase().contains(&input_lower));
        }

        suggestions
    }

    async fn suggest_select_context(&self, aliases: &AliasMap, input: &str) -> Vec<Suggestion> {
        let mut s = Vec::new();
        s.push(Suggestion::keyword("*"));
        s.extend(self.suggest_columns_with_aliases(aliases, input).await);
        s.extend(self.suggest_functions(input));
        s.extend(self.suggest_keywords(input));
        s
    }

    async fn suggest_where_context(&self, aliases: &AliasMap, input: &str) -> Vec<Suggestion> {
        let mut s = Vec::new();
        s.extend(self.suggest_columns_with_aliases(aliases, input).await);
        s.extend(self.suggest_functions(input));
        s.extend(self.suggest_keywords(input));
        s
    }

    async fn suggest_general_context(&self, aliases: &AliasMap, input: &str) -> Vec<Suggestion> {
        let mut s = Vec::new();
        s.extend(self.suggest_tables(input).await);
        s.extend(self.suggest_columns_with_aliases(aliases, input).await);
        s.extend(self.suggest_functions(input));
        s.extend(self.suggest_keywords(input));
        s
    }

    async fn suggest_subquery(
        &self,
        inner: Option<&CompletionContext>,
        aliases: &AliasMap,
        input: &str,
    ) -> Vec<Suggestion> {
        match inner {
            Some(CompletionContext::AfterSelect) => self.suggest_select_context(aliases, input).await,
            Some(CompletionContext::AfterFrom) | Some(CompletionContext::AfterJoin) => {
                let mut s = self.suggest_tables(input).await;
                s.extend(self.suggest_keywords(input));
                s
            }
            Some(CompletionContext::AfterWhere)
            | Some(CompletionContext::AfterOrderBy)
            | Some(CompletionContext::AfterGroupBy)
            | Some(CompletionContext::AfterHaving) => self.suggest_where_context(aliases, input).await,
            _ => self.suggest_select_context(aliases, input).await,
        }
    }

    fn suggest_keywords_after_table(&self, input: &str) -> Vec<Suggestion> {
        let input_lower = input.to_lowercase();
        KEYWORDS_AFTER_TABLE
            .iter()
            .filter(|kw| input_lower.is_empty() || kw.to_lowercase().starts_with(&input_lower))
            .map(|kw| Suggestion::keyword(*kw))
            .collect()
    }

    /// Check if the text before cursor ends with a keyword that introduces
    /// a table reference (FROM, JOIN, etc.) or a comma — meaning the user
    /// still needs to pick/type a table name.
    fn is_table_introducer(&self, text_before_cursor: &str) -> bool {
        let trimmed = text_before_cursor.trim_end();
        let last_word = trimmed.split_whitespace().last().unwrap_or("");
        let upper = last_word.to_uppercase();
        matches!(
            upper.as_str(),
            "FROM" | "JOIN"
                | "INNER" | "LEFT" | "RIGHT" | "CROSS" | "OUTER" | "FULL"
                | "STRAIGHT_JOIN" | "NATURAL"
                | "INTO"
                | "TABLE"
                | "SET"
        ) || trimmed.ends_with(',')
    }

    fn suggest_keywords(&self, input: &str) -> Vec<Suggestion> {
        let input_lower = input.to_lowercase();
        self.db_type
            .keywords()
            .iter()
            .filter(|kw| input_lower.is_empty() || kw.to_lowercase().starts_with(&input_lower))
            .map(|kw| Suggestion::keyword(*kw))
            .collect()
    }

    fn suggest_functions(&self, input: &str) -> Vec<Suggestion> {
        let input_lower = input.to_lowercase();
        self.db_type
            .functions()
            .iter()
            .filter(|f| input_lower.is_empty() || f.to_lowercase().starts_with(&input_lower))
            .map(|f| Suggestion::function(*f))
            .collect()
    }

    async fn suggest_tables(&self, input: &str) -> Vec<Suggestion> {
        self.provider
            .table_names(input, TABLE_LIMIT)
            .await
            .into_iter()
            .map(Suggestion::table)
            .collect()
    }

    async fn suggest_columns_with_aliases(
        &self,
        aliases: &AliasMap,
        _input: &str,
    ) -> Vec<Suggestion> {
        let table_refs = aliases.all_table_refs();
        if table_refs.is_empty() {
            return Vec::new();
        }
        let tables: Vec<String> = table_refs
            .iter()
            .map(|r| aliases.resolve_table(r).unwrap_or(r).to_string())
            .collect();
        self.provider
            .columns_for(&tables)
            .await
            .into_iter()
            .map(|(name, _)| Suggestion::column(name))
            .collect()
    }

    async fn suggest_columns_for_prefix(
        &self,
        prefix: &str,
        aliases: &AliasMap,
    ) -> Vec<Suggestion> {
        let table = aliases
            .resolve_table(prefix)
            .map(|s| s.to_string())
            .unwrap_or_else(|| prefix.to_string());

        self.provider
            .columns(&table)
            .await
            .into_iter()
            .map(|(name, _)| Suggestion::column(name))
            .collect()
    }

    fn suggest_column_types(&self, input: &str) -> Vec<Suggestion> {
        let input_lower = input.to_lowercase();
        self.db_type
            .column_types()
            .iter()
            .filter(|t| input_lower.is_empty() || t.to_lowercase().starts_with(&input_lower))
            .map(|t| Suggestion::keyword(*t))
            .collect()
    }

    fn suggest_constraint_keywords(&self) -> Vec<Suggestion> {
        self.db_type
            .constraint_keywords()
            .iter()
            .map(|kw| Suggestion::keyword(*kw))
            .collect()
    }
}

pub async fn get_suggestions<P: SchemaProvider>(
    db_type: DatabaseType,
    provider: Arc<P>,
    sql: &str,
    cursor_position: usize,
) -> Vec<Suggestion> {
    let completer = Completer::new(db_type, provider);
    let request = CompletionRequest::new(sql, cursor_position);
    completer.get_suggestions(&request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::SchemaProvider;
    use crate::suggestion::SuggestionKind;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Mock provider that records which methods were called.
    struct MockProvider {
        tables: Vec<String>,
        columns_called: Mutex<bool>,
        columns_for_called: Mutex<bool>,
    }

    #[async_trait]
    impl SchemaProvider for MockProvider {
        async fn table_names(&self, prefix: &str, limit: usize) -> Vec<String> {
            let pl = prefix.to_lowercase();
            self.tables
                .iter()
                .filter(|t| pl.is_empty() || t.to_lowercase().contains(&pl))
                .take(limit)
                .cloned()
                .collect()
        }
        async fn columns(&self, _table: &str) -> Vec<(String, String)> {
            *self.columns_called.lock().unwrap() = true;
            Vec::new()
        }
        async fn columns_for(&self, _tables: &[String]) -> Vec<(String, String)> {
            *self.columns_for_called.lock().unwrap() = true;
            Vec::new()
        }
    }

    fn mock(tables: &[&str]) -> Arc<MockProvider> {
        Arc::new(MockProvider {
            tables: tables.iter().map(|s| s.to_string()).collect(),
            columns_called: Mutex::new(false),
            columns_for_called: Mutex::new(false),
        })
    }

    #[tokio::test]
    async fn partial_table_name_only_suggests_tables() {
        let p = mock(&["sai_qixi_festival26_cp_log", "sai_qixi_festival26_gift_log"]);
        let sql = "SELECT * FROM sai_qixi_festival2";
        let suggestions = get_suggestions(DatabaseType::MySql, p.clone(), sql, sql.len()).await;

        assert!(*p.columns_called.lock().unwrap() == false);
        assert!(*p.columns_for_called.lock().unwrap() == false);
        assert!(suggestions.iter().all(|s| s.kind == SuggestionKind::Table));
        assert_eq!(suggestions.len(), 2);
    }

    #[tokio::test]
    async fn substring_table_name_not_prefix_still_suggests() {
        let p = mock(&["sai_qixi_festival26_cp_log", "sai_qixi_festival26_gift_log"]);
        let sql = "SELECT * FROM qixi_festival26";
        let suggestions = get_suggestions(DatabaseType::MySql, p.clone(), sql, sql.len()).await;

        assert!(suggestions.iter().all(|s| s.kind == SuggestionKind::Table));
        assert_eq!(suggestions.len(), 2);
    }

    #[tokio::test]
    async fn complete_table_plus_space_only_keywords() {
        let p = mock(&["sai_qixi_festival26_cp_log"]);
        let sql = "SELECT * FROM sai_qixi_festival26_cp_log ";
        let suggestions = get_suggestions(DatabaseType::MySql, p.clone(), sql, sql.len()).await;

        assert!(*p.columns_called.lock().unwrap() == false);
        assert!(*p.columns_for_called.lock().unwrap() == false);
        assert!(suggestions.iter().all(|s| s.kind == SuggestionKind::Keyword));
    }

    #[tokio::test]
    async fn keyword_after_table_with_prefix_suggests_keywords() {
        let p = mock(&["sai_qixi_festival26_cp_log"]);
        let sql = "SELECT * FROM sai_qixi_festival26_cp_log limi";
        let suggestions = get_suggestions(DatabaseType::MySql, p.clone(), sql, sql.len()).await;

        assert!(suggestions.iter().any(|s| s.text == "LIMIT" && s.kind == SuggestionKind::Keyword));
        assert!(!suggestions.iter().any(|s| s.kind == SuggestionKind::Table));
    }
}
