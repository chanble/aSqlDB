use super::Dialect;

/// MySQL dialect implementation using backtick quoting and standard MySQL SQL syntax
pub struct MySql;

impl Dialect for MySql {
    fn name(&self) -> &'static str {
        "MySQL"
    }

    fn quote_ident(&self, ident: &str) -> String {
        format!("`{}`", ident.replace('`', "``"))
    }

    fn auto_increment(&self) -> &'static str {
        "AUTO_INCREMENT"
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
        "NOW()"
    }
}
