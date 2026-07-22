use std::sync::Arc;

use asql_sql::completer::get_suggestions;
use asql_sql::db_type::DatabaseType;
use asql_sql::schema::{Column, DatabaseSchema, Table};
use asql_sql::suggestion::SuggestionKind;
use asql_sql::tokenizer::{tokenize_sql, TokenKind};

fn setup_schema() -> DatabaseSchema {
    let mut schema = DatabaseSchema::new();
    schema.add_table(Table {
        name: "users".into(),
        columns: vec![
            Column {
                name: "id".into(),
                data_type: "INTEGER".into(),
                comment: "".into(),
            },
            Column {
                name: "name".into(),
                data_type: "VARCHAR".into(),
                comment: "".into(),
            },
            Column {
                name: "email".into(),
                data_type: "VARCHAR".into(),
                comment: "".into(),
            },
        ],
        comment: "".into(),
    });
    schema.add_table(Table {
        name: "orders".into(),
        columns: vec![
            Column {
                name: "id".into(),
                data_type: "INTEGER".into(),
                comment: "".into(),
            },
            Column {
                name: "user_id".into(),
                data_type: "INTEGER".into(),
                comment: "".into(),
            },
            Column {
                name: "amount".into(),
                data_type: "DECIMAL".into(),
                comment: "".into(),
            },
        ],
        comment: "".into(),
    });
    schema.add_table(Table {
        name: "products".into(),
        columns: vec![
            Column {
                name: "id".into(),
                data_type: "INTEGER".into(),
                comment: "".into(),
            },
            Column {
                name: "title".into(),
                data_type: "VARCHAR".into(),
                comment: "".into(),
            },
            Column {
                name: "price".into(),
                data_type: "DECIMAL".into(),
                comment: "".into(),
            },
        ],
        comment: "".into(),
    });
    schema
}

// ===================== Completion tests =====================

#[tokio::test]
async fn test_start_suggests_keywords() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "", 0).await;
    assert!(suggestions.iter().any(|s| s.text == "SELECT"));
    assert!(suggestions.iter().any(|s| s.text == "INSERT"));
}

#[tokio::test]
async fn test_after_from_suggests_tables() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "SELECT * FROM ", 14).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "users" && s.kind == SuggestionKind::Table));
    assert!(suggestions
        .iter()
        .any(|s| s.text == "orders" && s.kind == SuggestionKind::Table));
}

#[tokio::test]
async fn test_after_from_with_prefix() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "SELECT * FROM us", 15).await;
    assert!(suggestions.iter().any(|s| s.text == "users"));
    assert!(!suggestions.iter().any(|s| s.text == "orders"));
}

#[tokio::test]
async fn test_after_select_suggests_columns() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "SELECT  FROM users", 7).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
    assert!(suggestions
        .iter()
        .any(|s| s.text == "name" && s.kind == SuggestionKind::Column));
}

#[tokio::test]
async fn test_after_select_with_prefix_matches_columns() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "SELECT i FROM users", 8).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
    assert!(!suggestions.iter().any(|s| s.text == "name"));
}

#[tokio::test]
async fn test_after_where_suggests_columns() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(
        DatabaseType::MySql,
        schema,
        "SELECT * FROM users WHERE ",
        26,
    )
    .await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
}

#[tokio::test]
async fn test_suggests_functions() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::Postgres, schema, "SELECT CO", 9).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "COUNT" && s.kind == SuggestionKind::Function));
}

#[tokio::test]
async fn test_after_dot_suggests_columns() {
    let schema = Arc::new(setup_schema());
    let suggestions = get_suggestions(DatabaseType::MySql, schema, "SELECT users.", 13).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
    assert!(suggestions
        .iter()
        .any(|s| s.text == "name" && s.kind == SuggestionKind::Column));
    assert!(!suggestions.iter().any(|s| s.text == "amount"));
}

#[tokio::test]
async fn test_db_type_keywords() {
    let mysql_kw = DatabaseType::MySql.keywords();
    assert!(mysql_kw.contains(&"SHOW"));
    assert!(mysql_kw.contains(&"SELECT"));
    let pg_kw = DatabaseType::Postgres.keywords();
    assert!(pg_kw.contains(&"RETURNING"));
    let sqlite_kw = DatabaseType::Sqlite.keywords();
    assert!(sqlite_kw.contains(&"PRAGMA"));
}

#[tokio::test]
async fn test_alias_as_dot_completion() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT u. FROM users AS u";
    let cursor = sql.find(". ").unwrap() + 2;
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, cursor).await;
    assert!(suggestions.iter().any(|s| s.text == "id"));
    assert!(suggestions.iter().any(|s| s.text == "name"));
    assert!(!suggestions.iter().any(|s| s.text == "amount"));
}

#[tokio::test]
async fn test_alias_implicit_dot_completion() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT u. FROM users u";
    let cursor = sql.find(". ").unwrap() + 2;
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, cursor).await;
    assert!(suggestions.iter().any(|s| s.text == "id"));
    assert!(!suggestions.iter().any(|s| s.text == "amount"));
}

#[tokio::test]
async fn test_alias_with_join_dot_completion() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT o. FROM users u JOIN orders o ON u.id = o.user_id";
    let cursor = sql.find(". ").unwrap() + 2;
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, cursor).await;
    assert!(suggestions.iter().any(|s| s.text == "amount"));
    assert!(!suggestions.iter().any(|s| s.text == "email"));
}

#[tokio::test]
async fn test_alias_in_where_context() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT * FROM users u WHERE ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
}

#[tokio::test]
async fn test_subquery_select_context() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT * FROM users WHERE id IN (SELECT ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "id" && s.kind == SuggestionKind::Column));
}

#[tokio::test]
async fn test_subquery_from_context() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT * FROM (SELECT * FROM ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "users" && s.kind == SuggestionKind::Table));
}

#[tokio::test]
async fn test_subquery_where_context() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT * FROM users WHERE id IN (SELECT id FROM orders WHERE ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "amount" && s.kind == SuggestionKind::Column));
}

#[tokio::test]
async fn test_nested_subquery() {
    let schema = Arc::new(setup_schema());
    let sql = "SELECT * FROM users WHERE id IN (SELECT )";
    let cursor = sql.len() - 1;
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, cursor).await;
    assert!(suggestions.iter().any(|s| s.text == "id"));
}

#[tokio::test]
async fn test_create_table_suggests_types() {
    let schema = Arc::new(setup_schema());
    let sql = "CREATE TABLE foo (id ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "INTEGER"));
    assert!(suggestions.iter().any(|s| s.text == "VARCHAR"));
}

#[tokio::test]
async fn test_create_table_suggests_types_postgres() {
    let schema = Arc::new(setup_schema());
    let sql = "CREATE TABLE foo (id ";
    let suggestions = get_suggestions(DatabaseType::Postgres, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "SERIAL"));
    assert!(suggestions.iter().any(|s| s.text == "UUID"));
}

#[tokio::test]
async fn test_create_table_suggests_constraints() {
    let schema = Arc::new(setup_schema());
    let sql = "CREATE TABLE foo (id INTEGER ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "PRIMARY"));
    assert!(suggestions.iter().any(|s| s.text == "NOT"));
    assert!(suggestions.iter().any(|s| s.text == "UNIQUE"));
}

#[tokio::test]
async fn test_alter_table_add() {
    let schema = Arc::new(setup_schema());
    let sql = "ALTER TABLE users ADD ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "VARCHAR" || s.text == "INTEGER"));
}

#[tokio::test]
async fn test_alter_table_suggests_tables() {
    let schema = Arc::new(setup_schema());
    let sql = "ALTER TABLE ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions
        .iter()
        .any(|s| s.text == "users" && s.kind == SuggestionKind::Table));
}

#[tokio::test]
async fn test_create_table_if_not_exists() {
    let schema = Arc::new(setup_schema());
    let sql = "CREATE TABLE IF NOT EXISTS ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "users"));
}

#[tokio::test]
async fn test_drop_table_suggests_tables() {
    let schema = Arc::new(setup_schema());
    let sql = "DROP TABLE ";
    let suggestions = get_suggestions(DatabaseType::MySql, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "users"));
}

#[tokio::test]
async fn test_column_types_sqlite() {
    let schema = Arc::new(setup_schema());
    let sql = "CREATE TABLE foo (bar ";
    let suggestions = get_suggestions(DatabaseType::Sqlite, schema, sql, sql.len()).await;
    assert!(suggestions.iter().any(|s| s.text == "INTEGER"));
    assert!(suggestions.iter().any(|s| s.text == "TEXT"));
}

// ===================== Tokenizer tests =====================

fn span_texts(sql: &str) -> Vec<(&str, TokenKind)> {
    tokenize_sql(sql, DatabaseType::MySql)
        .iter()
        .filter(|s| s.kind != TokenKind::Whitespace)
        .map(|s| (&sql[s.start..s.end], s.kind.clone()))
        .collect()
}

#[test]
fn test_tokenize_keywords() {
    let tokens = span_texts("SELECT * FROM users");
    assert_eq!(tokens[0], ("SELECT", TokenKind::Keyword));
    assert_eq!(tokens[1], ("*", TokenKind::Operator));
    assert_eq!(tokens[2], ("FROM", TokenKind::Keyword));
    assert_eq!(tokens[3], ("users", TokenKind::Identifier));
}

#[test]
fn test_tokenize_functions() {
    let tokens = span_texts("SELECT COUNT(id), MAX(amount) FROM orders");
    assert_eq!(tokens[1], ("COUNT", TokenKind::Function));
    assert_eq!(tokens[6], ("MAX", TokenKind::Function));
}

#[test]
fn test_tokenize_string_single_quote() {
    let tokens = span_texts("SELECT 'hello world'");
    assert_eq!(tokens[1], ("'hello world'", TokenKind::String));
}

#[test]
fn test_tokenize_string_escaped_quote() {
    let tokens = span_texts("SELECT 'it''s ok'");
    assert_eq!(tokens[1], ("'it''s ok'", TokenKind::String));
}

#[test]
fn test_tokenize_string_double_quote() {
    let tokens = span_texts("\"column name\" FROM t");
    assert_eq!(tokens[0], ("\"column name\"", TokenKind::String));
}

#[test]
fn test_tokenize_backtick_identifier() {
    let tokens = span_texts("SELECT `name` FROM `users`");
    assert_eq!(tokens[1], ("`name`", TokenKind::Identifier));
    assert_eq!(tokens[3], ("`users`", TokenKind::Identifier));
}

#[test]
fn test_tokenize_numbers() {
    let tokens = span_texts("SELECT 42, 3.14, 0.5");
    assert_eq!(tokens[1], ("42", TokenKind::Number));
    assert_eq!(tokens[3], ("3.14", TokenKind::Number));
    assert_eq!(tokens[5], ("0.5", TokenKind::Number));
}

#[test]
fn test_tokenize_operators() {
    let tokens = span_texts("a = 1 AND b <> 2 AND c != 3");
    assert_eq!(tokens[1], ("=", TokenKind::Operator));
    assert_eq!(tokens[5], ("<>", TokenKind::Operator));
    assert_eq!(tokens[9], ("!=", TokenKind::Operator));
}

#[test]
fn test_tokenize_comparison_operators() {
    let tokens = span_texts("a >= 1 AND b <= 2 AND c > 3 AND d < 4");
    assert_eq!(tokens[1], (">=", TokenKind::Operator));
    assert_eq!(tokens[5], ("<=", TokenKind::Operator));
    assert_eq!(tokens[9], (">", TokenKind::Operator));
    assert_eq!(tokens[13], ("<", TokenKind::Operator));
}

#[test]
fn test_tokenize_punctuation() {
    let tokens = span_texts("SELECT a, b FROM t;");
    assert_eq!(tokens[2], (",", TokenKind::Punctuation));
    assert_eq!(tokens[6], (";", TokenKind::Punctuation));
}

#[test]
fn test_tokenize_line_comment() {
    let tokens = span_texts("SELECT -- this is a comment\nid");
    assert_eq!(tokens[0], ("SELECT", TokenKind::Keyword));
    assert_eq!(tokens[1], ("-- this is a comment", TokenKind::Comment));
    assert_eq!(tokens[2], ("id", TokenKind::Identifier));
}

#[test]
fn test_tokenize_block_comment() {
    let tokens = span_texts("SELECT /* comment */ id");
    assert_eq!(tokens[0], ("SELECT", TokenKind::Keyword));
    assert_eq!(tokens[1], ("/* comment */", TokenKind::Comment));
    assert_eq!(tokens[2], ("id", TokenKind::Identifier));
}

#[test]
fn test_tokenize_hash_comment() {
    let tokens = span_texts("# comment\nSELECT");
    assert_eq!(tokens[0], ("# comment", TokenKind::Comment));
    assert_eq!(tokens[1], ("SELECT", TokenKind::Keyword));
}

#[test]
fn test_tokenize_variable() {
    let tokens = span_texts("SELECT @row_num");
    assert_eq!(tokens[1], ("@row_num", TokenKind::Variable));
}

#[test]
fn test_tokenize_parentheses() {
    let tokens = span_texts("COUNT(*);");
    assert_eq!(tokens[0], ("COUNT", TokenKind::Function));
    assert_eq!(tokens[1], ("(", TokenKind::Punctuation));
    assert_eq!(tokens[2], ("*", TokenKind::Operator));
    assert_eq!(tokens[3], (")", TokenKind::Punctuation));
    assert_eq!(tokens[4], (";", TokenKind::Punctuation));
}

#[test]
fn test_tokenize_dot() {
    let tokens = span_texts("u.id");
    assert_eq!(tokens[0], ("u", TokenKind::Identifier));
    assert_eq!(tokens[1], (".", TokenKind::Punctuation));
    assert_eq!(tokens[2], ("id", TokenKind::Identifier));
}

#[test]
fn test_tokenize_pipe_operator() {
    let tokens = span_texts("'a' || 'b'");
    assert_eq!(tokens[1], ("||", TokenKind::Operator));
}

#[test]
fn test_tokenize_complex_query() {
    let sql = "SELECT u.name, COUNT(o.id) AS total FROM users u JOIN orders o ON u.id = o.user_id WHERE u.name = 'Alice' GROUP BY u.name ORDER BY total DESC LIMIT 10";
    let tokens = tokenize_sql(&sql, DatabaseType::MySql);
    assert!(tokens.len() > 20);

    let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::Keyword));
    assert!(kinds.contains(&&TokenKind::Function));
    assert!(kinds.contains(&&TokenKind::Identifier));
    assert!(kinds.contains(&&TokenKind::Operator));
    assert!(kinds.contains(&&TokenKind::String));
    assert!(kinds.contains(&&TokenKind::Number));
    assert!(kinds.contains(&&TokenKind::Punctuation));
}

#[test]
fn test_tokenize_case_insensitive_keywords() {
    let tokens = span_texts("select from where");
    assert_eq!(tokens[0], ("select", TokenKind::Keyword));
    assert_eq!(tokens[1], ("from", TokenKind::Keyword));
    assert_eq!(tokens[2], ("where", TokenKind::Keyword));
}

#[test]
fn test_tokenize_mixed_case_keywords() {
    let tokens = span_texts("Select From users Where");
    assert_eq!(tokens[0], ("Select", TokenKind::Keyword));
    assert_eq!(tokens[1], ("From", TokenKind::Keyword));
    assert_eq!(tokens[3], ("Where", TokenKind::Keyword));
}

#[test]
fn test_tokenize_whitespace() {
    let tokens = tokenize_sql("SELECT \t id", DatabaseType::MySql);
    assert_eq!(tokens[1].kind, TokenKind::Whitespace);
    assert_eq!(&"SELECT \t id"[tokens[1].start..tokens[1].end], " \t ");
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
}
