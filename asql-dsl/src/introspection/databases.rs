use crate::dialect::Dialect;

/// Builder for database/schema introspection queries
pub struct DatabasesIntrospection;

impl DatabasesIntrospection {
    /// Generate a query to list all databases or schemas
    pub fn list_databases(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => {
                "SELECT SCHEMA_NAME AS name, DEFAULT_COLLATION_NAME AS collation \
                 FROM information_schema.SCHEMATA ORDER BY SCHEMA_NAME"
                    .to_string()
            }
            "PostgreSQL" => {
                "SELECT datname AS name, '' AS collation \
                 FROM pg_database WHERE datistemplate = false ORDER BY datname"
                    .to_string()
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }

    /// Generate a USE statement to switch the active database
    pub fn use_database(dialect: &dyn Dialect, db: &str) -> String {
        format!("USE {}", dialect.quote_ident(db))
    }

    /// Generate a query to get the current database/schema name.
    ///
    /// The result column is always named `db` so callers can read
    /// results with a fixed key regardless of the underlying engine.
    ///
    /// - MySQL:    `SELECT DATABASE() as db`
    /// - PostgreSQL: `SELECT current_database() as db`
    /// - SQLite:  `SELECT '' as db`
    pub fn current_database(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => "SELECT DATABASE() as db".to_string(),
            "PostgreSQL" => "SELECT current_database() as db".to_string(),
            "SQLite" => "SELECT '' as db".to_string(),
            _ => unreachable!(),
        }
    }
}
