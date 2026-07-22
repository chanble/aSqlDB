mod mysql;
mod postgres;
mod sqlite;

/// MySQL dialect implementation
pub use mysql::MySql;
/// PostgreSQL dialect implementation
pub use postgres::PostgreSql;
/// SQLite dialect implementation
pub use sqlite::Sqlite;

/// A database dialect that defines SQL syntax variations across database engines
pub trait Dialect {
    /// Returns the name of the dialect
    fn name(&self) -> &'static str;

    /// Quotes an identifier according to dialect-specific rules
    fn quote_ident(&self, ident: &str) -> String;

    /// Quotes a string literal according to dialect-specific rules
    fn quote_str(&self, s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    /// Returns the SQL keyword for auto-increment column definition
    fn auto_increment(&self) -> &'static str;

    /// Returns the LIMIT clause for a single limit value
    fn limit(&self, limit: usize) -> String;

    /// Returns the combined LIMIT OFFSET clause
    fn limit_offset(&self, limit: usize, offset: usize) -> String;

    /// Whether the dialect supports DELETE statements with LIMIT
    fn supports_delete_limit(&self) -> bool;

    /// Whether the dialect supports UPDATE statements with LIMIT
    fn supports_update_limit(&self) -> bool;

    /// Returns the SQL expression for the current timestamp
    fn current_timestamp(&self) -> &'static str;
}
