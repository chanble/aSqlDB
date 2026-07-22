use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    Start,
    AfterSelect,
    AfterFrom,
    AfterJoin,
    AfterWhere,
    AfterOrderBy,
    AfterGroupBy,
    AfterHaving,
    AfterLimit,
    AfterSet,
    AfterInsertInto,
    AfterDot { prefix: String },
    InsideFunction,
    Subquery(Option<Box<CompletionContext>>),
    AfterCreateTable,
    AfterAlterTable,
    AfterDrop,
    CreateTableColumnName,
    CreateTableColumnType,
    CreateTableConstraint,
    AfterAddColumn,
    AfterDropColumn,
    AfterModifyColumn,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct AliasMap {
    alias_to_table: HashMap<String, String>,
    table_to_alias: HashMap<String, String>,
}

impl AliasMap {
    pub fn new() -> Self {
        Self {
            alias_to_table: HashMap::new(),
            table_to_alias: HashMap::new(),
        }
    }

    pub fn insert(&mut self, table: String, alias: String) {
        self.alias_to_table.insert(alias.clone(), table.clone());
        self.table_to_alias.insert(table, alias);
    }

    pub fn resolve_table<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if let Some(table) = self.alias_to_table.get(name) {
            return Some(table.as_str());
        }
        if self.table_to_alias.contains_key(name) {
            return Some(name);
        }
        None
    }

    pub fn all_table_refs(&self) -> Vec<&str> {
        let mut refs: Vec<&str> = Vec::new();
        for alias in self.alias_to_table.keys() {
            refs.push(alias.as_str());
        }
        for table in self.table_to_alias.keys() {
            if !self.alias_to_table.contains_key(table) {
                refs.push(table.as_str());
            }
        }
        refs
    }
}

pub struct CompletionRequest {
    pub sql: String,
    pub cursor_position: usize,
}

impl CompletionRequest {
    pub fn new(sql: impl Into<String>, cursor_position: usize) -> Self {
        Self {
            sql: sql.into(),
            cursor_position,
        }
    }

    pub fn text_before_cursor(&self) -> &str {
        &self.sql[..self.cursor_position.min(self.sql.len())]
    }

    pub fn current_word(&self) -> &str {
        let text = self.text_before_cursor();
        if text.ends_with(|c: char| c.is_whitespace() || c == ',' || c == '(') {
            return "";
        }
        text.split_whitespace().last().unwrap_or("")
    }

    pub fn prefix_for_completion(&self) -> &str {
        let text = self.text_before_cursor();
        if text.ends_with(|c: char| c.is_whitespace() || c == ',' || c == '(') {
            return "";
        }
        let word = text.split_whitespace().last().unwrap_or("");
        if word.contains('.') {
            word.split('.').last().unwrap_or("")
        } else {
            word
        }
    }

    pub fn extract_dot_prefix(&self) -> Option<String> {
        let text = self.text_before_cursor();
        let trimmed = text.trim_end();
        if !trimmed.ends_with('.') {
            return None;
        }
        let before_dot = trimmed.trim_end_matches('.');
        let word = before_dot.split_whitespace().last().unwrap_or("");
        if word.is_empty() {
            return None;
        }
        Some(word.to_string())
    }

    pub fn extract_aliases(&self) -> AliasMap {
        let full_text = &self.sql;
        let tokens = tokenize_preserving_case(full_text);
        let mut aliases = AliasMap::new();

        let mut i = 0;
        while i < tokens.len() {
            let upper = tokens[i].to_uppercase();
            if upper == "FROM" || upper == "JOIN" || ends_with_join(&upper) {
                i += 1;
                if ends_with_join(&upper) && i < tokens.len() && tokens[i].to_uppercase() == "JOIN" {
                    i += 1;
                }
                loop {
                    if i >= tokens.len() {
                        break;
                    }
                    let u = tokens[i].to_uppercase();
                    if is_clause_keyword(&u) {
                        break;
                    }
                    let table_name = &tokens[i];
                    i += 1;
                    if i < tokens.len() {
                        let next_upper = tokens[i].to_uppercase();
                        if next_upper == "AS" {
                            i += 1;
                            if i < tokens.len() {
                                let alias = &tokens[i];
                                aliases.insert(table_name.clone(), alias.clone());
                                i += 1;
                            }
                        } else if !is_clause_keyword(&next_upper)
                            && !is_join_keyword(&next_upper)
                            && next_upper != "ON"
                            && next_upper != "AND"
                            && next_upper != "OR"
                            && next_upper != "WHERE"
                            && next_upper != "GROUP"
                            && next_upper != "ORDER"
                            && next_upper != "HAVING"
                            && next_upper != "LIMIT"
                        {
                            let alias = &tokens[i];
                            aliases.insert(table_name.clone(), alias.clone());
                            i += 1;
                        } else {
                            aliases.insert(table_name.clone(), table_name.clone());
                        }
                    } else {
                        aliases.insert(table_name.clone(), table_name.clone());
                    }
                    if i < tokens.len() {
                        let next_upper = tokens[i].to_uppercase();
                        if next_upper == "," {
                            i += 1;
                            continue;
                        }
                        if next_upper == "ON" {
                            i += 1;
                            while i < tokens.len() {
                                let uu = tokens[i].to_uppercase();
                                if is_clause_keyword(&uu) || is_join_keyword(&uu) {
                                    break;
                                }
                                i += 1;
                            }
                        }
                    }
                    break;
                }
            } else {
                i += 1;
            }
        }

        aliases
    }

    pub fn subquery_depth(&self) -> usize {
        let text = self.text_before_cursor();
        let mut depth = 0usize;
        let mut in_string = false;
        let mut string_char = ' ';

        for ch in text.chars() {
            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '\'' | '"' => {
                    in_string = true;
                    string_char = ch;
                }
                '(' => depth = depth.saturating_add(1),
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        depth
    }

    pub fn detect_context(&self) -> CompletionContext {
        let text = self.text_before_cursor();
        let _trimmed = text.trim_end();

        if let Some(prefix) = self.extract_dot_prefix() {
            return CompletionContext::AfterDot { prefix };
        }

        if self.is_ddl_context() {
            return self.detect_outer_context();
        }

        let depth = self.subquery_depth();
        if depth > 0 {
            let inner_ctx = self.detect_inner_context();
            return CompletionContext::Subquery(inner_ctx.map(Box::new));
        }

        self.detect_outer_context()
    }

    fn is_ddl_context(&self) -> bool {
        let text = self.text_before_cursor().to_uppercase();
        let tokens = tokenize(&text);
        tokens.contains(&"CREATE".to_string()) && tokens.contains(&"TABLE".to_string())
            || (tokens.contains(&"ALTER".to_string()) && tokens.contains(&"TABLE".to_string()))
    }

    fn detect_outer_context(&self) -> CompletionContext {
        let text = self.text_before_cursor().to_uppercase();
        let tokens = tokenize(&text);

        if tokens.is_empty() {
            return CompletionContext::Start;
        }

        let last_kw = last_keyword_str(&tokens);

        match last_kw {
            "SELECT" => CompletionContext::AfterSelect,
            "FROM" => CompletionContext::AfterFrom,
            k if is_join_context(k) => CompletionContext::AfterJoin,
            "WHERE" => CompletionContext::AfterWhere,
            "ORDER" | "BY" => CompletionContext::AfterOrderBy,
            "GROUP" => CompletionContext::AfterGroupBy,
            "HAVING" => CompletionContext::AfterHaving,
            "LIMIT" => CompletionContext::AfterLimit,
            "SET" => CompletionContext::AfterSet,
            "INTO" => {
                if tokens.contains(&"INSERT".to_string()) {
                    CompletionContext::AfterInsertInto
                } else {
                    CompletionContext::Unknown
                }
            }
            "TABLE" => {
                if tokens.contains(&"CREATE".to_string()) {
                    if let Some(table_idx) = tokens.iter().position(|t| t == "TABLE") {
                        if table_idx + 2 < tokens.len() {
                            let raw = self.text_before_cursor();
                            if raw.contains('(') {
                                return CompletionContext::CreateTableColumnType;
                            }
                        }
                    }
                    return CompletionContext::AfterCreateTable;
                }
                if tokens.contains(&"ALTER".to_string()) {
                    return CompletionContext::AfterAlterTable;
                }
                if tokens.contains(&"DROP".to_string()) {
                    return CompletionContext::AfterDrop;
                }
                CompletionContext::Unknown
            }
            "ADD" | "COLUMN" => CompletionContext::AfterAddColumn,
            "MODIFY" | "ALTER" => CompletionContext::AfterModifyColumn,
            "DROP" => {
                if tokens.contains(&"ALTER".to_string()) {
                    CompletionContext::AfterDropColumn
                } else {
                    CompletionContext::Unknown
                }
            }
            _ => {
                if detect_create_table_column_context(&tokens) {
                    CompletionContext::CreateTableColumnName
                } else if detect_create_table_type_context(&tokens) {
                    CompletionContext::CreateTableColumnType
                } else if detect_constraint_context(&tokens) {
                    CompletionContext::CreateTableConstraint
                } else {
                    CompletionContext::Unknown
                }
            }
        }
    }

    fn detect_inner_context(&self) -> Option<CompletionContext> {
        let text = self.text_before_cursor();
        let depth = self.subquery_depth();
        if depth == 0 {
            return None;
        }

        let mut start = 0usize;
        let mut current_depth = 0usize;
        let mut in_string = false;
        let mut string_char = ' ';

        for (i, ch) in text.char_indices() {
            if in_string {
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '\'' | '"' => {
                    in_string = true;
                    string_char = ch;
                }
                '(' => {
                    current_depth += 1;
                    if current_depth == depth {
                        start = i + 1;
                    }
                }
                ')' => {
                    current_depth -= 1;
                }
                _ => {}
            }
        }

        let inner_sql = &text[start..];
        let inner_upper = inner_sql.to_uppercase();
        let tokens = tokenize(&inner_upper);

        if tokens.is_empty() {
            return Some(CompletionContext::AfterSelect);
        }

        let last_kw = last_keyword_str(&tokens);

        match last_kw {
            "SELECT" => Some(CompletionContext::AfterSelect),
            "FROM" => Some(CompletionContext::AfterFrom),
            k if is_join_context(k) => Some(CompletionContext::AfterJoin),
            "WHERE" => Some(CompletionContext::AfterWhere),
            "ORDER" | "BY" => Some(CompletionContext::AfterOrderBy),
            "GROUP" => Some(CompletionContext::AfterGroupBy),
            "HAVING" => Some(CompletionContext::AfterHaving),
            "LIMIT" => Some(CompletionContext::AfterLimit),
            _ => None,
        }
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_whitespace() || ch == ',' || ch == '(' || ch == ')' || ch == ';' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn tokenize_preserving_case(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = ' ';

    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if ch == string_char {
                in_string = false;
            }
            continue;
        }
        if ch.is_whitespace() || ch == ',' || ch == ';' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else if ch == '(' || ch == ')' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else if ch == '\'' || ch == '"' {
            in_string = true;
            string_char = ch;
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn last_keyword_str(tokens: &[String]) -> &str {
    tokens
        .iter()
        .rev()
        .find(|t| is_keyword_token(t.as_str()))
        .map(|s| s.as_str())
        .unwrap_or("")
}

fn is_keyword_token(token: &str) -> bool {
    matches!(
        token,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "INSERT"
            | "INTO"
            | "UPDATE"
            | "DELETE"
            | "CREATE"
            | "DROP"
            | "ALTER"
            | "TABLE"
            | "INDEX"
            | "VIEW"
            | "JOIN"
            | "INNER"
            | "LEFT"
            | "RIGHT"
            | "OUTER"
            | "CROSS"
            | "FULL"
            | "ON"
            | "ORDER"
            | "BY"
            | "GROUP"
            | "HAVING"
            | "LIMIT"
            | "OFFSET"
            | "SET"
            | "VALUES"
            | "AND"
            | "OR"
            | "NOT"
            | "IN"
            | "IS"
            | "NULL"
            | "LIKE"
            | "BETWEEN"
            | "EXISTS"
            | "AS"
            | "DISTINCT"
            | "UNION"
            | "ALL"
            | "CASE"
            | "WHEN"
            | "THEN"
            | "ELSE"
            | "END"
            | "ASC"
            | "DESC"
            | "ADD"
            | "COLUMN"
            | "MODIFY"
            | "CONSTRAINT"
            | "PRIMARY"
            | "FOREIGN"
            | "REFERENCES"
            | "UNIQUE"
            | "CHECK"
            | "DEFAULT"
            | "IF"
            | "REPLACE"
            | "INT"
            | "INTEGER"
            | "VARCHAR"
            | "TEXT"
            | "BOOLEAN"
            | "BOOL"
            | "DATE"
            | "TIMESTAMP"
            | "FLOAT"
            | "DOUBLE"
            | "DECIMAL"
            | "CHAR"
            | "BLOB"
            | "REAL"
            | "BIGINT"
            | "SMALLINT"
            | "TINYINT"
            | "SERIAL"
            | "BIGSERIAL"
            | "ENUM"
    )
}

fn is_clause_keyword(token: &str) -> bool {
    matches!(
        token,
        "SELECT"
            | "FROM"
            | "WHERE"
            | "GROUP"
            | "HAVING"
            | "ORDER"
            | "LIMIT"
            | "OFFSET"
            | "UNION"
            | "EXCEPT"
            | "INTERSECT"
            | "SET"
            | "VALUES"
    )
}

fn is_join_keyword(token: &str) -> bool {
    matches!(
        token,
        "JOIN" | "INNER" | "LEFT" | "RIGHT" | "CROSS" | "FULL" | "OUTER"
    )
}

fn is_join_context(k: &str) -> bool {
    k.starts_with("JOIN") || matches!(k, "INNER" | "LEFT" | "RIGHT" | "CROSS" | "FULL" | "ON")
}

fn ends_with_join(upper: &str) -> bool {
    matches!(
        upper,
        "JOIN" | "INNER" | "LEFT" | "RIGHT" | "CROSS" | "FULL"
    )
}

fn detect_create_table_column_context(tokens: &[String]) -> bool {
    if !tokens.contains(&"CREATE".to_string()) || !tokens.contains(&"TABLE".to_string()) {
        return false;
    }
    if let Some(table_idx) = tokens.iter().position(|t| t == "TABLE") {
        if table_idx + 2 < tokens.len() {
            let after = &tokens[table_idx + 2..];
            let last = after.last().unwrap();
            if last == "(" || last == "," {
                return true;
            }
            if is_data_type(last) {
                return false;
            }
            if *last == "NOT" {
                return false;
            }
        }
    }
    false
}

fn detect_create_table_type_context(tokens: &[String]) -> bool {
    if !tokens.contains(&"CREATE".to_string()) || !tokens.contains(&"TABLE".to_string()) {
        return false;
    }
    if let Some(table_idx) = tokens.iter().position(|t| t == "TABLE") {
        if table_idx + 3 < tokens.len() {
            let after = &tokens[table_idx + 2..];
            let last = after.last().unwrap();
            if is_data_type(last) {
                return true;
            }
        }
    }
    false
}

fn detect_constraint_context(tokens: &[String]) -> bool {
    let empty = String::new();
    let last = tokens.last().unwrap_or(&empty);
    matches!(
        last.as_str(),
        "PRIMARY" | "FOREIGN" | "UNIQUE" | "CHECK" | "DEFAULT" | "CONSTRAINT" | "REFERENCES"
    )
}

fn is_data_type(token: &str) -> bool {
    matches!(
        token,
        "INT"
            | "INTEGER"
            | "VARCHAR"
            | "TEXT"
            | "BOOLEAN"
            | "BOOL"
            | "DATE"
            | "TIMESTAMP"
            | "FLOAT"
            | "DOUBLE"
            | "DECIMAL"
            | "CHAR"
            | "BLOB"
            | "REAL"
            | "BIGINT"
            | "SMALLINT"
            | "TINYINT"
            | "SERIAL"
            | "BIGSERIAL"
            | "ENUM"
            | "NOT"
            | "NULL"
    )
}
