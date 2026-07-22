use crate::dialect::Dialect;
use crate::dql::WhereBuilder;

/// Builder for constructing DELETE FROM statements with optional WHERE and LIMIT
pub struct DeleteBuilder {
    table: Option<String>,
    where_builder: WhereBuilder,
    limit: Option<usize>,
}

impl DeleteBuilder {
    /// Create a new empty DeleteBuilder
    pub fn new() -> Self {
        Self {
            table: None,
            where_builder: WhereBuilder::new(),
            limit: None,
        }
    }

    /// Set the target table name for the DELETE FROM statement
    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
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

    /// Set a LIMIT on the number of rows deleted (dialect-dependent support)
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Generate the DELETE SQL string using the given dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("DELETE requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let mut sql = format!("DELETE FROM {quoted_table}");

        let where_clause = self.where_builder.build(dialect);
        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause);
        }

        if let Some(n) = self.limit {
            if dialect.supports_delete_limit() {
                sql.push(' ');
                sql.push_str(&dialect.limit(n));
            }
        }

        sql
    }
}

impl Default for DeleteBuilder {
    fn default() -> Self {
        Self::new()
    }
}
