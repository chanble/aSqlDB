use crate::dialect::Dialect;
use crate::dql::WhereBuilder;

/// A column reference in a SELECT clause, optionally wrapped in a function
pub struct SelectColumn {
    /// Optional aggregate or scalar function (e.g. "COUNT", "MAX")
    pub func: Option<String>,
    /// The column name or expression
    pub name: String,
}

/// An ORDER BY clause specifying a column and sort direction
pub struct OrderBy {
    /// The column name to sort by
    pub column: String,
    /// Whether to sort in descending order
    pub desc: bool,
}

impl OrderBy {
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let col = dialect.quote_ident(&self.column);
        if self.desc {
            format!("{col} DESC")
        } else {
            col
        }
    }
}

/// A builder for constructing SELECT queries with optional WHERE, ORDER BY, LIMIT, and OFFSET
pub struct SelectBuilder {
    columns: Vec<SelectColumn>,
    table: Option<String>,
    where_builder: WhereBuilder,
    order_by: Vec<OrderBy>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl SelectBuilder {
    /// Creates a new empty SelectBuilder
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            table: None,
            where_builder: WhereBuilder::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Adds a column to the SELECT clause
    pub fn column(mut self, name: &str) -> Self {
        self.columns.push(SelectColumn {
            func: None,
            name: name.to_string(),
        });
        self
    }

    /// Adds a column wrapped in a function (e.g. COUNT(col), MAX(col))
    pub fn column_with_func(mut self, func: &str, name: &str) -> Self {
        self.columns.push(SelectColumn {
            func: Some(func.to_string()),
            name: name.to_string(),
        });
        self
    }

    /// Sets the FROM table for the query
    pub fn from(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Replaces the WHERE clause with a pre-built WhereBuilder
    pub fn where_(mut self, wb: WhereBuilder) -> Self {
        self.where_builder = wb;
        self
    }

    /// Adds an AND condition to the WHERE clause
    pub fn and_where(mut self, column: &str, op: &str, value: &str) -> Self {
        self.where_builder = self.where_builder.and(column, op, value);
        self
    }

    /// Adds an OR condition to the WHERE clause
    pub fn or_where(mut self, column: &str, op: &str, value: &str) -> Self {
        self.where_builder = self.where_builder.or(column, op, value);
        self
    }

    /// Adds a grouped AND condition using a closure that receives a WhereBuilder
    pub fn and_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        self.where_builder = self.where_builder.and_group(build);
        self
    }

    /// Adds a grouped OR condition using a closure that receives a WhereBuilder
    pub fn or_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        self.where_builder = self.where_builder.or_group(build);
        self
    }

    /// Adds an ORDER BY clause for the given column with optional descending sort
    pub fn order_by(mut self, column: &str, desc: bool) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            desc,
        });
        self
    }

    /// Sets a LIMIT on the number of rows returned
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Sets an OFFSET for the number of rows to skip
    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    /// Builds the complete SELECT SQL string for the given dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let mut sql = String::from("SELECT ");

        if self.columns.is_empty() {
            sql.push('*');
        } else {
            let parts: Vec<String> = self
                .columns
                .iter()
                .map(|c| match &c.func {
                    Some(f) => format!("{}({})", f, dialect.quote_ident(&c.name)),
                    None => dialect.quote_ident(&c.name),
                })
                .collect();
            sql.push_str(&parts.join(", "));
        }

        if let Some(table) = &self.table {
            sql.push_str(" FROM ");
            sql.push_str(&dialect.quote_ident(table));
        }

        let where_clause = self.where_builder.build(dialect);
        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause);
        }

        if !self.order_by.is_empty() {
            let parts: Vec<String> = self.order_by.iter().map(|o| o.build(dialect)).collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&parts.join(", "));
        }

        if let Some(n) = self.limit {
            if let Some(off) = self.offset {
                sql.push(' ');
                sql.push_str(&dialect.limit_offset(n, off));
            } else {
                sql.push(' ');
                sql.push_str(&dialect.limit(n));
            }
        }

        sql
    }

    /// Returns the table name if set
    pub fn table(&self) -> Option<&str> {
        self.table.as_deref()
    }

    /// Returns the list of explicitly selected columns
    pub fn columns(&self) -> &[SelectColumn] {
        &self.columns
    }

    /// Builds a SELECT COUNT(*) query retaining the same FROM and WHERE clauses
    pub fn build_count(&self, dialect: &dyn Dialect) -> String {
        let mut sql = String::from("SELECT COUNT(*) as cnt");

        if let Some(table) = &self.table {
            sql.push_str(" FROM ");
            sql.push_str(&dialect.quote_ident(table));
        }

        let where_clause = self.where_builder.build(dialect);
        if !where_clause.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause);
        }

        sql
    }
}

impl Default for SelectBuilder {
    fn default() -> Self {
        Self::new()
    }
}
