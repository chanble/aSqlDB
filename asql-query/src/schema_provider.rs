//! Lazy-loading schema provider backed by a live database connection.
//!
//! Used by `asql_sql::Completer` to fetch table/column metadata on demand
//! instead of pre-loading the whole schema. Each connection owns one
//! provider instance with a small in-memory TTL cache.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::RwLock;

use asql_sql::SchemaProvider;

use crate::{ConnectionParams, QueryBuilder, TableNameMatch};

const DEFAULT_TTL: Duration = Duration::from_secs(30);

pub struct DbSchemaProvider {
    qb: Arc<QueryBuilder>,
    conn: String,
    /// Resolved once at construction: prefer the configured database,
    /// fall back to `current_database()` when the config has none.
    db: Option<String>,
    ttl: Duration,
    /// Cached list of all table names + fetch timestamp.
    tables_cache: RwLock<Option<(Instant, Vec<String>)>>,
    /// Per-table column cache: table -> (fetched_at, columns).
    columns_cache: RwLock<HashMap<String, (Instant, Vec<(String, String)>)>>,
}

impl DbSchemaProvider {
    pub async fn new(qb: Arc<QueryBuilder>, conn: String) -> Self {
        Self::with_ttl(DEFAULT_TTL, qb, conn).await
    }

    pub async fn with_ttl(ttl: Duration, qb: Arc<QueryBuilder>, conn: String) -> Self {
        let db = Self::resolve_db(&qb, &conn).await;
        tracing::info!(conn = %conn, db = ?db, "DbSchemaProvider resolved database");
        Self {
            qb,
            conn,
            db,
            ttl,
            tables_cache: RwLock::new(None),
            columns_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Drop cached columns for a single table (call after DDL on it).
    pub async fn invalidate_table(&self, table: &str) {
        self.columns_cache.write().await.remove(table);
    }

    /// Drop all cached metadata (call when switching database).
    pub async fn invalidate_all(&self) {
        *self.tables_cache.write().await = None;
        self.columns_cache.write().await.clear();
    }

    /// Resolve the database to introspect. Strategy:
    /// 1. Look at the connection config's `database` field.
    /// 2. Fall back to querying `current_database()` once.
    async fn resolve_db(qb: &Arc<QueryBuilder>, conn: &str) -> Option<String> {
        if let Some(cfg) = qb.db_manager().get_connection_config(conn).await {
            let configured = match cfg.params {
                ConnectionParams::MySql { database, .. } => database,
                ConnectionParams::Postgres { database, .. } => database,
                ConnectionParams::Sqlite { .. } => None,
            };
            if let Some(db) = configured.filter(|s| !s.is_empty()) {
                return Some(db);
            }
        }
        qb.current_database(conn)
            .await
            .ok()
            .map(|r| r.data)
            .filter(|s| !s.is_empty())
    }

    async fn fetch_all_table_names(&self) -> Vec<String> {
        let db = self.db.as_deref();
        match self.qb.list_tables(&self.conn, db, None, TableNameMatch::Contains).await {
            Ok(res) => {
                let names: Vec<String> = res.data.into_iter().map(|t| t.table_name).collect();
                if names.is_empty() {
                    tracing::error!(
                        conn = %self.conn,
                        db = ?self.db,
                        "list_tables returned 0 rows — check db selection and permissions"
                    );
                } else {
                    tracing::info!(
                        conn = %self.conn,
                        db = ?self.db,
                        count = names.len(),
                        "list_tables fetched"
                    );
                }
                names
            }
            Err(e) => {
                tracing::error!(
                    conn = %self.conn,
                    db = ?self.db,
                    error = %e,
                    "list_tables failed in DbSchemaProvider"
                );
                Vec::new()
            }
        }
    }

    async fn fetch_columns(&self, table: &str) -> Vec<(String, String)> {
        let db = self.db.as_deref();

        // Validate the table actually exists before hitting the DB with
        // SHOW FULL COLUMNS — partial/unknown table names would otherwise
        // produce noisy 1146 errors in the log.
        {
            let cache = self.tables_cache.read().await;
            let need_fetch = cache
                .as_ref()
                .map(|(fetched_at, names)| {
                    fetched_at.elapsed() >= self.ttl || !names.iter().any(|n| n == table)
                })
                .unwrap_or(true);
            drop(cache);
            if need_fetch {
                let names = self.fetch_all_table_names().await;
                *self.tables_cache.write().await = Some((Instant::now(), names.clone()));
                if !names.iter().any(|n| n == table) {
                    tracing::debug!(
                        conn = %self.conn,
                        db = ?self.db,
                        table = %table,
                        "skip show_columns: table not in schema"
                    );
                    return Vec::new();
                }
            } else if let Some((_, names)) = self.tables_cache.read().await.as_ref() {
                if !names.iter().any(|n| n == table) {
                    tracing::debug!(
                        conn = %self.conn,
                        db = ?self.db,
                        table = %table,
                        "skip show_columns: table not in schema"
                    );
                    return Vec::new();
                }
            }
        }

        match self.qb.show_columns(&self.conn, table, db).await {
            Ok(res) => {
                let cols: Vec<(String, String)> = res
                    .data
                    .into_iter()
                    .map(|c| (c.name, c.col_type.to_sql()))
                    .collect();
                if cols.is_empty() {
                    tracing::error!(
                        conn = %self.conn,
                        db = ?self.db,
                        table = %table,
                        "show_columns returned 0 rows"
                    );
                }
                cols
            }
            Err(e) => {
                tracing::error!(
                    conn = %self.conn,
                    db = ?self.db,
                    table = %table,
                    error = %e,
                    "show_columns failed in DbSchemaProvider"
                );
                Vec::new()
            }
        }
    }
}

#[async_trait]
impl SchemaProvider for DbSchemaProvider {
    async fn table_names(&self, prefix: &str, limit: usize) -> Vec<String> {
        // Try cache first.
        {
            let cache = self.tables_cache.read().await;
            if let Some((fetched_at, names)) = cache.as_ref() {
                if fetched_at.elapsed() < self.ttl {
                    let prefix_lower = prefix.to_lowercase();
                    return names
                        .iter()
                        .filter(|n| prefix.is_empty() || n.to_lowercase().contains(&prefix_lower))
                        .take(limit)
                        .cloned()
                        .collect();
                }
            }
        }

        // Refresh.
        let names = self.fetch_all_table_names().await;
        *self.tables_cache.write().await = Some((Instant::now(), names.clone()));

        let prefix_lower = prefix.to_lowercase();
        names
            .into_iter()
            .filter(|n| prefix.is_empty() || n.to_lowercase().contains(&prefix_lower))
            .take(limit)
            .collect()
    }

    async fn columns(&self, table: &str) -> Vec<(String, String)> {
        {
            let cache = self.columns_cache.read().await;
            if let Some((fetched_at, cols)) = cache.get(table) {
                if fetched_at.elapsed() < self.ttl {
                    return cols.clone();
                }
            }
        }

        let cols = self.fetch_columns(table).await;
        self.columns_cache
            .write()
            .await
            .insert(table.to_string(), (Instant::now(), cols.clone()));
        cols
    }

    async fn columns_for(&self, tables: &[String]) -> Vec<(String, String)> {
        // Fetch sequentially to avoid spawning; provider cache makes repeat calls cheap.
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for t in tables {
            for (name, ty) in self.columns(t).await {
                if seen.insert(name.clone()) {
                    out.push((name, ty));
                }
            }
        }
        out
    }
}
