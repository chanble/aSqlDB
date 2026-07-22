use crate::dialect::Dialect;

/// The type of object to drop
pub enum DropTarget {
    /// Drop a table
    Table,
    /// Drop an index
    Index,
    /// Drop a database
    Database,
}

/// Builder for constructing `DROP` or `TRUNCATE` statements
pub struct DropBuilder {
    target: DropTarget,
    name: Option<String>,
    table: Option<String>,
    if_exists: bool,
}

impl DropBuilder {
    /// Creates a new builder for dropping the specified target type
    pub fn new(target: DropTarget) -> Self {
        Self {
            target,
            name: None,
            table: None,
            if_exists: false,
        }
    }

    /// Sets the name of the object to drop
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Sets the table name (required when dropping an index)
    pub fn on(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Adds `IF EXISTS` to the statement
    pub fn if_exists(mut self) -> Self {
        self.if_exists = true;
        self
    }

    /// Builds the `DROP` SQL statement for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let name = self.name.as_deref().expect("DROP requires a name");
        let quoted_name = dialect.quote_ident(name);

        let if_exists = if self.if_exists { " IF EXISTS" } else { "" };

        match self.target {
            DropTarget::Table => {
                format!("DROP TABLE{if_exists} {quoted_name}")
            }
            DropTarget::Index => {
                let table = self
                    .table
                    .as_ref()
                    .expect("DROP INDEX requires a table name");
                let quoted_table = dialect.quote_ident(table);
                if dialect.name() == "MySQL" {
                    format!("DROP INDEX {quoted_name} ON {quoted_table}")
                } else {
                    format!("DROP INDEX{if_exists} {quoted_name}")
                }
            }
            DropTarget::Database => {
                format!("DROP DATABASE{if_exists} {quoted_name}")
            }
        }
    }

    /// Builds a `TRUNCATE TABLE` SQL statement
    pub fn build_truncate(&self, dialect: &dyn Dialect) -> String {
        let name = self
            .name
            .as_deref()
            .expect("TRUNCATE requires a table name");
        let quoted_name = dialect.quote_ident(name);
        match dialect.name() {
            "SQLite" => format!("DELETE FROM {quoted_name}"),
            _ => format!("TRUNCATE TABLE {quoted_name}"),
        }
    }
}
