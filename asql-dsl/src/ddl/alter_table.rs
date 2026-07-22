use crate::ddl::{ColumnDef, IndexColumn, IndexMethod, IndexType};
use crate::dialect::Dialect;

/// Position of a new column in an `ALTER TABLE ADD COLUMN` statement
pub enum ColumnPosition {
    /// Place the column last (default)
    Last,
    /// Place the column first
    First,
    /// Place the column after the specified column
    After(String),
}

/// Represents a single action in an `ALTER TABLE` statement
pub enum AlterAction {
    /// Adds a new column to the table
    AddColumn(ColumnDef, ColumnPosition),
    /// Modifies the definition of an existing column
    ModifyColumn(ColumnDef, ColumnPosition),
    /// Renames an existing column to a new definition
    ChangeColumn {
        old_name: String,
        new_def: ColumnDef,
        position: ColumnPosition,
    },
    /// Removes a column from the table
    DropColumn(String),
    /// Renames the entire table
    RenameTable(String),
    /// Adds a primary key constraint on the given columns
    AddPrimaryKey(Vec<IndexColumn>),
    /// Removes the primary key constraint
    DropPrimaryKey,
    /// Adds a new index to the table
    AddIndex {
        name: String,
        index_type: IndexType,
        columns: Vec<IndexColumn>,
        method: Option<IndexMethod>,
    },
    /// Removes an index from the table
    DropIndex(String),
    /// Sets the table comment
    Comment(String),
    /// Sets the storage engine
    Engine(String),
    /// Sets the default collation
    Collation(String),
}

/// Builder for constructing `ALTER TABLE` statements with multiple actions
pub struct AlterTableBuilder {
    table: Option<String>,
    actions: Vec<AlterAction>,
}

impl AlterTableBuilder {
    /// Creates a new empty `AlterTableBuilder`
    pub fn new() -> Self {
        Self {
            table: None,
            actions: Vec::new(),
        }
    }

    /// Sets the name of the table to alter
    pub fn table(mut self, name: &str) -> Self {
        self.table = Some(name.to_string());
        self
    }

    /// Adds an action to add a new column (appended at the end)
    pub fn add_column(mut self, col: ColumnDef) -> Self {
        self.actions.push(AlterAction::AddColumn(col, ColumnPosition::Last));
        self
    }

    /// Adds an action to add a new column at the beginning
    pub fn add_column_first(mut self, col: ColumnDef) -> Self {
        self.actions.push(AlterAction::AddColumn(col, ColumnPosition::First));
        self
    }

    /// Adds an action to add a new column after an existing column
    pub fn add_column_after(mut self, col: ColumnDef, after: &str) -> Self {
        self.actions.push(AlterAction::AddColumn(col, ColumnPosition::After(after.to_string())));
        self
    }

    /// Adds an action to modify an existing column's definition
    pub fn modify_column(mut self, col: ColumnDef) -> Self {
        self.actions.push(AlterAction::ModifyColumn(col, ColumnPosition::Last));
        self
    }

    /// Adds an action to modify an existing column and place it first
    pub fn modify_column_first(mut self, col: ColumnDef) -> Self {
        self.actions.push(AlterAction::ModifyColumn(col, ColumnPosition::First));
        self
    }

    /// Adds an action to modify an existing column and place it after another column
    pub fn modify_column_after(mut self, col: ColumnDef, after: &str) -> Self {
        self.actions.push(AlterAction::ModifyColumn(col, ColumnPosition::After(after.to_string())));
        self
    }

    /// Adds an action to rename a column and optionally change its definition
    pub fn change_column(mut self, old_name: &str, new_def: ColumnDef) -> Self {
        self.actions
            .push(AlterAction::ChangeColumn {
                old_name: old_name.to_string(),
                new_def,
                position: ColumnPosition::Last,
            });
        self
    }

    /// Adds an action to rename a column and place it first
    pub fn change_column_first(mut self, old_name: &str, new_def: ColumnDef) -> Self {
        self.actions
            .push(AlterAction::ChangeColumn {
                old_name: old_name.to_string(),
                new_def,
                position: ColumnPosition::First,
            });
        self
    }

    /// Adds an action to rename a column and place it after another column
    pub fn change_column_after(mut self, old_name: &str, new_def: ColumnDef, after: &str) -> Self {
        self.actions
            .push(AlterAction::ChangeColumn {
                old_name: old_name.to_string(),
                new_def,
                position: ColumnPosition::After(after.to_string()),
            });
        self
    }

    /// Adds an action to drop a column by name
    pub fn drop_column(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::DropColumn(name.to_string()));
        self
    }

    /// Adds an action to rename the table
    pub fn rename_table(mut self, new_name: &str) -> Self {
        self.actions
            .push(AlterAction::RenameTable(new_name.to_string()));
        self
    }

    /// Adds an action to add a primary key on the given columns
    pub fn add_primary_key(mut self, columns: Vec<(&str, Option<usize>)>) -> Self {
        self.actions.push(AlterAction::AddPrimaryKey(
            columns
                .into_iter()
                .map(|(name, len)| IndexColumn {
                    name: name.to_string(),
                    prefix_len: len,
                })
                .collect(),
        ));
        self
    }

    /// Adds an action to drop the primary key
    pub fn drop_primary_key(mut self) -> Self {
        self.actions.push(AlterAction::DropPrimaryKey);
        self
    }

    /// Adds an action to add a new index with the given name, type, and columns
    pub fn add_index(
        mut self,
        name: &str,
        index_type: IndexType,
        columns: Vec<(&str, Option<usize>)>,
    ) -> Self {
        self.actions.push(AlterAction::AddIndex {
            name: name.to_string(),
            index_type,
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

    /// Sets the index method (BTREE, HASH, RTREE) for the most recently added index action
    pub fn using(mut self, method: IndexMethod) -> Self {
        if let Some(AlterAction::AddIndex {
            method: ref mut m, ..
        }) = self.actions.last_mut()
        {
            *m = Some(method);
        }
        self
    }

    /// Sets the table comment
    pub fn comment(mut self, comment: &str) -> Self {
        self.actions
            .push(AlterAction::Comment(comment.to_string()));
        self
    }

    /// Sets the storage engine
    pub fn engine(mut self, engine: &str) -> Self {
        self.actions
            .push(AlterAction::Engine(engine.to_string()));
        self
    }

    /// Sets the default collation
    pub fn collation(mut self, collation: &str) -> Self {
        self.actions
            .push(AlterAction::Collation(collation.to_string()));
        self
    }

    /// Adds an action to drop an index by name
    pub fn drop_index(mut self, name: &str) -> Self {
        self.actions
            .push(AlterAction::DropIndex(name.to_string()));
        self
    }

    /// Builds the `ALTER TABLE` SQL statement(s) for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> Vec<String> {
        let table = self
            .table
            .as_ref()
            .expect("ALTER TABLE requires a table name");
        let quoted_table = dialect.quote_ident(table);

        let mut stmts = Vec::new();
        let mut table_options: Vec<String> = Vec::new();

        for action in &self.actions {
            match action {
                AlterAction::AddColumn(col, pos) => {
                    let mut sql = format!(
                        "ALTER TABLE {quoted_table} ADD COLUMN {}",
                        build_column_sql(col, dialect)
                    );
                    append_position(&mut sql, pos, dialect);
                    stmts.push(sql);
                }
                AlterAction::ModifyColumn(col, pos) => {
                    let col_sql = build_column_sql(col, dialect);
                    if dialect.name() == "MySQL" {
                        let mut sql = format!("ALTER TABLE {quoted_table} MODIFY COLUMN {col_sql}");
                        append_position(&mut sql, pos, dialect);
                        stmts.push(sql);
                    } else {
                        stmts.push(format!(
                            "ALTER TABLE {quoted_table} ALTER COLUMN {} TYPE {}",
                            dialect.quote_ident(&col.name),
                            col.col_type.to_sql(),
                        ));
                    }
                }
                AlterAction::ChangeColumn { old_name, new_def, position } => {
                    let col_sql = build_column_sql(new_def, dialect);
                    let mut sql = format!(
                        "ALTER TABLE {quoted_table} CHANGE COLUMN {} {col_sql}",
                        dialect.quote_ident(old_name)
                    );
                    append_position(&mut sql, position, dialect);
                    stmts.push(sql);
                }
                AlterAction::DropColumn(name) => {
                    stmts.push(format!(
                        "ALTER TABLE {quoted_table} DROP COLUMN {}",
                        dialect.quote_ident(name)
                    ));
                }
                AlterAction::RenameTable(new_name) => {
                    stmts.push(format!(
                        "RENAME TABLE {quoted_table} TO {}",
                        dialect.quote_ident(new_name)
                    ));
                }
                AlterAction::AddPrimaryKey(columns) => {
                    let col_list = index_column_list(columns, dialect);
                    stmts.push(format!(
                        "ALTER TABLE {quoted_table} ADD PRIMARY KEY ({col_list})"
                    ));
                }
                AlterAction::DropPrimaryKey => {
                    stmts.push(format!(
                        "ALTER TABLE {quoted_table} DROP PRIMARY KEY"
                    ));
                }
                AlterAction::AddIndex {
                    name,
                    index_type,
                    columns,
                    method,
                } => {
                    let col_list = index_column_list(columns, dialect);
                    let quoted_name = dialect.quote_ident(name);
                    let using = match method {
                        Some(m) => format!(" USING {}", m.as_str()),
                        None => String::new(),
                    };
                    match index_type {
                        IndexType::Primary => {
                            stmts.push(format!(
                                "ALTER TABLE {quoted_table} ADD PRIMARY KEY ({col_list})"
                            ));
                        }
                        IndexType::Unique => {
                            if dialect.name() == "MySQL" {
                                stmts.push(format!(
                                    "ALTER TABLE {quoted_table} ADD UNIQUE INDEX {quoted_name} ({col_list}){using}"
                                ));
                            } else {
                                stmts.push(format!(
                                    "CREATE UNIQUE INDEX {quoted_name} ON {quoted_table} ({col_list}){using}"
                                ));
                            }
                        }
                        IndexType::Index => {
                            if dialect.name() == "MySQL" {
                                stmts.push(format!(
                                    "ALTER TABLE {quoted_table} ADD INDEX {quoted_name} ({col_list}){using}"
                                ));
                            } else {
                                stmts.push(format!(
                                    "CREATE INDEX {quoted_name} ON {quoted_table} ({col_list}){using}"
                                ));
                            }
                        }
                        IndexType::Fulltext => {
                            if dialect.name() == "MySQL" {
                                stmts.push(format!(
                                    "ALTER TABLE {quoted_table} ADD FULLTEXT INDEX {quoted_name} ({col_list}){using}"
                                ));
                            } else {
                                stmts.push(format!(
                                    "CREATE FULLTEXT INDEX {quoted_name} ON {quoted_table} ({col_list}){using}"
                                ));
                            }
                        }
                        IndexType::Spatial => {
                            if dialect.name() == "MySQL" {
                                stmts.push(format!(
                                    "ALTER TABLE {quoted_table} ADD SPATIAL INDEX {quoted_name} ({col_list}){using}"
                                ));
                            } else {
                                stmts.push(format!(
                                    "CREATE SPATIAL INDEX {quoted_name} ON {quoted_table} ({col_list}){using}"
                                ));
                            }
                        }
                    }
                }
                AlterAction::DropIndex(name) => {
                    let quoted_name = dialect.quote_ident(name);
                    if dialect.name() == "MySQL" {
                        stmts.push(format!(
                            "ALTER TABLE {quoted_table} DROP INDEX {quoted_name}"
                        ));
                    } else {
                        stmts.push(format!("DROP INDEX {quoted_name}"));
                    }
                }
                AlterAction::Comment(comment) => {
                    table_options.push(format!(
                        "COMMENT = {}", dialect.quote_str(comment)
                    ));
                }
                AlterAction::Engine(engine) => {
                    table_options.push(format!("ENGINE = {engine}"));
                }
                AlterAction::Collation(collation) => {
                    table_options.push(format!("COLLATE = {collation}"));
                }
            }
        }

        if !table_options.is_empty() {
            stmts.push(format!(
                "ALTER TABLE {quoted_table} {}",
                table_options.join(", ")
            ));
        }

        stmts
    }
}

impl Default for AlterTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn index_column_list(columns: &[IndexColumn], dialect: &dyn Dialect) -> String {
    columns
        .iter()
        .map(|c| {
            let col = dialect.quote_ident(&c.name);
            match c.prefix_len {
                Some(len) => format!("{col}({len})"),
                None => col,
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_column_sql(col: &ColumnDef, dialect: &dyn Dialect) -> String {
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

    sql
}

fn append_position(sql: &mut String, pos: &ColumnPosition, dialect: &dyn Dialect) {
    if dialect.name() != "MySQL" {
        return;
    }
    match pos {
        ColumnPosition::First => sql.push_str(" FIRST"),
        ColumnPosition::After(after) => {
            sql.push_str(&format!(" AFTER {}", dialect.quote_ident(after)));
        }
        ColumnPosition::Last => {}
    }
}
