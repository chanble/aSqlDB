use crate::dialect::Dialect;

/// The type of database index
#[derive(Clone)]
pub enum IndexType {
    /// A primary key index
    Primary,
    /// A unique constraint index
    Unique,
    /// A regular non-unique index
    Index,
    /// A full-text search index
    Fulltext,
    /// A spatial (R-tree) index
    Spatial,
}

/// The algorithm or data structure used by an index
#[derive(Clone)]
pub enum IndexMethod {
    /// B-tree index method
    BTree,
    /// Hash index method
    Hash,
    /// R-tree index method (used for spatial data)
    RTree,
}

impl IndexMethod {
    /// Returns the SQL string representation of the index method
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexMethod::BTree => "BTREE",
            IndexMethod::Hash => "HASH",
            IndexMethod::RTree => "RTREE",
        }
    }
}

/// A column reference within an index definition, with optional prefix length
#[derive(Clone)]
pub struct IndexColumn {
    /// Name of the column
    pub name: String,
    /// Optional prefix length for indexed column (e.g. `name(10)`)
    pub prefix_len: Option<usize>,
}

/// Builder for constructing `CREATE INDEX` or `DROP INDEX` statements
pub struct IndexBuilder {
    table: Option<String>,
    name: Option<String>,
    index_type: IndexType,
    columns: Vec<IndexColumn>,
    method: Option<IndexMethod>,
}

impl IndexBuilder {
    /// Creates a new empty `IndexBuilder`
    pub fn new() -> Self {
        Self {
            table: None,
            name: None,
            index_type: IndexType::Index,
            columns: Vec::new(),
            method: None,
        }
    }

    /// Sets the table on which the index is defined
    pub fn on(mut self, table: &str) -> Self {
        self.table = Some(table.to_string());
        self
    }

    /// Sets the name of the index
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Sets the type of the index
    pub fn index_type(mut self, t: IndexType) -> Self {
        self.index_type = t;
        self
    }

    /// Adds a column to the index with an optional prefix length
    pub fn column(mut self, name: &str, prefix_len: Option<usize>) -> Self {
        self.columns.push(IndexColumn {
            name: name.to_string(),
            prefix_len,
        });
        self
    }

    /// Sets the index method (BTREE, HASH, RTREE)
    pub fn using(mut self, method: IndexMethod) -> Self {
        self.method = Some(method);
        self
    }

    fn using_clause(&self) -> String {
        match &self.method {
            Some(m) => format!(" USING {}", m.as_str()),
            None => String::new(),
        }
    }

    /// Builds the `CREATE INDEX` SQL statement for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("INDEX requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let col_list: Vec<String> = self
            .columns
            .iter()
            .map(|c| {
                let col = dialect.quote_ident(&c.name);
                match c.prefix_len {
                    Some(len) => format!("{col}({len})"),
                    None => col,
                }
            })
            .collect();
        let cols = col_list.join(", ");
        let using = self.using_clause();

        match self.index_type {
            IndexType::Primary => {
                format!("ALTER TABLE {quoted_table} ADD PRIMARY KEY ({cols})")
            }
            IndexType::Unique => {
                let idx_name = self.name.as_deref().unwrap_or("idx_unique");
                let quoted_name = dialect.quote_ident(idx_name);
                format!("CREATE UNIQUE INDEX {quoted_name} ON {quoted_table} ({cols}){using}")
            }
            IndexType::Index => {
                let idx_name = self.name.as_deref().unwrap_or("idx");
                let quoted_name = dialect.quote_ident(idx_name);
                format!("CREATE INDEX {quoted_name} ON {quoted_table} ({cols}){using}")
            }
            IndexType::Fulltext => {
                let idx_name = self.name.as_deref().unwrap_or("idx_ft");
                let quoted_name = dialect.quote_ident(idx_name);
                format!("CREATE FULLTEXT INDEX {quoted_name} ON {quoted_table} ({cols}){using}")
            }
            IndexType::Spatial => {
                let idx_name = self.name.as_deref().unwrap_or("idx_sp");
                let quoted_name = dialect.quote_ident(idx_name);
                format!("CREATE SPATIAL INDEX {quoted_name} ON {quoted_table} ({cols}){using}")
            }
        }
    }

    /// Builds the `DROP INDEX` SQL statement for the configured dialect
    pub fn build_drop(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("DROP INDEX requires a table name");
        let quoted_table = dialect.quote_ident(table);

        match self.index_type {
            IndexType::Primary => {
                format!("ALTER TABLE {quoted_table} DROP PRIMARY KEY")
            }
            _ => {
                let idx_name = self.name.as_deref().unwrap_or("idx");
                let quoted_name = dialect.quote_ident(idx_name);
                if dialect.name() == "MySQL" {
                    format!("DROP INDEX {quoted_name} ON {quoted_table}")
                } else {
                    format!("DROP INDEX {quoted_name}")
                }
            }
        }
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}
