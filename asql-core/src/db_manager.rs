use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use sqlx::{AssertSqlSafe, Connection};
use tokio::sync::{Mutex, RwLock};

use crate::db_executor;
use crate::persistence::{self, ConnectionConfig};
use crate::result::{DbError, DbSuccessResult, ExecutionResult};

pub use asql_types::DatabaseType;

pub struct MySqlPool(pub Mutex<sqlx::MySqlConnection>);
pub struct PgPool(pub Mutex<sqlx::PgConnection>);
pub struct SqlitePool(pub Mutex<sqlx::SqliteConnection>);

pub enum Pool {
    MySql(MySqlPool),
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

/// Cumulative progress of an owned batch execution.
#[derive(Default, Clone)]
pub struct BatchProgress {
    pub done: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub errors: Vec<(usize, String)>,
    pub fatal_error: Option<String>,
    pub cancelled: bool,
}

async fn run_batch_mysql(
    conn: &mut sqlx::MySqlConnection,
    statements: &[String],
    stop_on_error: bool,
    single_transaction: bool,
    progress: &mut BatchProgress,
    on_progress: &mut (impl FnMut(&BatchProgress) -> bool + Send),
) {
    if single_transaction {
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe("SET autocommit=0"))
            .execute(&mut *conn)
            .await
        {
            progress.fatal_error = Some(format!("Failed to set autocommit=0: {}", e));
            return;
        }
    }
    for (i, stmt) in statements.iter().enumerate() {
        match db_executor::execute_sql_on_mysql_conn(stmt, conn).await {
            Ok(_) => progress.succeeded += 1,
            Err(e) => {
                progress.failed += 1;
                progress.errors.push((i, format!("{}", e)));
                if stop_on_error {
                    progress.done = i + 1;
                    on_progress(progress);
                    break;
                }
            }
        }
        progress.done = i + 1;
        if !on_progress(progress) {
            progress.cancelled = true;
            break;
        }
    }
    if single_transaction {
        let finalize = if (stop_on_error && progress.failed > 0) || progress.cancelled {
            "ROLLBACK"
        } else {
            "COMMIT"
        };
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe(finalize)).execute(&mut *conn).await {
            progress.fatal_error = Some(format!("Failed to {}: {}", finalize, e));
        }
    }
}

async fn run_batch_pg(
    conn: &mut sqlx::PgConnection,
    statements: &[String],
    stop_on_error: bool,
    single_transaction: bool,
    progress: &mut BatchProgress,
    on_progress: &mut (impl FnMut(&BatchProgress) -> bool + Send),
) {
    if single_transaction {
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe("BEGIN")).execute(&mut *conn).await {
            progress.fatal_error = Some(format!("Failed to BEGIN: {}", e));
            return;
        }
    }
    for (i, stmt) in statements.iter().enumerate() {
        match db_executor::execute_sql_on_pg_conn(stmt, conn).await {
            Ok(_) => progress.succeeded += 1,
            Err(e) => {
                progress.failed += 1;
                progress.errors.push((i, format!("{}", e)));
                if stop_on_error {
                    progress.done = i + 1;
                    on_progress(progress);
                    break;
                }
            }
        }
        progress.done = i + 1;
        if !on_progress(progress) {
            progress.cancelled = true;
            break;
        }
    }
    if single_transaction {
        let finalize = if (stop_on_error && progress.failed > 0) || progress.cancelled {
            "ROLLBACK"
        } else {
            "COMMIT"
        };
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe(finalize)).execute(&mut *conn).await {
            progress.fatal_error = Some(format!("Failed to {}: {}", finalize, e));
        }
    }
}

async fn run_batch_sqlite(
    conn: &mut sqlx::SqliteConnection,
    statements: &[String],
    stop_on_error: bool,
    single_transaction: bool,
    progress: &mut BatchProgress,
    on_progress: &mut (impl FnMut(&BatchProgress) -> bool + Send),
) {
    if single_transaction {
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe("BEGIN")).execute(&mut *conn).await {
            progress.fatal_error = Some(format!("Failed to BEGIN: {}", e));
            return;
        }
    }
    for (i, stmt) in statements.iter().enumerate() {
        match db_executor::execute_sql_on_sqlite_conn(stmt, conn).await {
            Ok(_) => progress.succeeded += 1,
            Err(e) => {
                progress.failed += 1;
                progress.errors.push((i, format!("{}", e)));
                if stop_on_error {
                    progress.done = i + 1;
                    on_progress(progress);
                    break;
                }
            }
        }
        progress.done = i + 1;
        if !on_progress(progress) {
            progress.cancelled = true;
            break;
        }
    }
    if single_transaction {
        let finalize = if (stop_on_error && progress.failed > 0) || progress.cancelled {
            "ROLLBACK"
        } else {
            "COMMIT"
        };
        if let Err(e) = sqlx::raw_sql(AssertSqlSafe(finalize)).execute(&mut *conn).await {
            progress.fatal_error = Some(format!("Failed to {}: {}", finalize, e));
        }
    }
}

impl Pool {
    pub fn db_type(&self) -> DatabaseType {
        match self {
            Pool::MySql(_) => DatabaseType::MySql,
            Pool::Postgres(_) => DatabaseType::Postgres,
            Pool::Sqlite(_) => DatabaseType::Sqlite,
        }
    }

    pub async fn close(self) {
        drop(self);
    }

    /// Create a new pool from a URL and database type.
    pub async fn connect(url: &str, db_type: DatabaseType) -> Result<Self, DbError> {
        match db_type {
            DatabaseType::MySql => {
                let conn = sqlx::MySqlConnection::connect(url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Ok(Pool::MySql(MySqlPool(Mutex::new(conn))))
            }
            DatabaseType::Postgres => {
                let conn = sqlx::PgConnection::connect(url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Ok(Pool::Postgres(PgPool(Mutex::new(conn))))
            }
            DatabaseType::Sqlite => {
                let conn = sqlx::SqliteConnection::connect(url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Ok(Pool::Sqlite(SqlitePool(Mutex::new(conn))))
            }
        }
    }

    /// Execute a single SQL statement against this pool.
    pub async fn execute_sql(&self, sql: &str) -> Result<DbSuccessResult, DbError> {
        use crate::db_executor::{
            classify_sql, execute_generic_mysql, execute_generic_pg, execute_generic_sqlite,
            execute_modify_mysql, execute_modify_pg, execute_modify_sqlite, execute_select_mysql,
            execute_select_pg, execute_select_sqlite, SqlKind,
        };

        let trimmed_sql = sql.trim();
        let kind = classify_sql(trimmed_sql);
        let start = Instant::now();
        let get_duration = || start.elapsed().as_millis() as u64;
        let sql = sql.to_string();
        match self {
            Pool::MySql(p) => {
                let mut conn = p.0.lock().await;
                match kind {
                    SqlKind::Query => execute_select_mysql(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Select(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Modify => execute_modify_mysql(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Modify(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Schema => execute_generic_mysql(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Schema(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                }
            }
            Pool::Postgres(p) => {
                let mut conn = p.0.lock().await;
                match kind {
                    SqlKind::Query => execute_select_pg(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Select(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Modify => execute_modify_pg(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Modify(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Schema => execute_generic_pg(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Schema(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                }
            }
            Pool::Sqlite(p) => {
                let mut conn = p.0.lock().await;
                match kind {
                    SqlKind::Query => execute_select_sqlite(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Select(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Modify => execute_modify_sqlite(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Modify(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                    SqlKind::Schema => execute_generic_sqlite(trimmed_sql, &mut *conn)
                        .await
                        .map(|data| DbSuccessResult::Schema(ExecutionResult { sql: sql.clone(), duration_ms: get_duration(), data })),
                }
            }
        }
    }

    /// Execute multiple statements in sequence, holding the connection lock for the entire batch.
    pub async fn execute_batch(
        &self,
        statements: &[&str],
        stop_on_error: bool,
        conn_name: &str,
    ) -> Vec<Result<DbSuccessResult, DbError>> {
        let mut results = Vec::with_capacity(statements.len());
        match self {
            Pool::MySql(p) => {
                let mut conn = p.0.lock().await;
                for stmt in statements {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let result = db_executor::execute_sql_on_mysql_conn(trimmed, &mut *conn).await;
                    Self::log_batch_result(conn_name, &result);
                    results.push(result);
                    if stop_on_error && results.last().unwrap().is_err() {
                        break;
                    }
                }
            }
            Pool::Postgres(p) => {
                let mut conn = p.0.lock().await;
                for stmt in statements {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let result = db_executor::execute_sql_on_pg_conn(trimmed, &mut *conn).await;
                    Self::log_batch_result(conn_name, &result);
                    results.push(result);
                    if stop_on_error && results.last().unwrap().is_err() {
                        break;
                    }
                }
            }
            Pool::Sqlite(p) => {
                let mut conn = p.0.lock().await;
                for stmt in statements {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let result = db_executor::execute_sql_on_sqlite_conn(trimmed, &mut *conn).await;
                    Self::log_batch_result(conn_name, &result);
                    results.push(result);
                    if stop_on_error && results.last().unwrap().is_err() {
                        break;
                    }
                }
            }
        }
        results
    }

    fn log_batch_result(name: &str, result: &Result<DbSuccessResult, DbError>) {
        match result {
            Ok(r) => tracing::info!(
                "[SQL BATCH DONE] [{}] {} ({}ms)",
                name,
                r.kind_label(),
                r.duration_ms()
            ),
            Err(e) => tracing::error!("[SQL BATCH FAIL] [{}] {}", name, e),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionItem {
    pub pool: Option<Arc<Pool>>,
    pub config: ConnectionConfig,
}

pub struct DbManager {
    pools: Arc<RwLock<HashMap<String, ConnectionItem>>>,
}

impl DbManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, config: ConnectionConfig) -> Result<(), DbError> {
        let mut pools = self.pools.write().await;
        if let Some(item) = pools.get_mut(&config.name) {
            item.config = config;
        } else {
            pools.insert(
                config.name.clone(),
                ConnectionItem {
                    pool: None,
                    config,
                },
            );
        }

        Ok(())
    }

    pub async fn open_connection(&self, name: &str) -> Result<(), DbError> {
        {
            let pools = self.pools.read().await;
            if let Some(item) = pools.get(name) {
                if item.pool.is_some() {
                    return Ok(());
                }
            } else {
                return Err(DbError::ConnectionError(format!(
                    "Connection '{}' not found",
                    name
                )));
            }
        }

        let (url, db_type) = {
            let pools = self.pools.read().await;
            let item = pools.get(name).unwrap();
            (item.config.params.to_url(), item.config.params.db_type())
        };

        let pool = match db_type {
            DatabaseType::MySql => {
                let conn = sqlx::MySqlConnection::connect(&url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Pool::MySql(MySqlPool(Mutex::new(conn)))
            }
            DatabaseType::Postgres => {
                let conn = sqlx::PgConnection::connect(&url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Pool::Postgres(PgPool(Mutex::new(conn)))
            }
            DatabaseType::Sqlite => {
                let conn = sqlx::SqliteConnection::connect(&url)
                    .await
                    .map_err(|e: sqlx::Error| DbError::ConnectionError(e.to_string()))?;
                Pool::Sqlite(SqlitePool(Mutex::new(conn)))
            }
        };

        let mut pools = self.pools.write().await;
        if let Some(item) = pools.get_mut(name) {
            item.pool = Some(Arc::new(pool));
        }

        Ok(())
    }

    pub async fn remove_connection(&self, name: &str) -> bool {
        let mut pools = self.pools.write().await;
        if let Some(item) = pools.remove(name) {
            if let Some(p) = item.pool {
                if let Ok(pool) = Arc::try_unwrap(p) {
                    pool.close().await;
                }
            }
            true
        } else {
            false
        }
    }

    pub async fn close_connection(&self, name: &str) -> bool {
        let mut pools = self.pools.write().await;
        if let Some(item) = pools.get_mut(name) {
            item.pool = None;
            return true;
        }
        false
    }

    pub async fn list_connections(&self) -> Vec<(String, ConnectionItem)> {
        let pools = self.pools.read().await;
        pools.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub async fn get_connection_url(&self, name: &str) -> Option<String> {
        let pools = self.pools.read().await;
        let keys: Vec<&String> = pools.keys().collect();
        let result = pools.get(name).map(|item| item.config.params.to_url());
        tracing::debug!(
            lookup = %name,
            found = result.is_some(),
            available = ?keys,
            "get_connection_url"
        );
        result
    }

    pub async fn get_connection_config(&self, name: &str) -> Option<ConnectionConfig> {
        let pools = self.pools.read().await;
        pools.get(name).map(|item| item.config.clone())
    }

    pub async fn get_pool(&self, name: &str) -> Option<Arc<Pool>> {
        let pools = self.pools.read().await;
        pools.get(name).and_then(|item| item.pool.clone())
    }

    /// Execute a SQL statement, wrapping the async state machine in `spawn_blocking` so
    /// the returned future is unconditionally `Send`. Useful when called from generic
    /// async contexts (axum handlers) that require a `Send` bound.
    pub fn execute_sql_send(
        dm: Arc<Self>,
        name: &str,
        sql: &str,
    ) -> Pin<Box<dyn Future<Output = Result<DbSuccessResult, DbError>> + Send>> {
        let name = name.to_string();
        let sql = sql.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(dm.execute_sql(&name, &sql))
            })
            .await
            .unwrap()
        })
    }

    /// Execute a SQL batch, wrapping the async state machine in `spawn_blocking` so
    /// the returned future is unconditionally `Send`.
    pub fn execute_sql_batch_send(
        dm: Arc<Self>,
        name: &str,
        sql: &str,
        stop_on_error: bool,
    ) -> Pin<Box<dyn Future<Output = Vec<Result<DbSuccessResult, DbError>>> + Send>> {
        let name = name.to_string();
        let sql = sql.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(dm.execute_sql_batch(&name, &sql, stop_on_error))
            })
            .await
            .unwrap()
        })
    }

    /// Execute an owned list of statements on a single connection, reporting
    /// progress via `on_progress`. When `new_connection` is true a brand-new
    /// connection is opened (and closed afterwards), so the pool's main
    /// connection is not blocked and session state (e.g. `USE database`) cannot
    /// leak between operations. SQLite always uses the pool connection.
    pub fn execute_sql_batch_owned_send(
        dm: Arc<Self>,
        name: &str,
        database: Option<String>,
        statements: Vec<String>,
        stop_on_error: bool,
        new_connection: bool,
        single_transaction: bool,
        on_progress: impl FnMut(&BatchProgress) -> bool + Send + 'static,
    ) -> Pin<Box<dyn Future<Output = BatchProgress> + Send>> {
        let name = name.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(dm.run_batch_owned(
                    &name,
                    database,
                    statements,
                    stop_on_error,
                    new_connection,
                    single_transaction,
                    on_progress,
                ))
            })
            .await
            .unwrap_or_default()
        })
    }

    async fn run_batch_owned(
        &self,
        name: &str,
        database: Option<String>,
        statements: Vec<String>,
        stop_on_error: bool,
        new_connection: bool,
        single_transaction: bool,
        mut on_progress: impl FnMut(&BatchProgress) -> bool + Send,
    ) -> BatchProgress {
        let mut progress = BatchProgress::default();

        let (url, db_type) = {
            let pools = self.pools.read().await;
            match pools.get(name) {
                Some(item) => (item.config.params.to_url(), item.config.params.db_type()),
                None => {
                    progress.fatal_error = Some(format!("Connection '{}' not found", name));
                    return progress;
                }
            }
        };

        let use_new = new_connection && !matches!(db_type, DatabaseType::Sqlite);
        if use_new {
            match db_type {
                DatabaseType::MySql => {
                    let mut conn = match sqlx::MySqlConnection::connect(&url).await {
                        Ok(c) => c,
                        Err(e) => {
                            progress.fatal_error =
                                Some(format!("Failed to open new connection: {}", e));
                            return progress;
                        }
                    };
                    if let Some(db) = &database {
                        let escaped = db.replace('`', "``");
                        let use_sql = format!("USE `{}`", escaped);
                        if let Err(e) = sqlx::raw_sql(AssertSqlSafe(use_sql.as_str()))
                            .execute(&mut conn)
                            .await
                        {
                            progress.fatal_error =
                                Some(format!("Failed to USE database '{}': {}", db, e));
                            return progress;
                        }
                    }
                    run_batch_mysql(&mut conn, &statements, stop_on_error, single_transaction, &mut progress, &mut on_progress)
                        .await;
                }
                DatabaseType::Postgres => {
                    if database.is_some() {
                        tracing::warn!(
                            "[SQL BATCH] PostgreSQL has no USE statement; ignoring database parameter"
                        );
                    }
                    let mut conn = match sqlx::PgConnection::connect(&url).await {
                        Ok(c) => c,
                        Err(e) => {
                            progress.fatal_error =
                                Some(format!("Failed to open new connection: {}", e));
                            return progress;
                        }
                    };
                    run_batch_pg(&mut conn, &statements, stop_on_error, single_transaction, &mut progress, &mut on_progress)
                        .await;
                }
                DatabaseType::Sqlite => unreachable!(),
            }
            return progress;
        }

        if matches!(db_type, DatabaseType::Sqlite) && new_connection {
            tracing::warn!(
                "[SQL BATCH] SQLite always uses the pool connection; ignoring new_connection"
            );
        }
        if self.get_pool(name).await.is_none() {
            if let Err(e) = self.open_connection(name).await {
                progress.fatal_error = Some(format!("{}", e));
                return progress;
            }
        }
        let pool = match self.get_pool(name).await {
            Some(p) => p,
            None => {
                progress.fatal_error = Some(format!("Connection '{}' not found", name));
                return progress;
            }
        };
        match &*pool {
            Pool::MySql(p) => {
                let mut conn = p.0.lock().await;
                run_batch_mysql(&mut *conn, &statements, stop_on_error, single_transaction, &mut progress, &mut on_progress)
                    .await;
            }
            Pool::Postgres(p) => {
                let mut conn = p.0.lock().await;
                run_batch_pg(&mut *conn, &statements, stop_on_error, single_transaction, &mut progress, &mut on_progress)
                    .await;
            }
            Pool::Sqlite(p) => {
                let mut conn = p.0.lock().await;
                run_batch_sqlite(
                    &mut *conn,
                    &statements,
                    stop_on_error,
                    single_transaction,
                    &mut progress,
                    &mut on_progress,
                )
                .await;
            }
        }
        progress
    }

    pub async fn execute_sql(&self, name: &str, sql: &str) -> Result<DbSuccessResult, DbError> {
        if self.get_pool(name).await.is_none() {
            self.open_connection(name).await?;
        }
        let pool = self
            .get_pool(name)
            .await
            .ok_or_else(|| DbError::ConnectionError(format!("Connection '{}' not found", name)))?;
        let result = pool.execute_sql(sql).await;
        match &result {
            Ok(r) => tracing::info!(
                "[SQL DONE] [{}] {} ({}ms)",
                name,
                r.kind_label(),
                r.duration_ms()
            ),
            Err(e) => tracing::error!("[SQL FAIL] [{}] {}", name, e),
        }
        result
    }

    pub async fn execute_sql_batch(
        &self,
        name: &str,
        sql: &str,
        stop_on_error: bool,
    ) -> Vec<Result<DbSuccessResult, DbError>> {
        if self.get_pool(name).await.is_none() {
            if let Err(e) = self.open_connection(name).await {
                return vec![Err(e)];
            }
        }
        let pool = match self.get_pool(name).await {
            Some(p) => p,
            None => {
                return vec![Err(DbError::ConnectionError(format!(
                    "Connection '{}' not found",
                    name
                )))];
            }
        };

        let statements = db_executor::split_sql_statements(sql);
        tracing::info!(
            "[SQL BATCH] [{}] {} statement(s)",
            name,
            statements.len()
        );

        // Lock the single connection once for the entire batch so session state
        // (e.g. `USE database`) is preserved across all statements.
        pool.execute_batch(&statements, stop_on_error, name).await
    }

    pub async fn save_configs(&self, configs: &[ConnectionConfig], path: &Path) -> Result<(), DbError> {
        persistence::save(configs, path).await
    }

    pub async fn load_configs(&self, path: &Path) -> Result<Vec<ConnectionConfig>, DbError> {
        persistence::load(path).await
    }
}

impl Default for DbManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ConnectionItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("ConnectionItem")
            .field("name", &self.config.name)
            .finish()
    }
}

