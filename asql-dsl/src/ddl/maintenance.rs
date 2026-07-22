use crate::dialect::Dialect;

/// The type of table maintenance operation
pub enum MaintenanceOp {
    /// `REPAIR TABLE`
    Repair,
    /// `OPTIMIZE TABLE`
    Optimize,
    /// `ANALYZE TABLE`
    Analyze,
    /// `CHECK TABLE`
    Check,
}

/// Builder for MySQL-style table maintenance SQL statements:
/// `REPAIR TABLE`, `OPTIMIZE TABLE`, `ANALYZE TABLE`, `CHECK TABLE`
pub struct TableMaintenanceBuilder {
    operation: MaintenanceOp,
    tables: Vec<String>,
}

impl TableMaintenanceBuilder {
    /// Creates a new builder for the specified maintenance operation
    pub fn new(operation: MaintenanceOp) -> Self {
        Self {
            operation,
            tables: Vec::new(),
        }
    }

    /// Adds a table name to the maintenance list
    pub fn add_table(mut self, table: &str) -> Self {
        self.tables.push(table.to_string());
        self
    }

    /// Builds the SQL statement for the configured dialect
    ///
    /// Returns the SQL string (e.g., `REPAIR TABLE \`tbl1\`, \`tbl2\``).
    /// For non-MySQL dialects the SQL is still generated; execution may fail
    /// at the database level.
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let op = match self.operation {
            MaintenanceOp::Repair => "REPAIR",
            MaintenanceOp::Optimize => "OPTIMIZE",
            MaintenanceOp::Analyze => "ANALYZE",
            MaintenanceOp::Check => "CHECK",
        };
        let quoted: Vec<String> = self
            .tables
            .iter()
            .map(|t| dialect.quote_ident(t))
            .collect();
        format!("{} TABLE {}", op, quoted.join(", "))
    }
}
