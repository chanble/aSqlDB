use crate::dialect::Dialect;

/// Builder for index introspection queries across supported dialects
pub struct IndexesIntrospection;

impl IndexesIntrospection {
    /// Generate a query to list all indexes for a given table
    pub fn list_indexes(dialect: &dyn Dialect, table: &str) -> String {
        match dialect.name() {
            "MySQL" => {
                format!("SHOW INDEX FROM {}", dialect.quote_ident(table))
            }
            "PostgreSQL" => {
                format!(
                    "SELECT indexname AS Key_name, tablename AS Table, indexdef \
                     FROM pg_indexes WHERE tablename = '{}'",
                    table.replace('\'', "''")
                )
            }
            "SQLite" => {
                format!("PRAGMA index_list(\"{}\")", table)
            }
            _ => unreachable!(),
        }
    }
}
