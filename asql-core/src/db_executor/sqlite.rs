use indexmap::IndexMap;
use sqlx::{Column, Row, TypeInfo};

use crate::result::{DbError, DbRow, ModifyResult, SchemaResult, SetResult};

pub(crate) async fn execute_select_sqlite(
    sql: &str,
    conn: &mut sqlx::SqliteConnection,
) -> Result<SetResult, DbError> {
    let rows = sqlx::query(sql)
        .persistent(false)
        .fetch_all(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    let result_rows = extract_rows_sqlite(&rows);
    Ok(SetResult {
        rows: result_rows,
    })
}

pub(crate) async fn execute_modify_sqlite(
    sql: &str,
    conn: &mut sqlx::SqliteConnection,
) -> Result<ModifyResult, DbError> {
    let r = sqlx::query(sql)
        .persistent(false)
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(ModifyResult {
        rows_affected: r.rows_affected(),
        last_insert_id: Some(r.last_insert_rowid()),
    })
}

pub(crate) async fn execute_generic_sqlite(
    sql: &str,
    conn: &mut sqlx::SqliteConnection,
) -> Result<SchemaResult, DbError> {
    sqlx::query(sql)
        .persistent(false)
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(SchemaResult {
        message: "Operation succeeded".to_string(),
    })
}

pub(crate) fn get_column_value_sqlite(
    row: &sqlx::sqlite::SqliteRow,
    idx: usize,
) -> serde_json::Value {
    let type_name = row.columns()[idx].type_info().name();

    let result = match type_name {
        "BOOLEAN" => super::decode_col::<bool, _>(row, idx),
        "INTEGER" => super::decode_col::<i64, _>(row, idx),
        "REAL" => super::decode_col::<f64, _>(row, idx),
        "TEXT" | "BLOB" => super::decode_col::<String, _>(row, idx),
        "DATE" => super::decode_col::<chrono::NaiveDate, _>(row, idx),
        "TIME" => super::decode_col::<chrono::NaiveTime, _>(row, idx),
        "DATETIME" => super::decode_col::<chrono::NaiveDateTime, _>(row, idx),
        "NULL" => Some(serde_json::Value::Null),
        _ => super::decode_col::<i64, _>(row, idx)
            .or_else(|| super::decode_col::<f64, _>(row, idx))
            .or_else(|| super::decode_col::<String, _>(row, idx)),
    };

    result.unwrap_or_else(|| serde_json::json!(format!("<{}>", type_name)))
}

pub(crate) fn extract_rows_sqlite(rows: &[sqlx::sqlite::SqliteRow]) -> Vec<DbRow> {
    let mut result_rows: Vec<DbRow> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = IndexMap::new();
        for (i, column) in row.columns().iter().enumerate() {
            map.insert(column.name().to_string(), get_column_value_sqlite(row, i));
        }
        result_rows.push(map);
    }
    result_rows
}
