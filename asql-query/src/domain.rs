use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub collation: String,
}

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub table_name: String,
    pub table_comment: Option<String>,
    pub engine: Option<String>,
    pub table_collation: Option<String>,
    pub table_rows: u64,
    pub table_size: u64,
    pub data_length: Option<u64>,
    pub index_length: Option<u64>,
    pub data_free: Option<u64>,
    pub auto_increment: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TableSize {
    pub table_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct IndexDetail {
    pub key_name: String,
    pub column_name: String,
    pub non_unique: bool,
    pub seq_in_index: i64,
    pub index_type: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user: String,
    pub host: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub id: i64,
    pub user: String,
    pub host: String,
    pub db: Option<String>,
    pub command: String,
    pub time: i64,
    pub state: Option<String>,
    pub info: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VariableInfo {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct DdlSummary {}

#[derive(Debug, Serialize)]
pub struct ModifySummary {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}
