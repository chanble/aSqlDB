use crate::dialect::Dialect;

/// How to match a table name filter in introspection queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableNameMatch {
    /// Exact equality match: `TABLE_NAME = 'name'`.
    Exact,
    /// Prefix match: `TABLE_NAME LIKE 'name%'`.
    StartsWith,
    /// Substring match: `TABLE_NAME LIKE '%name%'`.
    Contains,
}

/// Builder for table introspection queries across supported dialects
pub struct TablesIntrospection;

impl TablesIntrospection {
    /// 根据 database 参数生成 MySQL 的 table_schema 条件。
    fn schema_condition_mysql(database: Option<&str>) -> String {
        match database {
            Some(db) => {
                let escaped = db.replace('\'', "''");
                format!("table_schema = '{}'", escaped)
            }
            None => "table_schema = DATABASE()".to_string(),
        }
    }

    /// 根据 database 参数生成 PostgreSQL 的 schemaname 条件。
    fn schema_condition_pg(database: Option<&str>) -> String {
        match database {
            Some(schema) => {
                let escaped = schema.replace('\'', "''");
                format!("schemaname = '{}'", escaped)
            }
            None => "schemaname = 'public'".to_string(),
        }
    }

    fn like_pattern(name: &str, mode: TableNameMatch) -> String {
        let escaped = name.replace('\'', "''");
        match mode {
            TableNameMatch::Exact => escaped,
            TableNameMatch::StartsWith => format!("{}%", escaped),
            TableNameMatch::Contains => format!("%{}%", escaped),
        }
    }

    /// Generate a query to list tables with optional database/schema and name filtering
    pub fn list_tables(
        dialect: &dyn Dialect,
        database: Option<&str>,
        table_name: Option<&str>,
        match_mode: TableNameMatch,
    ) -> String {
        let name_condition = match table_name {
            None => String::new(),
            Some(name) if match_mode == TableNameMatch::Exact => {
                let escaped = name.replace('\'', "''");
                match dialect.name() {
                    "MySQL" => format!(" AND TABLE_NAME = '{}'", escaped),
                    "PostgreSQL" => format!(" AND tablename = '{}'", escaped),
                    "SQLite" => format!(" AND name = '{}'", escaped),
                    _ => unreachable!(),
                }
            }
            Some(name) => {
                let pattern = Self::like_pattern(name, match_mode);
                match dialect.name() {
                    "MySQL" => format!(" AND TABLE_NAME LIKE '{}'", pattern),
                    "PostgreSQL" => format!(" AND tablename LIKE '{}'", pattern),
                    "SQLite" => format!(" AND name LIKE '{}'", pattern),
                    _ => unreachable!(),
                }
            }
        };

        match dialect.name() {
            "MySQL" => {
                let schema = Self::schema_condition_mysql(database);
                format!(
                    "SELECT TABLE_NAME, TABLE_COMMENT, ENGINE, TABLE_COLLATION, \
                     TABLE_ROWS, DATA_LENGTH, INDEX_LENGTH, DATA_FREE, AUTO_INCREMENT, \
                     (DATA_LENGTH + INDEX_LENGTH) AS TABLE_SIZE \
                     FROM information_schema.tables \
                     WHERE {schema}{name_condition} \
                     ORDER BY TABLE_NAME"
                )
            }
            "PostgreSQL" => {
                let schema = Self::schema_condition_pg(database);
                format!(
                    "SELECT tablename AS TABLE_NAME, \
                     obj_description((quote_ident(tablename)::regclass)::oid, 'pg_class') AS TABLE_COMMENT, \
                     '' AS ENGINE, '' AS TABLE_COLLATION, \
                     GREATEST((SELECT reltuples FROM pg_catalog.pg_class \
                               WHERE relname = tablename LIMIT 1), 0)::bigint AS TABLE_ROWS, \
                     COALESCE(pg_relation_size(quote_ident(tablename)::regclass), 0) AS DATA_LENGTH, \
                     COALESCE(pg_indexes_size(quote_ident(tablename)::regclass), 0) AS INDEX_LENGTH, \
                     0 AS DATA_FREE, \
                     NULL AS AUTO_INCREMENT, \
                     COALESCE(pg_total_relation_size(quote_ident(tablename)::regclass), 0) AS TABLE_SIZE \
                     FROM pg_tables WHERE {schema}{name_condition} \
                     ORDER BY tablename"
                )
            }
            "SQLite" => {
                format!(
                    "SELECT name AS TABLE_NAME, '' AS TABLE_COMMENT, '' AS ENGINE, \
                     '' AS TABLE_COLLATION, 0 AS TABLE_ROWS, \
                     0 AS DATA_LENGTH, 0 AS INDEX_LENGTH, 0 AS DATA_FREE, \
                     NULL AS AUTO_INCREMENT, 0 AS TABLE_SIZE \
                     FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'{name_condition} \
                     ORDER BY name"
                )
            }
            _ => unreachable!(),
        }
    }

    /// Generate a query to get metadata for a single table
    pub fn table_info(dialect: &dyn Dialect, database: Option<&str>, table: &str) -> String {
        Self::list_tables(dialect, database, Some(table), TableNameMatch::Exact)
    }

    /// Generate a query to count the number of tables in a database
    pub fn table_count(dialect: &dyn Dialect, database: Option<&str>) -> String {
        match dialect.name() {
            "MySQL" => {
                let schema = Self::schema_condition_mysql(database);
                format!(
                    "SELECT COUNT(*) AS count FROM information_schema.tables WHERE {schema}"
                )
            }
            "PostgreSQL" => {
                let schema = Self::schema_condition_pg(database);
                format!("SELECT COUNT(*) AS count FROM pg_tables WHERE {schema}")
            }
            "SQLite" => {
                "SELECT COUNT(*) AS count FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
                    .to_string()
            }
            _ => unreachable!(),
        }
    }

    /// Generate a query to retrieve storage sizes for all tables
    pub fn table_sizes(dialect: &dyn Dialect, database: Option<&str>) -> String {
        match dialect.name() {
            "MySQL" => {
                let schema = Self::schema_condition_mysql(database);
                format!(
                    "SELECT TABLE_NAME, \
                     DATA_LENGTH + INDEX_LENGTH AS size_bytes \
                     FROM information_schema.tables \
                     WHERE {schema} \
                     ORDER BY size_bytes DESC"
                )
            }
            "PostgreSQL" => "SELECT relname AS TABLE_NAME, \
                 pg_total_relation_size(relid) AS size_bytes \
                 FROM pg_catalog.pg_statio_user_tables \
                 ORDER BY size_bytes DESC"
                .to_string(),
            "SQLite" => "SELECT name AS TABLE_NAME, pgsize AS size_bytes \
                 FROM dbstat WHERE type='table' \
                 ORDER BY pgsize DESC"
                .to_string(),
            _ => unreachable!(),
        }
    }
}
