use std::path::PathBuf;
use std::sync::Arc;

use asql_query::{DbManager, ConnectionConfig};

use crate::config::AppConfig;
use crate::Backend;
use crate::{DbError, DbSuccessResult};

pub struct BackendSync {
    pub(crate) backend: Backend,
    pub(crate) rt: tokio::runtime::Runtime,
}

impl BackendSync {
    pub fn new(config_dir: PathBuf) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let backend = Backend::new(config_dir);
        Self { backend, rt }
    }

    pub fn config(&self) -> &AppConfig {
        self.backend.config()
    }

    pub fn load_connections(&self) -> Vec<ConnectionConfig> {
        self.rt.block_on(self.backend.load_connections())
    }

    pub fn save_connections(&self, configs: &[ConnectionConfig]) -> Result<(), DbError> {
        self.rt.block_on(self.backend.save_connections(configs))
    }

    pub fn list_connections(&self) -> Vec<(String, String, crate::DbType)> {
        self.rt.block_on(self.backend.list_connections())
    }

    pub fn add_connection_sync(&self, config: ConnectionConfig) -> Result<(), DbError> {
        self.rt.block_on(self.backend.add_connection(config))
    }

    pub fn remove_connection_sync(&self, name: &str) -> bool {
        self.rt.block_on(self.backend.remove_connection(name))
    }

    pub fn get_connection_url_sync(&self, name: &str) -> Option<String> {
        self.rt.block_on(self.backend.get_connection_url(name))
    }

    pub fn active_connection(&self) -> Option<&str> {
        self.backend.active_connection()
    }

    pub fn set_active_connection(&mut self, name: Option<String>) {
        self.backend.set_active_connection(name)
    }

    pub fn execute_sql_sync(
        &self,
        connection: &str,
        sql: &str,
    ) -> Vec<Result<DbSuccessResult, DbError>> {
        self.rt.block_on(self.backend.execute_sql(connection, sql))
    }

    pub fn save_and_reload_connections(&self) {
        self.rt.block_on(self.backend.save_and_reload_connections())
    }

    pub fn db_manager(&self) -> &Arc<DbManager> {
        self.backend.query_builder().db_manager()
    }
}
