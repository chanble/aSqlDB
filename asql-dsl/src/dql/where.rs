use crate::dialect::Dialect;

/// Logical operator connecting WHERE conditions: AND or OR
pub enum LogicalOp {
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

/// A WHERE expression: either a simple condition or a grouped sub-expression
pub enum WhereExpr {
    /// A simple column operator value condition
    Condition {
        column: String,
        operator: String,
        value: String,
    },
    /// A group of conditions wrapped in parentheses
    Group(WhereBuilder),
}

/// A single item in a WHERE clause, consisting of an optional logical operator and an expression
pub struct WhereItem {
    /// The logical operator connecting this item to the previous one (None for the first item)
    pub logical: Option<LogicalOp>,
    /// The WHERE expression (condition or grouped sub-expression)
    pub expr: WhereExpr,
}

/// WHERE 条件构建器，支持 AND/OR 组合和括号分组。
///
/// # Examples
///
/// 基本 AND 条件：
///
/// ```ignore
/// WhereBuilder::new()
///     .and("a", "=", "1")
///     .and("b", ">", "2")
/// // => a = '1' AND b > '2'
/// ```
///
/// AND + OR 混合：
///
/// ```ignore
/// WhereBuilder::new()
///     .and("a", "=", "1")
///     .and_group(|g|
///         g.or("b", "=", "2")
///          .or("c", "=", "3")
///     )
/// // => a = '1' AND (b = '2' OR c = '3')
/// ```
///
/// 多层括号分组：
///
/// ```ignore
/// WhereBuilder::new()
///     .or_group(|g|
///         g.and("a", "=", "1")
///          .and("b", "=", "2")
///     )
///     .or_group(|g|
///         g.and("c", "=", "3")
///          .and("d", "=", "4")
///     )
/// // => (a = '1' AND b = '2') OR (c = '3' AND d = '4')
/// ```
pub struct WhereBuilder {
    items: Vec<WhereItem>,
}

impl WhereBuilder {
    /// Creates a new empty WhereBuilder
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    /// Adds an AND condition (column operator value). The logical operator is omitted for the first condition
    pub fn and(mut self, column: &str, operator: &str, value: &str) -> Self {
        let logical = if self.items.is_empty() {
            None
        } else {
            Some(LogicalOp::And)
        };
        self.items.push(WhereItem {
            logical,
            expr: WhereExpr::Condition {
                column: column.to_string(),
                operator: operator.to_string(),
                value: value.to_string(),
            },
        });
        self
    }

    /// Adds an OR condition (column operator value). The logical operator is omitted for the first condition
    pub fn or(mut self, column: &str, operator: &str, value: &str) -> Self {
        let logical = if self.items.is_empty() {
            None
        } else {
            Some(LogicalOp::Or)
        };
        self.items.push(WhereItem {
            logical,
            expr: WhereExpr::Condition {
                column: column.to_string(),
                operator: operator.to_string(),
                value: value.to_string(),
            },
        });
        self
    }

    /// Adds a grouped AND condition using a closure. The group is wrapped in parentheses
    pub fn and_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        let group = build(WhereBuilder::new());
        if !group.is_empty() {
            let logical = if self.items.is_empty() {
                None
            } else {
                Some(LogicalOp::And)
            };
            self.items.push(WhereItem {
                logical,
                expr: WhereExpr::Group(group),
            });
        }
        self
    }

    /// Adds a grouped OR condition using a closure. The group is wrapped in parentheses
    pub fn or_group(mut self, build: impl FnOnce(WhereBuilder) -> WhereBuilder) -> Self {
        let group = build(WhereBuilder::new());
        if !group.is_empty() {
            let logical = if self.items.is_empty() {
                None
            } else {
                Some(LogicalOp::Or)
            };
            self.items.push(WhereItem {
                logical,
                expr: WhereExpr::Group(group),
            });
        }
        self
    }

    /// Returns true if no conditions have been added
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Builds the WHERE clause SQL string for the given dialect. Returns an empty string if no conditions exist
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        if self.items.is_empty() {
            return String::new();
        }

        let mut sql = String::new();
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                match &item.logical {
                    Some(LogicalOp::And) => sql.push_str(" AND "),
                    Some(LogicalOp::Or) => sql.push_str(" OR "),
                    None => sql.push(' '),
                }
            }
            sql.push_str(&render_expr(&item.expr, dialect));
        }
        sql
    }
}

impl Default for WhereBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn render_expr(expr: &WhereExpr, dialect: &dyn Dialect) -> String {
    match expr {
        WhereExpr::Condition {
            column,
            operator,
            value,
        } => {
            let col = dialect.quote_ident(column);
            if value == "NULL" {
                format!("{col} IS NULL")
            } else {
                match operator.to_uppercase().as_str() {
                    "LIKE" | "NOT LIKE" => {
                        format!("{col} {operator} '%{value}%'")
                    }
                    "IN" | "NOT IN" => {
                        let vals: Vec<String> = value.split(',')
                            .map(|v| dialect.quote_str(v.trim()))
                            .collect();
                        format!("{col} {operator} ({})", vals.join(", "))
                    }
                    _ => {
                        let val = dialect.quote_str(value);
                        format!("{col} {operator} {val}")
                    }
                }
            }
        }
        WhereExpr::Group(builder) => {
            format!("({})", builder.build(dialect))
        }
    }
}
