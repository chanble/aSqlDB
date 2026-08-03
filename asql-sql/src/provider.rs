use async_trait::async_trait;

/// Provider for lazy schema metadata used by the SQL completer.
///
/// Implementations are responsible for their own caching strategy.
/// All methods return owned data; failures should surface as empty results
/// rather than errors so the completer stays resilient to db hiccups.
#[async_trait]
pub trait SchemaProvider: Send + Sync {
    /// Return up to `limit` table names that contain `prefix`
    /// (case-insensitive). Empty `prefix` means "all tables up to limit".
    async fn table_names(&self, prefix: &str, limit: usize) -> Vec<String>;

    /// Return the columns of a single table as `(name, data_type)` pairs.
    async fn columns(&self, table: &str) -> Vec<(String, String)>;

    /// Return the combined columns of several tables. Useful when the SQL
    /// statement already mentions one or more tables and we want columns
    /// from all of them. Implementations may fetch concurrently.
    async fn columns_for(&self, tables: &[String]) -> Vec<(String, String)>;
}
