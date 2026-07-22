use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

// ─── Re-exports: asql-dsl ────────────────────────────────────────────
pub use asql_dsl::ddl::{
    AlterTableBuilder, ColumnDef, CreateTableBuilder, DatabaseBuilder, DropBuilder, DropTarget,
    GrantBuilder, GrantRole, IndexBuilder, IndexColumn, IndexMethod, IndexType, MaintenanceOp,
    TableMaintenanceBuilder, UserBuilder,
};
pub use asql_dsl::dialect::{Dialect, MySql, PostgreSql, Sqlite};
pub use asql_dsl::dml::{DeleteBuilder, InsertBuilder, UpdateBuilder};
pub use asql_dsl::dql::{SelectBuilder, SelectColumn};
pub use asql_dsl::introspection::{
    ColumnsIntrospection, DatabasesIntrospection, IndexesIntrospection, ServerIntrospection,
    TableNameMatch, TablesIntrospection, UsersIntrospection,
};
pub use asql_dsl::{OrderBy, WhereBuilder};
pub use asql_types::{ColumnExtra, ColumnType, EnumType, FloatType, IntType, StringType};

// ─── Re-exports: asql-core ───────────────────────────────────────────
pub use asql_core::db_manager::{DatabaseType, DbManager, Pool};
pub use asql_core::export::{
    ColumnTarget, DataFormat, DatabaseOption, ExportConfig, ExportReceiver, TableDef, TableOption,
    TableTarget,
};
pub use asql_core::persistence::{ConnectionConfig, ConnectionParams};
pub use asql_core::result::{
    DbError, DbRow, DbSuccessResult, ExecutionResult, ModifyResult, SchemaResult, SetResult,
};

// ─── Re-exports: preset types from asql-types ────────────────────────
pub use asql_types::{
    Charset, DataType, DataTypeCategory, DbFunction, DbTypeInfo, Engine, FunctionCategory,
    Privilege, PrivilegeScope, SqlMode, DATABASE_TYPES,
};

// ─── Re-exports: asql-sql (SQL completion) ───────────────────────────
pub use asql_sql::{
    get_suggestions, Column as SchemaColumn, DatabaseSchema, SchemaProvider, Suggestion,
    SuggestionKind, Table as SchemaTable,
};

pub mod domain;
pub mod schema_provider;
pub use domain::*;
pub use schema_provider::DbSchemaProvider;

// ─── Protocol types ──────────────────────────────────────────────────

/// The result of a SELECT query, with full column metadata.
#[derive(Debug, Serialize)]
pub struct SelectResult {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<DbRow>,
}

// ─── QueryBuilder ────────────────────────────────────────────────────

/// Unified query protocol layer.
///
/// Each method is a typed protocol entry: it accepts structured parameters,
/// builds the SQL internally via `asql-dsl`, executes it via `asql-core`,
/// and returns a structured result.
///
/// Any frontend (Web, TUI, CLI, WASM, etc.) can use this as the single
/// contract for database operations.
pub struct QueryBuilder {
    dialect: Box<dyn Dialect + Send + Sync>,
    db_manager: Arc<DbManager>,
}

pub struct ExportBuilder {
    pub database: Option<String>,
    pub db_option: DatabaseOption,
    pub tables: ExportTable,
    pub table_option: TableOption,
    pub data_format: DataFormat,
}

pub enum ExportTable {
    /// 库中所有表
    All,
    /// 指定的表列表
    Selected(Vec<ExportTableDef>),
}

pub struct ExportTableDef {
    pub name: String,
    /// �?
    pub columns: ColumnTarget,
    /// WHERE �?SQL 片段（不�?WHERE 关键字）
    pub filter_sql: WhereBuilder,
    /// ORDER BY �?SQL 片段（不�?ORDER BY 关键字）
    pub order_by: Vec<OrderBy>,
    /// SQL格式: 是否输出 CREATE TABLE；非SQL格式: 是否输出表头
    pub ddl: bool,
    /// 是否导出数据
    pub data: bool,
}

impl QueryBuilder {
    /// Create a new `QueryBuilder` with an explicit dialect.
    pub fn new<D: Dialect + Send + Sync + 'static>(dialect: D, db_manager: Arc<DbManager>) -> Self {
        Self {
            dialect: Box::new(dialect),
            db_manager,
        }
    }

    /// Create a `QueryBuilder` from an already-boxed dialect.
    pub fn new_boxed(dialect: Box<dyn Dialect + Send + Sync>, db_manager: Arc<DbManager>) -> Self {
        Self {
            dialect,
            db_manager,
        }
    }

    /// Create a `QueryBuilder` by auto-detecting the dialect from a connection's URL.
    pub async fn from_connection(
        db_manager: &Arc<DbManager>,
        conn_name: &str,
    ) -> Result<Self, DbError> {
        let url = db_manager
            .get_connection_url(conn_name)
            .await
            .ok_or_else(|| {
                DbError::ConnectionError(format!("Connection '{}' not found", conn_name))
            })?;
        let db_type = DatabaseType::from_url(&url);
        let dialect: Box<dyn Dialect + Send + Sync> = match db_type {
            DatabaseType::MySql => Box::new(MySql),
            DatabaseType::Postgres => Box::new(PostgreSql),
            DatabaseType::Sqlite => Box::new(Sqlite),
        };
        Ok(Self {
            dialect,
            db_manager: db_manager.clone(),
        })
    }

    /// Access the dialect used by this builder.
    pub fn dialect(&self) -> &dyn Dialect {
        &*self.dialect
    }

    /// Access the underlying `DbManager`.
    pub fn db_manager(&self) -> &Arc<DbManager> {
        &self.db_manager
    }

    /// Create a fully self-contained `QueryBuilder` from a database URL.
    ///
    /// Internally creates a `DbManager`, adds the connection under the
    /// given name (defaults to `"default"`), and auto-detects the dialect.
    pub async fn new_with_url(url: &str, name: Option<&str>) -> Result<Self, DbError> {
        let config = ConnectionConfig::from_url(name.unwrap_or("default").to_string(), url);
        let dialect: Box<dyn Dialect + Send + Sync> = match config.params.db_type() {
            DatabaseType::MySql => Box::new(MySql),
            DatabaseType::Postgres => Box::new(PostgreSql),
            DatabaseType::Sqlite => Box::new(Sqlite),
        };
        let dm = Arc::new(DbManager::new());
        dm.add_connection(config).await?;
        Ok(Self {
            dialect,
            db_manager: dm,
        })
    }

    // ─── Connection management ──────────────────────────────────────

    /// Add a connection configuration.
    pub async fn add_connection(&self, config: ConnectionConfig) -> Result<(), DbError> {
        self.db_manager.add_connection(config).await
    }

    /// Remove a connection by name.
    pub async fn remove_connection(&self, name: &str) -> bool {
        self.db_manager.remove_connection(name).await
    }

    /// List all connections with their name, URL, and database type.
    pub async fn list_connections(&self) -> Vec<(String, String, DatabaseType)> {
        self.db_manager
            .list_connections()
            .await
            .into_iter()
            .map(|(name, item)| {
                (
                    name,
                    item.config.params.to_url(),
                    item.config.params.db_type(),
                )
            })
            .collect()
    }

    /// Test whether a connection is reachable by closing and re-opening it.
    pub async fn test_connection(&self, name: &str) -> Result<(), DbError> {
        self.db_manager.close_connection(name).await;
        self.db_manager.open_connection(name).await
    }

    /// Get the URL for a named connection.
    pub async fn get_connection_url(&self, name: &str) -> Option<String> {
        self.db_manager.get_connection_url(name).await
    }

    /// Save all current connections to a JSON file.
    pub async fn save_connections(&self, path: &Path) -> Result<(), DbError> {
        let configs: Vec<ConnectionConfig> = self
            .db_manager
            .list_connections()
            .await
            .into_iter()
            .map(|(_, item)| item.config)
            .collect();
        self.db_manager.save_configs(&configs, path).await
    }

    /// Load connections from a JSON file and register any new ones.
    pub async fn load_connections(&self, path: &Path) -> Result<Vec<ConnectionConfig>, DbError> {
        let configs = self.db_manager.load_configs(path).await?;
        for config in &configs {
            if self
                .db_manager
                .get_connection_url(&config.name)
                .await
                .is_none()
            {
                self.db_manager.add_connection(config.clone()).await?;
            }
        }
        Ok(configs)
    }

    // ─── Internal helpers ───────────────────────────────────────────

    /// Build `ColumnDef` from a raw `show_columns` row.
    fn column_def_from_row(raw: &DbRow) -> ColumnDef {
        let name = raw
            .get("Field")
            .or_else(|| raw.get("column_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let data_type = raw
            .get("Type")
            .or_else(|| raw.get("data_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let nullable = raw
            .get("Null")
            .and_then(|v| v.as_str())
            .map(|s| s == "YES")
            .or_else(|| {
                raw.get("is_nullable")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "YES")
            });

        let default = raw
            .get("Default")
            .or_else(|| raw.get("column_default"))
            .and_then(|v| {
                if v.is_null() {
                    return None;
                }
                let s = v.as_str()?;
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });

        let comment = raw
            .get("Comment")
            .or_else(|| raw.get("comment"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let collation = raw
            .get("Collation")
            .or_else(|| raw.get("collation_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let extra = raw
            .get("Extra")
            .or_else(|| raw.get("extra"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let key = raw
            .get("Key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                raw.get("is_primary_key")
                    .and_then(|v| v.as_i64())
                    .map(|pk| {
                        if pk > 0 {
                            "PRI".to_string()
                        } else {
                            String::new()
                        }
                    })
            })
            .or_else(|| {
                raw.get("pk").and_then(|v| v.as_i64()).map(|pk| {
                    if pk > 0 {
                        "PRI".to_string()
                    } else {
                        String::new()
                    }
                })
            })
            .filter(|s| !s.is_empty());

        ColumnDef {
            name,
            col_type: asql_types::parse_column_type(&data_type),
            nullable,
            default_value: default,
            comment,
            extra: asql_types::ColumnExtra {
                auto_increment: extra
                    .as_ref()
                    .map_or(false, |s| s.contains("auto_increment"))
                    || raw.get("is_identity").and_then(|v| v.as_str()) == Some("YES"),
                on_update: extra.as_ref().map_or(false, |s| s.contains("on update")),
            },
            collation,
            key,
        }
    }

    // ══════════════════════════════════════════════════════════════════
    //  DQL
    // ══════════════════════════════════════════════════════════════════

    /// Execute a `SELECT` query built from a `SelectBuilder`.
    pub fn select(
        &self,
        conn: &str,
        builder: &SelectBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<SelectResult>, DbError>> + Send + '_>>
    {
        let sql = builder.build(&*self.dialect);
        let show_sql = match builder.table() {
            Some(table) => Some(ColumnsIntrospection::show_columns(
                &*self.dialect,
                table,
                None,
            )),
            None => None,
        };
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        let table = builder.table().map(|s| s.to_string());
        let col_names: Vec<String> = builder.columns().iter().map(|c| c.name.clone()).collect();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let result = exec_result.data;
            let meta_rows = if let Some(ref _table_name) = table {
                if let Some(ref sql_cols) = show_sql {
                    match DbManager::execute_sql_send(dm.clone(), &conn, sql_cols).await {
                        Ok(DbSuccessResult::Select(r)) => r.data.rows,
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            let meta_map: HashMap<String, DbRow> = meta_rows
                .iter()
                .filter_map(|row| {
                    row.get("Field")
                        .or_else(|| row.get("column_name"))
                        .and_then(|v| v.as_str())
                        .map(|name| (name.to_string(), row.clone()))
                })
                .collect();
            let result_col_names: Vec<String> = result
                .rows
                .first()
                .map(|row| row.keys().cloned().collect())
                .filter(|names: &Vec<String>| !names.is_empty())
                .unwrap_or(col_names);
            let columns = if !result_col_names.is_empty() {
                result_col_names
                    .iter()
                    .map(|name| match meta_map.get(name) {
                        Some(raw) => Self::column_def_from_row(raw),
                        None => ColumnDef {
                            name: name.clone(),
                            col_type: asql_types::ColumnType::Varchar(asql_types::StringType {
                                length: None,
                            }),
                            nullable: None,
                            default_value: None,
                            comment: None,
                            extra: asql_types::ColumnExtra::default(),
                            collation: None,
                            key: None,
                        },
                    })
                    .collect()
            } else {
                meta_rows
                    .iter()
                    .map(|raw| Self::column_def_from_row(raw))
                    .collect()
            };
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: SelectResult {
                    columns,
                    rows: result.rows,
                },
            })
        })
    }

    /// Execute a `SELECT COUNT(*)` built from the same `SelectBuilder` context.
    pub fn select_count(
        &self,
        conn: &str,
        builder: &SelectBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<u64>, DbError>> + Send + '_>> {
        let sql = builder.build_count(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let count = exec_result
                .data
                .rows
                .first()
                .and_then(|row| row.values().next())
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u64;
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: count,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Export
    // ══════════════════════════════════════════════════════════════════

    /// Stream export data in the given format.
    pub async fn export_select(
        &self,
        conn: &str,
        export_conf: ExportBuilder,
    ) -> Result<ExportReceiver, DbError> {
        use asql_core::export::{ExportConfig, TableDef, TableTarget};
        let url = self
            .get_connection_url(conn)
            .await
            .ok_or_else(|| DbError::ConnectionError(format!("Connection '{}' not found", conn)))?;
        let db_type = DatabaseType::from_url(&url);
        let pool = Arc::new(Pool::connect(&url, db_type).await?);

        let export_config = ExportConfig {
            database: export_conf.database,
            db_option: export_conf.db_option,
            tables: match export_conf.tables {
                ExportTable::All => TableTarget::All,
                ExportTable::Selected(tables) => TableTarget::Selected(
                    tables
                        .into_iter()
                        .map(|t| TableDef {
                            name: t.name,
                            columns: t.columns,
                            filter_sql: t.filter_sql.build(&*self.dialect),
                            order_by: t
                                .order_by
                                .into_iter()
                                .map(|o| o.build(&*self.dialect))
                                .collect::<Vec<_>>()
                                .join(", "),
                            ddl: t.ddl,
                            data: t.data,
                        })
                        .collect(),
                ),
            },
            table_option: export_conf.table_option,
            data_format: export_conf.data_format,
        };
        asql_core::export::export_stream(pool, &export_config).await
    }

    // ══════════════════════════════════════════════════════════════════
    //  DML
    // ══════════════════════════════════════════════════════════════════

    /// Execute an `INSERT` statement built from an `InsertBuilder`.
    pub fn insert(
        &self,
        conn: &str,
        builder: &InsertBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<ModifySummary>, DbError>> + Send + '_>>
    {
        let sql = builder.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Modify(r) => r,
                _ => unreachable!(),
            };
            let inner = exec_result.data;
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: ModifySummary {
                    rows_affected: inner.rows_affected,
                    last_insert_id: inner.last_insert_id,
                },
            })
        })
    }

    /// Execute an `UPDATE` statement built from an `UpdateBuilder`.
    pub fn update(
        &self,
        conn: &str,
        builder: &UpdateBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<ModifySummary>, DbError>> + Send + '_>>
    {
        let sql = builder.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Modify(r) => r,
                _ => unreachable!(),
            };
            let inner = exec_result.data;
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: ModifySummary {
                    rows_affected: inner.rows_affected,
                    last_insert_id: inner.last_insert_id,
                },
            })
        })
    }

    /// Execute a `DELETE` statement built from a `DeleteBuilder`.
    pub fn delete(
        &self,
        conn: &str,
        builder: &DeleteBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<ModifySummary>, DbError>> + Send + '_>>
    {
        let sql = builder.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Modify(r) => r,
                _ => unreachable!(),
            };
            let inner = exec_result.data;
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: ModifySummary {
                    rows_affected: inner.rows_affected,
                    last_insert_id: inner.last_insert_id,
                },
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?builders (stateful, retain as builder input)
    // ══════════════════════════════════════════════════════════════════

    /// Execute a `CREATE TABLE` statement built from a `CreateTableBuilder`.
    pub fn create_table(
        &self,
        conn: &str,
        builder: &CreateTableBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let sql = builder.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Execute one or more `ALTER TABLE` statements built from an `AlterTableBuilder`.
    /// Each action in the builder produces a separate statement.
    pub fn alter_table(
        &self,
        conn: &str,
        builder: &AlterTableBuilder,
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let statements = builder.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let mut results = Vec::with_capacity(statements.len());
            for stmt in statements {
                let stmt = stmt.trim().to_string();
                if stmt.is_empty() {
                    continue;
                }
                match DbManager::execute_sql_send(dm.clone(), &conn, &stmt).await {
                    Ok(DbSuccessResult::Schema(r)) => results.push(Ok(ExecutionResult {
                        sql: r.sql,
                        duration_ms: r.duration_ms,
                        data: DdlSummary {},
                    })),
                    Ok(_) => unreachable!(),
                    Err(e) => results.push(Err(e)),
                }
            }
            results
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?DROP / TRUNCATE
    // ══════════════════════════════════════════════════════════════════

    /// Drop a table.
    pub fn drop_table(
        &self,
        conn: &str,
        table_name: &str,
        if_exists: bool,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let mut b =
            asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Table).name(table_name);
        if if_exists {
            b = b.if_exists();
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Drop an index.
    pub fn drop_index(
        &self,
        conn: &str,
        index_name: &str,
        table_name: &str,
        if_exists: bool,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let mut b = asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Index)
            .name(index_name)
            .on(table_name);
        if if_exists {
            b = b.if_exists();
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Drop a database.
    pub fn drop_database(
        &self,
        conn: &str,
        db_name: &str,
        if_exists: bool,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let mut b =
            asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Database).name(db_name);
        if if_exists {
            b = b.if_exists();
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Truncate a table.
    pub fn truncate_table(
        &self,
        conn: &str,
        table_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let sql = asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Table)
            .name(table_name)
            .build_truncate(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?BATCH TABLE OPERATIONS
    // ══════════════════════════════════════════════════════════════════

    /// Drop multiple tables. Returns one result per table.
    pub fn drop_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let conn = conn.to_string();
        let tables: Vec<String> = tables.iter().map(|t| t.to_string()).collect();
        let dm = self.db_manager.clone();
        let dialect_name = self.dialect.name().to_string();
        Box::pin(async move {
            let dialect = dialect_from_name(&dialect_name);
            let mut results = Vec::with_capacity(tables.len());
            for t in &tables {
                let sql = asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Table)
                    .name(t)
                    .build(&*dialect);
                match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await {
                    Ok(DbSuccessResult::Schema(r)) => results.push(Ok(ExecutionResult {
                        sql: r.sql,
                        duration_ms: r.duration_ms,
                        data: DdlSummary {},
                    })),
                    Ok(_) => unreachable!(),
                    Err(e) => results.push(Err(e)),
                }
            }
            results
        })
    }

    /// Truncate multiple tables. Returns one result per table.
    pub fn truncate_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let conn = conn.to_string();
        let tables: Vec<String> = tables.iter().map(|t| t.to_string()).collect();
        let dm = self.db_manager.clone();
        let dialect_name = self.dialect.name().to_string();
        Box::pin(async move {
            let dialect = dialect_from_name(&dialect_name);
            let mut results = Vec::with_capacity(tables.len());
            for t in &tables {
                let sql = asql_dsl::ddl::DropBuilder::new(asql_dsl::ddl::DropTarget::Table)
                    .name(t)
                    .build_truncate(&*dialect);
                match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await {
                    Ok(DbSuccessResult::Schema(r)) => results.push(Ok(ExecutionResult {
                        sql: r.sql,
                        duration_ms: r.duration_ms,
                        data: DdlSummary {},
                    })),
                    Ok(_) => unreachable!(),
                    Err(e) => results.push(Err(e)),
                }
            }
            results
        })
    }

    /// Repair one or more tables using `REPAIR TABLE`.
    pub fn repair_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let b = tables.iter().fold(
            asql_dsl::ddl::TableMaintenanceBuilder::new(asql_dsl::ddl::MaintenanceOp::Repair),
            |b, t| b.add_table(t),
        );
        let stmt = b.build(&*self.dialect);
        let conn = conn.to_string();
        let dm = self.db_manager.clone();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &stmt).await {
                Ok(DbSuccessResult::Schema(r)) => vec![Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                })],
                Ok(r) => vec![Ok(ExecutionResult {
                    sql: r.get_sql().to_string(),
                    duration_ms: r.duration_ms(),
                    data: DdlSummary {},
                })],
                Err(e) => vec![Err(e)],
            }
        })
    }

    /// Optimize one or more tables using `OPTIMIZE TABLE`.
    pub fn optimize_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let b = tables.iter().fold(
            asql_dsl::ddl::TableMaintenanceBuilder::new(asql_dsl::ddl::MaintenanceOp::Optimize),
            |b, t| b.add_table(t),
        );
        let stmt = b.build(&*self.dialect);
        let conn = conn.to_string();
        let dm = self.db_manager.clone();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &stmt).await {
                Ok(DbSuccessResult::Schema(r)) => vec![Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                })],
                Ok(r) => vec![Ok(ExecutionResult {
                    sql: r.get_sql().to_string(),
                    duration_ms: r.duration_ms(),
                    data: DdlSummary {},
                })],
                Err(e) => vec![Err(e)],
            }
        })
    }

    /// Analyze one or more tables using `ANALYZE TABLE`.
    pub fn analyze_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let b = tables.iter().fold(
            asql_dsl::ddl::TableMaintenanceBuilder::new(asql_dsl::ddl::MaintenanceOp::Analyze),
            |b, t| b.add_table(t),
        );
        let stmt = b.build(&*self.dialect);
        let conn = conn.to_string();
        let dm = self.db_manager.clone();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &stmt).await {
                Ok(DbSuccessResult::Schema(r)) => vec![Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                })],
                Ok(r) => vec![Ok(ExecutionResult {
                    sql: r.get_sql().to_string(),
                    duration_ms: r.duration_ms(),
                    data: DdlSummary {},
                })],
                Err(e) => vec![Err(e)],
            }
        })
    }

    /// Check one or more tables using `CHECK TABLE`.
    pub fn check_tables(
        &self,
        conn: &str,
        tables: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<ExecutionResult<DdlSummary>, DbError>>> + Send + '_>>
    {
        let b = tables.iter().fold(
            asql_dsl::ddl::TableMaintenanceBuilder::new(asql_dsl::ddl::MaintenanceOp::Check),
            |b, t| b.add_table(t),
        );
        let stmt = b.build(&*self.dialect);
        let conn = conn.to_string();
        let dm = self.db_manager.clone();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &stmt).await {
                Ok(DbSuccessResult::Schema(r)) => vec![Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                })],
                Ok(r) => vec![Ok(ExecutionResult {
                    sql: r.get_sql().to_string(),
                    duration_ms: r.duration_ms(),
                    data: DdlSummary {},
                })],
                Err(e) => vec![Err(e)],
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?DATABASE
    // ══════════════════════════════════════════════════════════════════

    /// Create a database.
    pub fn create_database(
        &self,
        conn: &str,
        name: &str,
        if_not_exists: bool,
        character_set: Option<&str>,
        collation: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let cs = character_set.map(|s| s.to_string());
        let coll = collation.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::DatabaseBuilder::create(name);
        if if_not_exists {
            b = b.if_not_exists();
        }
        if let Some(ref s) = cs {
            b = b.character_set(s);
        }
        if let Some(ref s) = coll {
            b = b.collation(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Alter a database.
    pub fn alter_database(
        &self,
        conn: &str,
        name: &str,
        character_set: Option<&str>,
        collation: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let cs = character_set.map(|s| s.to_string());
        let coll = collation.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::DatabaseBuilder::alter(name);
        if let Some(ref s) = cs {
            b = b.character_set(s);
        }
        if let Some(ref s) = coll {
            b = b.collation(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?INDEX
    // ══════════════════════════════════════════════════════════════════

    /// Create an index.
    pub fn create_index(
        &self,
        conn: &str,
        table: &str,
        name: &str,
        index_type: IndexType,
        columns: Vec<(&str, Option<usize>)>,
        method: Option<IndexMethod>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let cols_owned: Vec<(String, Option<usize>)> =
            columns.iter().map(|(c, p)| (c.to_string(), *p)).collect();
        let method_owned = method;
        let mut b = asql_dsl::ddl::IndexBuilder::new()
            .on(table)
            .name(name)
            .index_type(index_type);
        for (col, prefix_len) in &cols_owned {
            b = b.column(col, *prefix_len);
        }
        if let Some(m) = method_owned {
            b = b.using(m);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?USER
    // ══════════════════════════════════════════════════════════════════

    /// Create a database user.
    pub fn create_user(
        &self,
        conn: &str,
        username: &str,
        password: Option<&str>,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let pw = password.map(|s| s.to_string());
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::UserBuilder::new().create_user(username);
        if let Some(ref s) = pw {
            b = b.identified_by(s);
        }
        if let Some(ref s) = h {
            b = b.host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Alter a database user's password.
    pub fn alter_user(
        &self,
        conn: &str,
        username: &str,
        password: &str,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::UserBuilder::new()
            .alter_user(username)
            .identified_by(password);
        if let Some(ref s) = h {
            b = b.host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Drop a database user.
    pub fn drop_user(
        &self,
        conn: &str,
        username: &str,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::UserBuilder::new().drop_user(username);
        if let Some(ref s) = h {
            b = b.host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Rename a database user.
    pub fn rename_user(
        &self,
        conn: &str,
        old_username: &str,
        new_username: &str,
        old_host: Option<&str>,
        new_host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let oh = old_host.map(|s| s.to_string());
        let nh = new_host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::UserBuilder::new().rename_user(old_username, new_username);
        if let Some(ref s) = oh {
            b = b.host(s);
        }
        if let Some(ref s) = nh {
            b = b.new_host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  DDL �?GRANT / REVOKE
    // ══════════════════════════════════════════════════════════════════

    /// Grant privileges to a user.
    #[allow(clippy::too_many_arguments)]
    pub fn grant(
        &self,
        conn: &str,
        privileges: Vec<&str>,
        on: &str,
        to_user: &str,
        host: Option<&str>,
        with_grant_option: bool,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let privs: Vec<String> = privileges.iter().map(|s| s.to_string()).collect();
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::GrantBuilder::new()
            .grant(privs.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
            .on(on)
            .to(to_user);
        if let Some(ref s) = h {
            b = b.host(s);
        }
        if with_grant_option {
            b = b.with_grant_option();
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Grant a predefined role profile to a user.
    pub fn grant_role(
        &self,
        conn: &str,
        role: GrantRole,
        on: &str,
        to_user: &str,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::GrantBuilder::new()
            .role(role)
            .on(on)
            .to(to_user);
        if let Some(ref s) = h {
            b = b.host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Revoke privileges from a user.
    pub fn revoke(
        &self,
        conn: &str,
        privileges: Vec<&str>,
        on: &str,
        from_user: &str,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let privs: Vec<String> = privileges.iter().map(|s| s.to_string()).collect();
        let h = host.map(|s| s.to_string());
        let mut b = asql_dsl::ddl::GrantBuilder::new()
            .revoke(privs.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
            .on(on)
            .from(from_user);
        if let Some(ref s) = h {
            b = b.host(s);
        }
        let sql = b.build(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Columns
    // ══════════════════════════════════════════════════════════════════

    /// Show columns of a table.
    pub fn show_columns(
        &self,
        conn: &str,
        table: &str,
        database: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<ColumnDef>>, DbError>> + Send + '_>>
    {
        let sql = ColumnsIntrospection::show_columns(&*self.dialect, table, database);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let columns: Vec<ColumnDef> = exec_result
                .data
                .rows
                .iter()
                .map(|row| Self::column_def_from_row(row))
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: columns,
            })
        })
    }

    /// Show the CREATE TABLE statement for a table.
    pub fn show_create_table(
        &self,
        conn: &str,
        table: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<String>, DbError>> + Send + '_>> {
        let sql = ColumnsIntrospection::show_create_table(&*self.dialect, table);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let ddl = exec_result
                .data
                .rows
                .first()
                .and_then(|row| {
                    row.get("Create Table")
                        .or_else(|| row.get("sql"))
                        .or_else(|| row.get("?column?"))
                        .or_else(|| row.values().next())
                })
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: ddl,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Tables
    // ══════════════════════════════════════════════════════════════════

    /// List tables, optionally filtered by database/schema and name pattern.
    pub fn list_tables(
        &self,
        conn: &str,
        database: Option<&str>,
        table_name: Option<&str>,
        match_mode: TableNameMatch,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<TableInfo>>, DbError>> + Send + '_>>
    {
        let sql =
            TablesIntrospection::list_tables(&*self.dialect, database, table_name, match_mode);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let tables: Vec<TableInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| row_to_table_info(row))
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: tables,
            })
        })
    }

    /// Get metadata for a single table.
    pub fn table_info(
        &self,
        conn: &str,
        database: Option<&str>,
        table: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<TableInfo>>, DbError>> + Send + '_>>
    {
        let sql = TablesIntrospection::table_info(&*self.dialect, database, table);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let tables: Vec<TableInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| row_to_table_info(row))
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: tables,
            })
        })
    }

    /// Count tables in a database/schema.
    pub fn table_count(
        &self,
        conn: &str,
        database: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<u64>, DbError>> + Send + '_>> {
        let sql = TablesIntrospection::table_count(&*self.dialect, database);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let count = exec_result
                .data
                .rows
                .first()
                .and_then(|row| row.get("count"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u64;
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: count,
            })
        })
    }

    /// Get storage sizes for all tables in a database/schema.
    pub fn table_sizes(
        &self,
        conn: &str,
        database: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<TableSize>>, DbError>> + Send + '_>>
    {
        let sql = TablesIntrospection::table_sizes(&*self.dialect, database);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let sizes: Vec<TableSize> = exec_result
                .data
                .rows
                .iter()
                .map(|row| {
                    let size_bytes = row
                        .get("size_bytes")
                        .and_then(|v| v.as_i64())
                        .map(|b| b as u64)
                        .or_else(|| {
                            row.get("size_mb")
                                .and_then(|v| v.as_f64())
                                .map(|mb| (mb * 1024.0 * 1024.0) as u64)
                        })
                        .unwrap_or(0);
                    TableSize {
                        table_name: row
                            .get("TABLE_NAME")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        size_bytes,
                    }
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: sizes,
            })
        })
    }

    /// Get total size (bytes) of all tables in a database/schema.
    pub fn db_sizes(
        &self,
        conn: &str,
        database: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<u64>, DbError>> + Send + '_>> {
        let database = database.map(|s| s.to_string());
        let conn = conn.to_string();
        Box::pin(async move {
            let result = self.table_sizes(&conn, database.as_deref()).await?;
            let total: u64 = result.data.iter().map(|t| t.size_bytes).sum();
            Ok(ExecutionResult {
                sql: result.sql,
                duration_ms: result.duration_ms,
                data: total,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Databases
    // ══════════════════════════════════════════════════════════════════

    /// List all databases/schemas.
    pub fn list_databases(
        &self,
        conn: &str,
    ) -> Pin<
        Box<dyn Future<Output = Result<ExecutionResult<Vec<DatabaseInfo>>, DbError>> + Send + '_>,
    > {
        let sql = DatabasesIntrospection::list_databases(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let dbs: Vec<DatabaseInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| DatabaseInfo {
                    name: row
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    collation: row
                        .get("collation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: dbs,
            })
        })
    }

    /// Execute a `USE` statement to switch the active database.
    pub fn use_database(
        &self,
        conn: &str,
        db: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<DdlSummary>, DbError>> + Send + '_>>
    {
        let sql = DatabasesIntrospection::use_database(&*self.dialect, db);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Schema(r) => Ok(ExecutionResult {
                    sql: r.sql,
                    duration_ms: r.duration_ms,
                    data: DdlSummary {},
                }),
                _ => unreachable!(),
            }
        })
    }

    /// Get the current database/schema name.
    pub fn current_database(
        &self,
        conn: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<String>, DbError>> + Send + '_>> {
        let sql = DatabasesIntrospection::current_database(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let db = exec_result
                .data
                .rows
                .first()
                .and_then(|row| row.get("db"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: db,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Indexes
    // ══════════════════════════════════════════════════════════════════

    /// List indexes for a table.
    pub fn list_indexes(
        &self,
        conn: &str,
        table: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<IndexDetail>>, DbError>> + Send + '_>>
    {
        let sql = IndexesIntrospection::list_indexes(&*self.dialect, table);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let indexes: Vec<IndexDetail> = exec_result
                .data
                .rows
                .iter()
                .map(|row| IndexDetail {
                    key_name: row
                        .get("Key_name")
                        .or_else(|| row.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    column_name: row
                        .get("Column_name")
                        .or_else(|| row.get("column_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    non_unique: row
                        .get("Non_unique")
                        .or_else(|| row.get("unique"))
                        .and_then(|v| {
                            if v.is_boolean() {
                                v.as_bool()
                            } else if v.is_i64() {
                                Some(v.as_i64().unwrap() == 0)
                            } else {
                                None
                            }
                        })
                        .map(|unique| !unique)
                        .unwrap_or(true),
                    seq_in_index: row
                        .get("Seq_in_index")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    index_type: row
                        .get("Index_type")
                        .or_else(|| row.get("index_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: indexes,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Users
    // ══════════════════════════════════════════════════════════════════

    /// List all database users.
    pub fn list_users(
        &self,
        conn: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<UserInfo>>, DbError>> + Send + '_>>
    {
        let sql = UsersIntrospection::list_users(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let users: Vec<UserInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| UserInfo {
                    user: row
                        .get("User")
                        .or_else(|| row.get("user"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    host: row
                        .get("Host")
                        .or_else(|| row.get("host"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: users,
            })
        })
    }

    /// Get detailed info for a specific user.
    pub fn user_info(
        &self,
        conn: &str,
        username: &str,
        host: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<UserInfo>>, DbError>> + Send + '_>>
    {
        let sql = UsersIntrospection::user_info(&*self.dialect, username, host);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let users: Vec<UserInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| UserInfo {
                    user: row
                        .get("user")
                        .or_else(|| row.get("User"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    host: row
                        .get("host")
                        .or_else(|| row.get("Host"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: users,
            })
        })
    }

    // ══════════════════════════════════════════════════════════════════
    //  Introspection �?Server
    // ══════════════════════════════════════════════════════════════════

    /// List active server processes / connections.
    pub fn process_list(
        &self,
        conn: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<Vec<ProcessInfo>>, DbError>> + Send + '_>>
    {
        let sql = ServerIntrospection::process_list(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let processes: Vec<ProcessInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| ProcessInfo {
                    id: row
                        .get("Id")
                        .or_else(|| row.get("pid"))
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    user: row
                        .get("User")
                        .or_else(|| row.get("user"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    host: row
                        .get("Host")
                        .or_else(|| row.get("host"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    db: row
                        .get("db")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    command: row
                        .get("Command")
                        .or_else(|| row.get("state"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    time: row.get("Time").and_then(|v| v.as_i64()).unwrap_or(0),
                    state: row
                        .get("State")
                        .or_else(|| row.get("wait_event"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    info: row
                        .get("Info")
                        .or_else(|| row.get("query"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: processes,
            })
        })
    }

    /// Show server configuration variables.
    pub fn variables(
        &self,
        conn: &str,
    ) -> Pin<
        Box<dyn Future<Output = Result<ExecutionResult<Vec<VariableInfo>>, DbError>> + Send + '_>,
    > {
        let sql = ServerIntrospection::variables(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let vars: Vec<VariableInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| VariableInfo {
                    name: row
                        .get("Variable_name")
                        .or_else(|| row.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    value: row
                        .get("Value")
                        .or_else(|| row.get("setting"))
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                Some(format!("{}", v))
                            }
                        })
                        .unwrap_or_default(),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: vars,
            })
        })
    }

    /// Show server status metrics.
    pub fn status(
        &self,
        conn: &str,
    ) -> Pin<
        Box<dyn Future<Output = Result<ExecutionResult<Vec<VariableInfo>>, DbError>> + Send + '_>,
    > {
        let sql = ServerIntrospection::status(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let vars: Vec<VariableInfo> = exec_result
                .data
                .rows
                .iter()
                .map(|row| VariableInfo {
                    name: row
                        .get("Variable_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    value: row
                        .get("Value")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                Some(format!("{}", v))
                            }
                        })
                        .unwrap_or_default(),
                })
                .collect();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: vars,
            })
        })
    }

    /// Get the database server version.
    pub fn version(
        &self,
        conn: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ExecutionResult<String>, DbError>> + Send + '_>> {
        let sql = ServerIntrospection::version(&*self.dialect);
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(async move {
            let exec_result = match DbManager::execute_sql_send(dm.clone(), &conn, &sql).await? {
                DbSuccessResult::Select(r) => r,
                _ => unreachable!(),
            };
            let ver = exec_result
                .data
                .rows
                .first()
                .and_then(|row| row.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ExecutionResult {
                sql: exec_result.sql,
                duration_ms: exec_result.duration_ms,
                data: ver,
            })
        })
    }

    /// Kill one or more server processes by PID.
    ///
    /// For MySQL each PID produces a `KILL` statement (Schema result).
    /// For PostgreSQL all PIDs are terminated in a single `SELECT` query (Select result).
    /// Returns the raw execution results so callers can inspect the type.
    pub fn kill_process(
        &self,
        conn: &str,
        pids: &[&str],
    ) -> Pin<Box<dyn Future<Output = Vec<Result<DbSuccessResult, DbError>>> + Send + '_>> {
        let pids_owned: Vec<String> = pids.iter().map(|s| s.to_string()).collect();
        let pid_refs: Vec<&str> = pids_owned.iter().map(|s| s.as_str()).collect();
        let raw = ServerIntrospection::kill_process(&*self.dialect, &pid_refs);
        if raw.is_empty() {
            return Box::pin(async move { Vec::new() });
        }
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        Box::pin(
            async move { DbManager::execute_sql_batch_send(dm.clone(), &conn, &raw, false).await },
        )
    }

    // ══════════════════════════════════════════════════════════════════
    //  Raw SQL passthrough
    // ══════════════════════════════════════════════════════════════════

    /// Execute an arbitrary SQL statement.
    pub fn execute_raw(
        &self,
        conn: &str,
        sql: &str,
    ) -> Pin<Box<dyn Future<Output = Result<DbSuccessResult, DbError>> + Send + '_>> {
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        let sql = sql.to_string();
        Box::pin(async move { DbManager::execute_sql_send(dm.clone(), &conn, &sql).await })
    }

    /// Execute multiple semicolon-separated SQL statements in a batch.
    pub fn execute_raw_batch(
        &self,
        conn: &str,
        sql: &str,
    ) -> Pin<Box<dyn Future<Output = Vec<Result<DbSuccessResult, DbError>>> + Send + '_>> {
        let dm = self.db_manager.clone();
        let conn = conn.to_string();
        let sql = sql.to_string();
        Box::pin(
            async move { DbManager::execute_sql_batch_send(dm.clone(), &conn, &sql, false).await },
        )
    }
}

/// Create a dialect from a dialect name string (e.g. "MySQL", "PostgreSQL", "SQLite").
fn dialect_from_name(name: &str) -> Box<dyn Dialect + Send + Sync> {
    match name {
        "MySQL" => Box::new(MySql),
        "PostgreSQL" => Box::new(PostgreSql),
        "SQLite" => Box::new(Sqlite),
        _ => Box::new(MySql),
    }
}

fn row_to_table_info(row: &DbRow) -> TableInfo {
    TableInfo {
        table_name: row
            .get("TABLE_NAME")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        table_comment: row
            .get("TABLE_COMMENT")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        engine: row
            .get("ENGINE")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        table_collation: row
            .get("TABLE_COLLATION")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        table_rows: row
            .get("TABLE_ROWS")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64)
            .unwrap_or(0),
        table_size: row
            .get("TABLE_SIZE")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64)
            .unwrap_or(0),
        data_length: row
            .get("DATA_LENGTH")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64),
        index_length: row
            .get("INDEX_LENGTH")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64),
        data_free: row
            .get("DATA_FREE")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64),
        auto_increment: row
            .get("AUTO_INCREMENT")
            .and_then(|v| v.as_i64())
            .map(|v| v as u64),
    }
}

// ══════════════════════════════════════════════════════════════════
//  Preset / metadata queries
// ══════════════════════════════════════════════════════════════════

pub fn database_type_params(db_type: DatabaseType) -> ConnectionParams {
    match db_type {
        DatabaseType::MySql => ConnectionParams::MySql {
            host: String::new(),
            port: 3306,
            user: String::new(),
            password: None,
            database: None,
        },
        DatabaseType::Postgres => ConnectionParams::Postgres {
            host: String::new(),
            port: 5432,
            user: String::new(),
            password: None,
            database: None,
        },
        DatabaseType::Sqlite => ConnectionParams::Sqlite {
            path: String::new(),
        },
    }
}

pub fn database_types() -> Vec<(DbTypeInfo, ConnectionParams)> {
    DATABASE_TYPES
        .iter()
        .map(|info| {
            let db_type: DatabaseType = match info.enum_name {
                "MySql" => DatabaseType::MySql,
                "Postgres" => DatabaseType::Postgres,
                "Sqlite" => DatabaseType::Sqlite,
                _ => unreachable!(),
            };
            (info.clone(), database_type_params(db_type))
        })
        .collect()
}

pub fn data_types_of(db_type: DatabaseType) -> &'static [DataType] {
    db_type.data_types_info()
}

pub fn functions_of(db_type: DatabaseType) -> &'static [DbFunction] {
    db_type.functions_info()
}

pub fn engines_of(db_type: DatabaseType) -> &'static [Engine] {
    db_type.engines()
}

pub fn charsets_of(db_type: DatabaseType) -> &'static [Charset] {
    db_type.charsets()
}

pub fn privileges_of(db_type: DatabaseType) -> &'static [Privilege] {
    db_type.privileges()
}

pub fn sql_modes_of(db_type: DatabaseType) -> &'static [SqlMode] {
    db_type.sql_modes()
}
