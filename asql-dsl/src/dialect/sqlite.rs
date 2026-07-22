use super::Dialect;

/// SQLite dialect implementation using double-quote quoting and SQLite SQL syntax
pub struct Sqlite;

impl Dialect for Sqlite {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }

    fn auto_increment(&self) -> &'static str {
        "AUTOINCREMENT"
    }

    fn limit(&self, limit: usize) -> String {
        format!("LIMIT {limit}")
    }

    fn limit_offset(&self, limit: usize, offset: usize) -> String {
        format!("LIMIT {limit} OFFSET {offset}")
    }

    fn supports_delete_limit(&self) -> bool {
        true
    }

    fn supports_update_limit(&self) -> bool {
        true
    }

    fn current_timestamp(&self) -> &'static str {
        "CURRENT_TIMESTAMP"
    }
}
