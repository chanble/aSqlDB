use crate::ddl::{IndexColumn, IndexMethod, IndexType};
use crate::dialect::Dialect;
use asql_types::ColumnExtra;
use asql_types::ColumnType;
use serde::ser::SerializeStruct;
use serde::Serialize;

/// Unified column definition used for DDL construction and query result metadata.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub nullable: Option<bool>,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub extra: ColumnExtra,
    pub collation: Option<String>,
    pub key: Option<String>,
}

impl Serialize for ColumnDef {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("ColumnDef", 9)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("col_type", &self.col_type)?;
        state.serialize_field("data_type", &self.col_type.to_sql())?;
        state.serialize_field("nullable", &self.nullable)?;
        state.serialize_field("default", &self.default_value)?;
        state.serialize_field("comment", &self.comment)?;
        state.serialize_field("extra", &self.extra)?;
        state.serialize_field("collation", &self.collation)?;
        state.serialize_field("key", &self.key)?;
        state.end()
    }
}

impl Default for ColumnDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            col_type: asql_types::ColumnType::Int(asql_types::IntType {
                display_width: None, unsigned: false, zerofill: false,
            }),
            nullable: None,
            default_value: None,
            comment: None,
            extra: ColumnExtra::default(),
            collation: None,
            key: None,
        }
    }
}

/// Describes a table index (primary, unique, fulltext, or spatial)
#[derive(Clone)]
pub struct TableIndex {
    /// The type of index
    pub index_type: IndexType,
    /// Optional index name
    pub name: Option<String>,
    /// The columns that make up the index
    pub columns: Vec<IndexColumn>,
    /// Optional index method (BTREE, HASH, RTREE)
    pub method: Option<IndexMethod>,
}

/// Builder for constructing `CREATE TABLE` statements
pub struct CreateTableBuilder {
    table: Option<String>,
    columns: Vec<ColumnDef>,
    indexes: Vec<TableIndex>,
    engine: Option<String>,
    collation: Option<String>,
    comment: Option<String>,
}

impl CreateTableBuilder {
    /// Creates a new empty `CreateTableBuilder`
    pub fn new() -> Self {
        Self {
            table: None,
            columns: Vec::new(),
            indexes: Vec::new(),
            engine: None,
            collation: None,
            comment: None,
        }
    }

    /// Sets the name of the table
    pub fn table(mut self, name: &str) -> Self {
        self.table = Some(name.to_string());
        self
    }

    /// Adds a column definition to the table
    pub fn column(mut self, col: ColumnDef) -> Self {
        if col.extra.auto_increment {
            let has_pk = self
                .indexes
                .iter()
                .any(|i| matches!(i.index_type, IndexType::Primary));
            if !has_pk {
                self.indexes.push(TableIndex {
                    index_type: IndexType::Primary,
                    name: None,
                    columns: vec![IndexColumn {
                        name: col.name.clone(),
                        prefix_len: None,
                    }],
                    method: None,
                });
            }
        }
        self.columns.push(col);
        self
    }

    /// Adds a primary key constraint on the given columns (replaces any existing primary key)
    pub fn primary_key(mut self, columns: Vec<&str>) -> Self {
        self.indexes.retain(|i| !matches!(i.index_type, IndexType::Primary));
        self.indexes.push(TableIndex {
            index_type: IndexType::Primary,
            name: None,
            columns: columns
                .into_iter()
                .map(|c| IndexColumn {
                    name: c.to_string(),
                    prefix_len: None,
                })
                .collect(),
            method: None,
        });
        self
    }

    /// Adds a unique constraint index with the given name and columns
    pub fn unique(mut self, name: &str, columns: Vec<(&str, Option<usize>)>) -> Self {
        self.indexes.push(TableIndex {
            index_type: IndexType::Unique,
            name: Some(name.to_string()),
            columns: columns
                .into_iter()
                .map(|(n, len)| IndexColumn {
                    name: n.to_string(),
                    prefix_len: len,
                })
                .collect(),
            method: None,
        });
        self
    }

    /// Adds a regular index with the given name and columns
    pub fn index(mut self, name: &str, columns: Vec<(&str, Option<usize>)>) -> Self {
        self.indexes.push(TableIndex {
            index_type: IndexType::Index,
            name: Some(name.to_string()),
            columns: columns
                .into_iter()
                .map(|(n, len)| IndexColumn {
                    name: n.to_string(),
                    prefix_len: len,
                })
                .collect(),
            method: None,
        });
        self
    }

    /// Adds a full-text search index with the given name and columns
    pub fn fulltext(mut self, name: &str, columns: Vec<(&str, Option<usize>)>) -> Self {
        self.indexes.push(TableIndex {
            index_type: IndexType::Fulltext,
            name: Some(name.to_string()),
            columns: columns
                .into_iter()
                .map(|(n, len)| IndexColumn {
                    name: n.to_string(),
                    prefix_len: len,
                })
                .collect(),
            method: None,
        });
        self
    }

    /// Adds a spatial index with the given name and columns
    pub fn spatial(mut self, name: &str, columns: Vec<(&str, Option<usize>)>) -> Self {
        self.indexes.push(TableIndex {
            index_type: IndexType::Spatial,
            name: Some(name.to_string()),
            columns: columns
                .into_iter()
                .map(|(n, len)| IndexColumn {
                    name: n.to_string(),
                    prefix_len: len,
                })
                .collect(),
            method: None,
        });
        self
    }

    /// Sets the storage engine for the table (e.g. `InnoDB`)
    pub fn engine(mut self, engine: &str) -> Self {
        self.engine = Some(engine.to_string());
        self
    }

    /// Sets the default collation for the table
    pub fn collation(mut self, collation: &str) -> Self {
        self.collation = Some(collation.to_string());
        self
    }

    /// Sets the table comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    /// Sets the index method (BTREE, HASH, RTREE) for the most recently added index
    pub fn using(mut self, method: IndexMethod) -> Self {
        if let Some(last) = self.indexes.last_mut() {
            last.method = Some(method);
        }
        self
    }

    /// Builds the `CREATE TABLE` SQL statement for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let table = self
            .table
            .as_ref()
            .expect("CREATE TABLE requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let col_defs: Vec<String> = self
            .columns
            .iter()
            .map(|c| self.build_column_def(c, dialect))
            .collect();

        let mut all_parts = col_defs;

            for idx in &self.indexes {
                let col_list: Vec<String> = idx
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
                let using = match &idx.method {
                    Some(m) => format!(" USING {}", m.as_str()),
                    None => String::new(),
                };

                match idx.index_type {
                    IndexType::Primary => {
                        all_parts.push(format!("PRIMARY KEY ({cols})"));
                    }
                    IndexType::Unique => {
                        let name = idx.name.as_deref().unwrap_or("uq");
                        if dialect.name() == "MySQL" {
                            all_parts.push(format!(
                                "UNIQUE KEY {} ({cols}){using}",
                                dialect.quote_ident(name)
                            ));
                        } else {
                            all_parts.push(format!(
                                "CONSTRAINT {} UNIQUE ({cols})",
                                dialect.quote_ident(name)
                            ));
                        }
                    }
                    IndexType::Index => {
                        if dialect.name() == "MySQL" {
                            let name = idx.name.as_deref().unwrap_or("idx");
                            all_parts.push(format!(
                                "INDEX {} ({cols}){using}",
                                dialect.quote_ident(name)
                            ));
                        }
                    }
                    IndexType::Fulltext => {
                        if dialect.name() == "MySQL" {
                            let name = idx.name.as_deref().unwrap_or("idx_ft");
                            all_parts.push(format!(
                                "FULLTEXT INDEX {} ({cols})",
                                dialect.quote_ident(name)
                            ));
                        }
                    }
                    IndexType::Spatial => {
                        if dialect.name() == "MySQL" {
                            let name = idx.name.as_deref().unwrap_or("idx_sp");
                            all_parts.push(format!(
                                "SPATIAL INDEX {} ({cols}){using}",
                                dialect.quote_ident(name)
                            ));
                        }
                    }
                }
            }

        let mut sql = format!(
            "CREATE TABLE {quoted_table} (\n  {}\n)",
            all_parts.join(",\n  ")
        );

        if let Some(engine) = &self.engine {
            sql.push_str(&format!(" ENGINE={engine}"));
        }

        if let Some(collation) = &self.collation {
            sql.push_str(&format!(" COLLATE={collation}"));
        }

        if let Some(comment) = &self.comment {
            sql.push_str(&format!(" COMMENT={}", dialect.quote_str(comment)));
        }

        sql
    }

    fn build_column_def(&self, col: &ColumnDef, dialect: &dyn Dialect) -> String {
        let mut sql =
            format!("{} {}", dialect.quote_ident(&col.name), col.col_type.to_sql());

        if col.nullable == Some(false) {
            sql.push_str(" NOT NULL");
        }

        if col.extra.auto_increment {
            sql.push(' ');
            sql.push_str(dialect.auto_increment());
        }

        if let Some(ref default) = col.default_value {
            sql.push_str(&format!(" DEFAULT {default}"));
        }

        if let Some(ref comment) = col.comment {
            if dialect.name() == "MySQL" {
                sql.push_str(&format!(" COMMENT {}", dialect.quote_str(comment)));
            }
        }

        if col.extra.on_update && dialect.name() == "MySQL" {
            sql.push_str(" ON UPDATE CURRENT_TIMESTAMP");
        }

        sql
    }
}

impl Default for CreateTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}
