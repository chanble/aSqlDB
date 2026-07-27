use indexmap::IndexMap;
use sqlx::{AssertSqlSafe, Column, Row, TypeInfo};

use crate::result::{DbError, DbRow, ModifyResult, SchemaResult, SetResult};

pub(crate) async fn execute_select_pg(
    sql: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<SetResult, DbError> {
    let rows = sqlx::query(AssertSqlSafe(sql))
        .persistent(false)
        .fetch_all(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    let result_rows = extract_rows_pg(&rows);
    Ok(SetResult {
        rows: result_rows,
    })
}

pub(crate) async fn execute_modify_pg(
    sql: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<ModifyResult, DbError> {
    let r = sqlx::query(AssertSqlSafe(sql))
        .persistent(false)
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(ModifyResult {
        rows_affected: r.rows_affected(),
        last_insert_id: None,
    })
}

pub(crate) async fn execute_generic_pg(
    sql: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<SchemaResult, DbError> {
    sqlx::query(AssertSqlSafe(sql))
        .persistent(false)
        .execute(conn)
        .await
        .map_err(|e| crate::make_error!(sql, e))?;
    Ok(SchemaResult {
        message: "Operation succeeded".to_string(),
    })
}

pub(crate) fn get_column_value_pg(row: &sqlx::postgres::PgRow, idx: usize) -> serde_json::Value {
    let type_name = row.columns()[idx].type_info().name();

    let result = match type_name {
        "BOOL" => super::decode_col::<bool, _>(row, idx),
        "INT2" => super::decode_col::<i16, _>(row, idx),
        "INT4" => super::decode_col::<i32, _>(row, idx),
        "INT8" => super::decode_col::<i64, _>(row, idx),
        "OID" => super::decode_col::<i32, _>(row, idx),
        "FLOAT4" => super::decode_col::<f32, _>(row, idx),
        "FLOAT8" => super::decode_col::<f64, _>(row, idx),
        "NUMERIC" => super::decode_col::<i64, _>(row, idx)
            .or_else(|| super::decode_col::<f64, _>(row, idx))
            .or_else(|| super::decode_col::<String, _>(row, idx)),
        "DATE" => super::decode_col::<chrono::NaiveDate, _>(row, idx),
        "TIME" => super::decode_col::<chrono::NaiveTime, _>(row, idx),
        "TIMESTAMP" => super::decode_col::<chrono::NaiveDateTime, _>(row, idx),
        "TIMESTAMPTZ" => super::decode_col::<chrono::DateTime<chrono::Utc>, _>(row, idx),
        "JSON" | "JSONB" => super::decode_col::<serde_json::Value, _>(row, idx),
        "UUID" => super::decode_col::<String, _>(row, idx),
        "BYTEA" => super::decode_col::<String, _>(row, idx)
            .or_else(|| super::decode_col::<Vec<u8>, _>(row, idx)),
        "VOID" => Some(serde_json::Value::Null),
        _ => super::decode_col::<String, _>(row, idx),
    };

    result.unwrap_or_else(|| serde_json::json!(format!("<{}>", type_name)))
}

pub(crate) fn extract_rows_pg(rows: &[sqlx::postgres::PgRow]) -> Vec<DbRow> {
    let mut result_rows: Vec<DbRow> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = IndexMap::new();
        for (i, column) in row.columns().iter().enumerate() {
            map.insert(column.name().to_string(), get_column_value_pg(row, i));
        }
        result_rows.push(map);
    }
    result_rows
}
