use crate::dialect::Dialect;

/// Builder for constructing INSERT INTO statements with columns and rows
pub struct InsertBuilder {
    table: Option<String>,
    columns: Vec<String>,
    values: Vec<Vec<String>>,
}

impl InsertBuilder {
    /// Create a new empty InsertBuilder
    pub fn new() -> Self {
        Self {
            table: None,
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Set the target table name for the INSERT statement
    pub fn into(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Add a column name to the INSERT column list
    pub fn column(mut self, name: &str) -> Self {
        self.columns.push(name.to_string());
        self
    }

    /// Add a row of values corresponding to the declared columns
    pub fn row(mut self, values: Vec<&str>) -> Self {
        self.values.push(values.into_iter().map(String::from).collect());
        self
    }

    /// Generate the INSERT SQL string using the given dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("INSERT requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let quoted_cols: Vec<String> = self
            .columns
            .iter()
            .map(|c| dialect.quote_ident(c))
            .collect();

        let value_rows: Vec<String> = self
            .values
            .iter()
            .map(|row| {
                let vals: Vec<String> = row
                    .iter()
                    .map(|v| {
                        if v == "NULL" || v == "NOW()" {
                            v.to_string()
                        } else {
                            dialect.quote_str(v)
                        }
                    })
                    .collect();
                format!("({})", vals.join(", "))
            })
            .collect();

        format!(
            "INSERT INTO {quoted_table} ({}) VALUES {}",
            quoted_cols.join(", "),
            value_rows.join(", ")
        )
    }

    /// Generate multiple INSERT statements for bulk export from pre-collected rows
    pub fn build_export(
        &self,
        dialect: &dyn Dialect,
        rows: &[Vec<String>],
    ) -> Vec<String> {
        let table = self
            .table
            .as_ref()
            .expect("INSERT requires a table name");
        let quoted_table = dialect.quote_ident(table);
        let quoted_cols: Vec<String> = self
            .columns
            .iter()
            .map(|c| dialect.quote_ident(c))
            .collect();
        let cols_part = quoted_cols.join(", ");

        rows.iter()
            .map(|row| {
                let vals: Vec<String> = row
                    .iter()
                    .map(|v| {
                        if v == "NULL" {
                            v.to_string()
                        } else {
                            dialect.quote_str(v)
                        }
                    })
                    .collect();
                format!(
                    "INSERT INTO {quoted_table} ({cols_part}) VALUES ({});",
                    vals.join(", ")
                )
            })
            .collect()
    }
}

impl Default for InsertBuilder {
    fn default() -> Self {
        Self::new()
    }
}
