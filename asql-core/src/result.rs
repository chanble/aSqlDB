use serde::Serialize;
use serde_json::Value as JsonValue;
use indexmap::IndexMap;

#[derive(Debug, Serialize)]
pub struct ExecutionResult<T> {
    pub sql: String,
    pub duration_ms: u64,
    pub data: T,
}

#[derive(Debug)]
pub enum DbError {
    SqlExecutionError {
        sql: String,
        source: sqlx::Error,
    },
    ConnectionError(String),
    Timeout,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::SqlExecutionError { sql, source } => {
                write!(f, "SQL execution failed: {} | Reason: {}", sql, source)
            }
            DbError::ConnectionError(detail) => {
                write!(f, "Connection error: {}", detail)
            }
            DbError::Timeout => {
                write!(f, "SQL execution timeout")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::SqlExecutionError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl DbError {
    pub fn get_sql(&self) -> &str {
        match self {
            DbError::SqlExecutionError { sql, .. } => sql,
            DbError::ConnectionError(_) => "",
            DbError::Timeout { .. } => "",
        }
    }
}

#[derive(Debug, Serialize)]
pub enum DbSuccessResult {
    Select(ExecutionResult<SetResult>),
    Modify(ExecutionResult<ModifyResult>),
    Schema(ExecutionResult<SchemaResult>),
}

impl DbSuccessResult {
    pub fn get_sql(&self) -> &str {
        match self {
            DbSuccessResult::Select(r) => &r.sql,
            DbSuccessResult::Modify(r) => &r.sql,
            DbSuccessResult::Schema(r) => &r.sql,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            DbSuccessResult::Select(_) => "SELECT",
            DbSuccessResult::Modify(_) => "MODIFY",
            DbSuccessResult::Schema(_) => "SCHEMA",
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match self {
            DbSuccessResult::Select(r) => r.duration_ms,
            DbSuccessResult::Modify(r) => r.duration_ms,
            DbSuccessResult::Schema(r) => r.duration_ms,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SetResult {
    pub rows: Vec<DbRow>,
}

// 使用 serde_json::Value 可以保留数字、布尔值、字符串、Null 的区别
pub type DbRow = IndexMap<String, JsonValue>;

#[derive(Debug, Serialize)]
pub struct ModifyResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SchemaResult {
    pub message: String,
}
