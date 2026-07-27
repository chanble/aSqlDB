use indexmap::IndexMap;
use sqlx::{AssertSqlSafe, Column, Row, TypeInfo};

use crate::result::{DbError, DbRow, ModifyResult, SchemaResult, SetResult};

pub(crate) async fn execute_select_mysql(
    sql: &str,
    conn: &mut sqlx::MySqlConnection,
) -> Result<SetResult, DbError> {
    let rows = sqlx::query(AssertSqlSafe(sql))
        .persistent(false)
        .fetch_all(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    let result_rows = extract_rows_mysql(&rows);
    Ok(SetResult {
        rows: result_rows,
    })
}

pub(crate) async fn execute_modify_mysql(
    sql: &str,
    conn: &mut sqlx::MySqlConnection,
) -> Result<ModifyResult, DbError> {
    let r = sqlx::query(AssertSqlSafe(sql))
        .persistent(false)
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(ModifyResult {
        rows_affected: r.rows_affected(),
        last_insert_id: Some(r.last_insert_id() as i64),
    })
}

pub(crate) async fn execute_generic_mysql(
    sql: &str,
    conn: &mut sqlx::MySqlConnection,
) -> Result<SchemaResult, DbError> {
    sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(SchemaResult {
        message: "Operation succeeded".to_string(),
    })
}

pub(crate) fn get_column_value_mysql(row: &sqlx::mysql::MySqlRow, idx: usize) -> serde_json::Value {
    let type_name = row.columns()[idx].type_info().name();

    let result = match type_name {
        "BOOLEAN" => super::decode_col::<u8, _>(row, idx)
            .or_else(|| super::decode_col::<i8, _>(row, idx)),
        "TINYINT UNSIGNED" => super::decode_col::<u8, _>(row, idx),
        "TINYINT" => super::decode_col::<i8, _>(row, idx),
        "SMALLINT UNSIGNED" => super::decode_col::<u16, _>(row, idx),
        "SMALLINT" => super::decode_col::<i16, _>(row, idx),
        "MEDIUMINT UNSIGNED" | "INT UNSIGNED" => super::decode_col::<u32, _>(row, idx),
        "MEDIUMINT" | "INT" => super::decode_col::<i32, _>(row, idx),
        "BIGINT UNSIGNED" => super::decode_col::<u64, _>(row, idx),
        "BIGINT" => super::decode_col::<i64, _>(row, idx),
        "FLOAT" => super::decode_col::<f32, _>(row, idx),
        "DOUBLE" => super::decode_col::<f64, _>(row, idx),
        "BIT" => super::decode_col::<u64, _>(row, idx),
        "YEAR" => super::decode_col::<i64, _>(row, idx),
        "DATE" => super::decode_col::<chrono::NaiveDate, _>(row, idx),
        "TIME" => super::decode_col::<chrono::NaiveTime, _>(row, idx),
        "DATETIME" => super::decode_col::<chrono::NaiveDateTime, _>(row, idx),
        "TIMESTAMP" => super::decode_col::<chrono::DateTime<chrono::Utc>, _>(row, idx),
        "JSON" => super::decode_col::<serde_json::Value, _>(row, idx),
        "DECIMAL" => super::decode_col::<rust_decimal::Decimal, _>(row, idx)
            .or_else(|| super::decode_col::<f64, _>(row, idx))
            .or_else(|| super::decode_col::<String, _>(row, idx)),
        "VARBINARY" | "BINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => {
            super::decode_col::<String, _>(row, idx).or_else(|| decode_col_bytes_mysql(row, idx))
        }
        "NULL" => Some(serde_json::Value::Null),
        _ => super::decode_col::<String, _>(row, idx),
    };

    result.unwrap_or_else(|| serde_json::json!(format!("<{}>", type_name)))
}

fn decode_col_bytes_mysql(row: &sqlx::mysql::MySqlRow, idx: usize) -> Option<serde_json::Value> {
    match row.try_get::<Option<Vec<u8>>, _>(idx) {
        Ok(Some(v)) => Some(serde_json::json!(String::from_utf8_lossy(&v).to_string())),
        Ok(None) => Some(serde_json::Value::Null),
        Err(_) => None,
    }
}

pub(crate) fn extract_rows_mysql(rows: &[sqlx::mysql::MySqlRow]) -> Vec<DbRow> {
    let mut result_rows: Vec<DbRow> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = IndexMap::new();
        for (i, column) in row.columns().iter().enumerate() {
            map.insert(column.name().to_string(), get_column_value_mysql(row, i));
        }
        result_rows.push(map);
    }
    result_rows
}
