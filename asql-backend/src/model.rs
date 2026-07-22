use serde::Serialize;

/// 数据库信息（名称+字符集）
#[derive(Debug, Serialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub collation: String,
}
