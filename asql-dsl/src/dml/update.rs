use crate::dialect::Dialect;
use crate::dql::WhereBuilder;

/// A single column-value assignment in an UPDATE SET clause
pub struct UpdateSet {
    /// Name of the column to update
    pub column: String,
    /// New value to assign to the column
    pub value: String,
}

/// Builder for constructing UPDATE statements with SET and optional WHERE clauses
pub struct UpdateBuilder {
    table: Option<String>,
    sets: Vec<UpdateSet>,
    where_builder: WhereBuilder,
    limit: Option<usize>,
}

impl UpdateBuilder {
    /// Create a new empty UpdateBuilder
    pub fn new() -> Self {
        Self {
            table: None,
            sets: Vec::new(),
            where_builder: WhereBuilder::new(),
            limit: None,
        }
    }

    /// Set the target table name for the UPDATE statement
    pub fn table(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Add a SET assignment (column = value) to the UPDATE statement
    pub fn set(mut self, column: &str, value: &str) -> Self {
        self.sets.push(UpdateSet {
            column: column.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Add a WHERE condition (initial or chained AND)
    pub fn where_(mut self, column: &str, op: &str, value: &str) -> Self {
        self.where_builder = self.where_builder.and(column, op, value);
        self
    }

    /// Chain an additional AND condition to the WHERE clause
    pub fn and_where(mut self, column: &str, op: &str, value: &str) -> Self {
        self.where_builder = self.where_builder.and(column, op, value);
        self
    }

    /// Chain an OR condition to the WHERE clause
    pub fn or_where(mut self, column: &str, op: &str, value: &str) -> Self {
        self.where_builder = self.where_builder.or(column, op, value);
        self
    }

    /// Add a grouped AND condition using a sub-builder closure
    pub fn and_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        self.where_builder = self.where_builder.and_group(build);
        self
    }

    /// Add a grouped OR condition using a sub-builder closure
    pub fn or_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        self.where_builder = self.where_builder.or_group(build);
        self
    }

    /// Set a LIMIT on the number of rows updated (dialect-dependent support)
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Generate the UPDATE SQL string using the given dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("UPDATE requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let set_parts: Vec<String> = self
            .sets
            .iter()
            .map(|s| {
                let col = dialect.quote_ident(&s.column);
                let val = dialect.quote_str(&s.value);
                format!("{col} = {val}")
            })
            .collect();

        let mut sql = format!(
            "UPDATE {quoted_table} SET {}",
            set_parts.join(", ")
        );

        let where_clause = self.where_builder.build(dialect);
        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause);
        }

        if let Some(n) = self.limit {
            if dialect.supports_update_limit() {
                sql.push(' ');
                sql.push_str(&dialect.limit(n));
            }
        }

        sql
    }
}

impl Default for UpdateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
