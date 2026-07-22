pub mod config;
pub mod model;
pub mod sync;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use asql_query::{DbManager, MySql, QueryBuilder};
pub use asql_query::{ConnectionConfig, DbError, DbSuccessResult, DatabaseType};
pub use config::AppConfig;
pub use sync::BackendSync;

pub type DbType = DatabaseType;

pub struct Backend {
    config: AppConfig,
    query_builder: QueryBuilder,
    active_connection: Option<String>,
}

impl Backend {
    pub fn new(config_dir: PathBuf) -> Self {
        let config = AppConfig::new(config_dir);
        let db_manager = Arc::new(DbManager::new());
        let query_builder = QueryBuilder::new(MySql, db_manager);
        Self {
            config,
            query_builder,
            active_connection: None,
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn query_builder(&self) -> &QueryBuilder {
        &self.query_builder
    }

    pub async fn load_connections(&self) -> Vec<ConnectionConfig> {
        let path = self.config.connections_path();
        if path.exists() {
            match self.query_builder.load_connections(&path).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to load connections: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    pub async fn save_connections(&self, configs: &[ConnectionConfig]) -> Result<(), DbError> {
        self.query_builder
            .db_manager()
            .save_configs(configs, &self.config.connections_path())
            .await
    }

    pub async fn list_connections(&self) -> Vec<(String, String, DatabaseType)> {
        self.query_builder.list_connections().await
    }

    pub async fn add_connection(&self, config: ConnectionConfig) -> Result<(), DbError> {
        self.query_builder.add_connection(config).await
    }

    pub async fn remove_connection(&self, name: &str) -> bool {
        self.query_builder.remove_connection(name).await
    }

    pub async fn test_connection(&self, name: &str) -> Result<(), DbError> {
        self.query_builder.test_connection(name).await
    }

    pub async fn get_connection_url(&self, name: &str) -> Option<String> {
        self.query_builder.get_connection_url(name).await
    }

    pub fn active_connection(&self) -> Option<&str> {
        self.active_connection.as_deref()
    }

    pub fn set_active_connection(&mut self, name: Option<String>) {
        self.active_connection = name;
    }

    pub async fn execute_sql(
        &self,
        connection: &str,
        sql: &str,
    ) -> Vec<Result<DbSuccessResult, DbError>> {
        self.query_builder
            .execute_raw_batch(connection, sql)
            .await
    }

    pub async fn save_and_reload_connections(&self) {
        let configs: Vec<ConnectionConfig> = self
            .query_builder
            .db_manager()
            .list_connections()
            .await
            .into_iter()
            .map(|(_, item)| item.config)
            .collect();
        let _ = self.save_connections(&configs).await;
    }

    pub fn db_manager(&self) -> &Arc<DbManager> {
        self.query_builder.db_manager()
    }
}

#[derive(Clone)]
pub struct BackendHandle {
    inner: Arc<RwLock<Backend>>,
}

impl BackendHandle {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Backend::new(config_dir))),
        }
    }

    pub async fn db_manager(&self) -> Arc<DbManager> {
        self.inner.read().await.db_manager().clone()
    }

    pub async fn load_connections(&self) -> Vec<ConnectionConfig> {
        self.inner.read().await.load_connections().await
    }

    pub async fn list_connections(&self) -> Vec<(String, String, DatabaseType)> {
        self.inner.read().await.list_connections().await
    }

    pub async fn add_connection(&self, config: ConnectionConfig) -> Result<(), DbError> {
        self.inner.read().await.add_connection(config).await
    }

    pub async fn remove_connection(&self, name: &str) -> bool {
        self.inner.read().await.remove_connection(name).await
    }

    pub async fn save_and_reload_connections(&self) {
        self.inner.read().await.save_and_reload_connections().await
    }

    pub async fn set_active_connection(&self, name: Option<String>) {
        self.inner.write().await.set_active_connection(name);
    }

    pub async fn test_connection(&self, name: &str) -> Result<(), DbError> {
        let conn = name.to_string();
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            let backend = inner.blocking_read();
            rt.block_on(backend.test_connection(&conn))
        })
        .await
        .unwrap()
    }

    pub async fn execute_sql(
        &self,
        connection: &str,
        sql: &str,
    ) -> Vec<Result<DbSuccessResult, DbError>> {
        let conn = connection.to_string();
        let sql = sql.to_string();
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            let backend = inner.blocking_read();
            rt.block_on(backend.execute_sql(&conn, &sql))
        })
        .await
        .unwrap()
    }
}
