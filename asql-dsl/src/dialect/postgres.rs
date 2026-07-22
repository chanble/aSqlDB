use super::Dialect;

/// PostgreSQL dialect implementation using double-quote quoting and PostgreSQL SQL syntax
pub struct PostgreSql;

impl Dialect for PostgreSql {
    fn name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }

    fn auto_increment(&self) -> &'static str {
        "GENERATED ALWAYS AS IDENTITY"
    }

    fn limit(&self, limit: usize) -> String {
        format!("LIMIT {limit}")
    }

    fn limit_offset(&self, limit: usize, offset: usize) -> String {
        format!("LIMIT {limit} OFFSET {offset}")
    }

    fn supports_delete_limit(&self) -> bool {
        false
    }

    fn supports_update_limit(&self) -> bool {
        false
    }

    fn current_timestamp(&self) -> &'static str {
        "CURRENT_TIMESTAMP"
    }
}
