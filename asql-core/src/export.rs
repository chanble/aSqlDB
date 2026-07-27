use std::sync::Arc;

use futures_util::StreamExt;
use sqlx::{AssertSqlSafe, Executor, Row};
use tokio::sync::mpsc;

use crate::db_executor::{get_column_value_mysql, get_column_value_pg, get_column_value_sqlite};
use crate::db_manager::Pool;
use crate::result::DbError;

/// Receiver type for streaming export data.
pub type ExportReceiver = mpsc::Receiver<String>;

/// 导出选项
#[derive(Clone)]
pub struct ExportConfig {
    pub database: Option<String>,
    pub db_option: DatabaseOption,
    pub tables: TableTarget,
    pub table_option: TableOption,
    pub data_format: DataFormat,
}

/// 导出时数据库选项
#[derive(Clone, Copy)]
pub enum DatabaseOption {
    Skip,
    /// 在导出的数据前加上 USE 语句
    Use,
    /// 在导出的数据前加上创建数据库的语句
    Create,
    /// 在导出的数据前加上删除然后创建数据库的语句
    DropCreate,
}

/// 表定义
#[derive(Clone)]
pub struct TableDef {
    pub name: String,
    /// 列
    pub columns: ColumnTarget,
    /// WHERE 的 SQL 片段（不含 WHERE 关键字）
    pub filter_sql: String,
    /// ORDER BY 的 SQL 片段（不含 ORDER BY 关键字）
    pub order_by: String,
    /// SQL格式: 是否输出 CREATE TABLE；非SQL格式: 是否输出表头
    pub ddl: bool,
    /// 是否导出数据
    pub data: bool,
}

/// 列选择
#[derive(Clone)]
pub enum ColumnTarget {
    All,
    Selected(Vec<String>),
}

/// 表选择
#[derive(Clone)]
pub enum TableTarget {
    /// 库中所有表
    All,
    /// 指定的表列表
    Selected(Vec<TableDef>),
}

/// 表选项
#[derive(Clone, Copy)]
pub enum TableOption {
    /// 不包含建表语句
    Skip,
    /// 包含建表语句
    Create,
    /// 包含删除然后建表语句
    DropCreate,
}

/// 数据格式
#[derive(Clone, Copy)]
pub enum DataFormat {
    /// 不输出数据
    Skip,
    /// SQL INSERT
    Sql,
    /// CSV（逗号分隔）
    Csv,
    /// CSV（分号分隔）
    CsvSemicolon,
    /// TSV
    Tsv,
}

// ─── Helpers ────────────────────────────────────────────────────────

fn qualified_table(table: &str, database: &Option<String>) -> String {
    match database {
        Some(db) => format!("`{}`.`{}`", db, table),
        None => format!("`{}`", table),
    }
}

fn qualified_db(database: &str) -> String {
    format!("`{}`", database)
}

/// Stream query results as formatted lines (CSV, TSV, or SQL INSERT).
///
/// Uses a temporary connection (via `Pool`) so the shared connection is not blocked.
/// Spawns a background task that reads rows via cursor, formats them, and sends
/// formatted lines through the returned receiver.
pub async fn export_stream(
    pool: Arc<Pool>,
    export_config: &ExportConfig,
) -> Result<ExportReceiver, DbError> {
    let (tx, rx) = mpsc::channel::<String>(64);

    let database = export_config.database.clone();
    let db_option = export_config.db_option;
    let table_option = export_config.table_option;
    let data_format = export_config.data_format;
    let tables = export_config.tables.clone();

    tokio::spawn(async move {
        match &*pool {
            Pool::MySql(p) => {
                let mut conn = p.0.lock().await;

                if let Some(ref db) = database {
                    if let Err(e) = conn.execute(sqlx::raw_sql(AssertSqlSafe(format!("USE `{}`", db)))).await {
                        let msg = format!("-- Error: cannot switch to database `{}`: {}", db, e);
                        let _ = tx.send(msg).await;
                        return;
                    } else {
                        // if let Ok(row) =
                        //     sqlx::query("SELECT DATABASE()").fetch_one(&mut *conn).await
                        // {
                        //     if let Ok(db_name) = row.try_get::<String, _>(0) {
                        //         let _ = tx.send(format!("-- Current database: {}", db_name)).await;
                        //     }
                        // }
                    }
                }

                // DatabaseOption
                if matches!(data_format, DataFormat::Sql) {
                    if let Err(()) = emit_db_option_mysql(&tx, &database, db_option).await {
                        return;
                    }
                }

                // Resolve tables
                let table_defs = resolve_tables_mysql(&mut conn, &tables).await;

                for td in &table_defs {
                    // Resolve columns
                    let col_names = resolve_columns_mysql(&mut conn, td).await;

                    let col_strs: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();

                    // DDL
                    if td.ddl && matches!(data_format, DataFormat::Sql) {
                        match table_option {
                            TableOption::DropCreate => {
                                let sql = format!(
                                    "DROP TABLE IF EXISTS {};\n",
                                    qualified_table(&td.name, &database)
                                );
                                if tx.send(sql).await.is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                        match table_option {
                            TableOption::Create | TableOption::DropCreate => {
                                match get_create_table_mysql(&mut conn, &td.name, &database).await {
                                    Ok(create_sql) => {
                                        if tx.send(create_sql).await.is_err() {
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!(
                                            "-- Error: SHOW CREATE TABLE failed for `{}`: {}",
                                            td.name, e
                                        );
                                        let _ = tx.send(msg).await;
                                        return;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // Data
                    if td.data && !matches!(data_format, DataFormat::Skip) {
                        let sep = match data_format {
                            DataFormat::Csv => ",",
                            DataFormat::CsvSemicolon => ";",
                            DataFormat::Tsv => "\t",
                            _ => "",
                        };

                        // Header for non-SQL formats
                        if !matches!(data_format, DataFormat::Sql) && td.ddl {
                            if tx.send(col_names.join(sep) + "\n").await.is_err() {
                                return;
                            }
                        }

                        // Build SELECT
                        let select_sql = build_select_sql(
                            &col_strs,
                            &td.name,
                            &td.filter_sql,
                            &td.order_by,
                            &database,
                        );

                        let mut stream = sqlx::query(AssertSqlSafe(select_sql.as_str())).fetch(&mut *conn);
                        while let Some(row_result) = stream.next().await {
                            let row = match row_result {
                                Ok(r) => r,
                                Err(e) => {
                                    let _ = tx.send(format!("-- Error: {}", e)).await;
                                    return;
                                }
                            };
                            let mut vals = Vec::with_capacity(col_strs.len());
                            for i in 0..col_strs.len() {
                                let v = get_column_value_mysql(&row, i);
                                vals.push(value_to_string(&v));
                            }
                            let line = format_export_line(&vals, &col_strs, &td.name, &data_format);
                            if tx.send(line).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Pool::Postgres(p) => {
                let mut conn = p.0.lock().await;

                if let Some(ref db) = database {
                    if let Err(e) = conn
                        .execute(sqlx::raw_sql(AssertSqlSafe(format!("SET search_path TO \"{}\"", db))))
                        .await
                    {
                        let msg = format!("-- Error: cannot set search_path to \"{}\": {}", db, e);
                        let _ = tx.send(msg).await;
                        return;
                    } else {
                        // if let Ok(row) = sqlx::query("SELECT current_database()")
                        //     .fetch_one(&mut *conn)
                        //     .await
                        // {
                        //     if let Ok(db_name) = row.try_get::<String, _>(0) {
                        //         let _ = tx.send(format!("-- Current database: {}", db_name)).await;
                        //     }
                        // }
                    }
                }

                if matches!(data_format, DataFormat::Sql) {
                    if let Err(()) = emit_db_option_pg(&tx, &database, db_option).await {
                        return;
                    }
                }

                let table_defs = resolve_tables_pg(&mut conn, &tables).await;

                for td in &table_defs {
                    let col_names = resolve_columns_pg(&mut conn, td).await;

                    let col_strs: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();

                    if td.ddl && matches!(data_format, DataFormat::Sql) {
                        match table_option {
                            TableOption::DropCreate => {
                                let sql = format!(
                                    "DROP TABLE IF EXISTS {};\n",
                                    qualified_table(&td.name, &database)
                                );
                                if tx.send(sql).await.is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                        match table_option {
                            TableOption::Create | TableOption::DropCreate => {
                                if let Some(create_sql) =
                                    get_create_table_pg(&mut conn, &td.name, &database).await
                                {
                                    if tx.send(create_sql).await.is_err() {
                                        return;
                                    }
                                } else {
                                    let msg = format!(
                                        "-- Error: CREATE TABLE generation failed for `{}`",
                                        td.name
                                    );
                                    let _ = tx.send(msg).await;
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }

                    if td.data && !matches!(data_format, DataFormat::Skip) {
                        let sep = match data_format {
                            DataFormat::Csv => ",",
                            DataFormat::CsvSemicolon => ";",
                            DataFormat::Tsv => "\t",
                            _ => "",
                        };

                        if !matches!(data_format, DataFormat::Sql) && td.ddl {
                            if tx.send(col_names.join(sep) + "\n").await.is_err() {
                                return;
                            }
                        }

                        let select_sql = build_select_sql(
                            &col_strs,
                            &td.name,
                            &td.filter_sql,
                            &td.order_by,
                            &database,
                        );

                        let mut stream = sqlx::query(AssertSqlSafe(select_sql.as_str())).fetch(&mut *conn);
                        while let Some(row_result) = stream.next().await {
                            let row = match row_result {
                                Ok(r) => r,
                                Err(e) => {
                                    let _ = tx.send(format!("-- Error: {}", e)).await;
                                    return;
                                }
                            };
                            let mut vals = Vec::with_capacity(col_strs.len());
                            for i in 0..col_strs.len() {
                                let v = get_column_value_pg(&row, i);
                                vals.push(value_to_string(&v));
                            }
                            let line = format_export_line(&vals, &col_strs, &td.name, &data_format);
                            if tx.send(line).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Pool::Sqlite(p) => {
                let mut conn = p.0.lock().await;

                let table_defs = resolve_tables_sqlite(&mut conn, &tables).await;

                for td in &table_defs {
                    let col_names = resolve_columns_sqlite(&mut conn, td).await;

                    let col_strs: Vec<&str> = col_names.iter().map(|s| s.as_str()).collect();

                    if td.ddl && matches!(data_format, DataFormat::Sql) {
                        match table_option {
                            TableOption::DropCreate => {
                                let sql = format!(
                                    "DROP TABLE IF EXISTS {};\n",
                                    qualified_table(&td.name, &database)
                                );
                                if tx.send(sql).await.is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                        match table_option {
                            TableOption::Create | TableOption::DropCreate => {
                                if let Some(create_sql) =
                                    get_create_table_sqlite(&mut conn, &td.name, &database).await
                                {
                                    if tx.send(create_sql).await.is_err() {
                                        return;
                                    }
                                } else {
                                    let msg = format!(
                                        "-- Error: CREATE TABLE generation failed for `{}`",
                                        td.name
                                    );
                                    let _ = tx.send(msg).await;
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }

                    if td.data && !matches!(data_format, DataFormat::Skip) {
                        let sep = match data_format {
                            DataFormat::Csv => ",",
                            DataFormat::CsvSemicolon => ";",
                            DataFormat::Tsv => "\t",
                            _ => "",
                        };

                        if !matches!(data_format, DataFormat::Sql) && td.ddl {
                            if tx.send(col_names.join(sep) + "\n").await.is_err() {
                                return;
                            }
                        }

                        let select_sql = build_select_sql(
                            &col_strs,
                            &td.name,
                            &td.filter_sql,
                            &td.order_by,
                            &database,
                        );

                        let mut stream = sqlx::query(AssertSqlSafe(select_sql.as_str())).fetch(&mut *conn);
                        while let Some(row_result) = stream.next().await {
                            let row = match row_result {
                                Ok(r) => r,
                                Err(e) => {
                                    let _ = tx.send(format!("-- Error: {}", e)).await;
                                    return;
                                }
                            };
                            let mut vals = Vec::with_capacity(col_strs.len());
                            for i in 0..col_strs.len() {
                                let v = get_column_value_sqlite(&row, i);
                                vals.push(value_to_string(&v));
                            }
                            let line = format_export_line(&vals, &col_strs, &td.name, &data_format);
                            if tx.send(line).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        };
    });

    Ok(rx)
}

// ─── MySQL helpers ──────────────────────────────────────────────────

async fn emit_db_option_mysql(
    tx: &mpsc::Sender<String>,
    database: &Option<String>,
    option: DatabaseOption,
) -> Result<(), ()> {
    match (option, database) {
        (DatabaseOption::Use, Some(db)) => tx
            .send(format!("USE {};\n", qualified_db(db)))
            .await
            .map_err(|_| ()),
        (DatabaseOption::Create, Some(db)) => tx
            .send(format!(
                "CREATE DATABASE IF NOT EXISTS {};\n",
                qualified_db(db)
            ))
            .await
            .map_err(|_| ()),
        (DatabaseOption::DropCreate, Some(db)) => {
            tx.send(format!("DROP DATABASE IF EXISTS {};\n", qualified_db(db)))
                .await
                .map_err(|_| ())?;
            tx.send(format!("CREATE DATABASE {};", qualified_db(db)))
                .await
                .map_err(|_| ())
        }
        _ => Ok(()),
    }
}

async fn resolve_tables_mysql(
    conn: &mut sqlx::mysql::MySqlConnection,
    target: &TableTarget,
) -> Vec<TableDef> {
    match target {
        TableTarget::All => {
            let rows = sqlx::query("SHOW TABLES").fetch_all(conn).await;
            match rows {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| {
                        r.try_get::<String, _>(0).ok().map(|name| TableDef {
                            name,
                            columns: ColumnTarget::All,
                            filter_sql: String::new(),
                            order_by: String::new(),
                            ddl: true,
                            data: true,
                        })
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
        TableTarget::Selected(list) => list.clone(),
    }
}

async fn resolve_columns_mysql(
    conn: &mut sqlx::mysql::MySqlConnection,
    td: &TableDef,
) -> Vec<String> {
    match &td.columns {
        ColumnTarget::Selected(cols) => cols.clone(),
        ColumnTarget::All => {
            let sql = format!("SHOW COLUMNS FROM {}", qualified_table(&td.name, &None));
            match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_all(conn).await {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| r.try_get::<String, _>(0).ok())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
    }
}

async fn get_create_table_mysql(
    conn: &mut sqlx::mysql::MySqlConnection,
    table: &str,
    database: &Option<String>,
) -> Result<String, String> {
    let table_ref = qualified_table(table, database);
    let sql = format!("SHOW CREATE TABLE {}", table_ref);

    match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_one(conn).await {
        Ok(row) => row
            .try_get::<String, _>(1)
            .map(|s| format!("{};\n", s))
            .map_err(|e| format!("{}\n", e)),
        Err(e) => Err(format!("{}\n", e)),
    }
}

// ─── PostgreSQL helpers ─────────────────────────────────────────────

async fn emit_db_option_pg(
    tx: &mpsc::Sender<String>,
    database: &Option<String>,
    option: DatabaseOption,
) -> Result<(), ()> {
    match (option, database) {
        (DatabaseOption::Use, Some(db)) => tx.send(format!("\\c {}", db)).await.map_err(|_| ()),
        (DatabaseOption::Create, Some(db)) => tx
            .send(format!("CREATE DATABASE {};\n", db))
            .await
            .map_err(|_| ()),
        (DatabaseOption::DropCreate, Some(db)) => {
            tx.send(format!("DROP DATABASE IF EXISTS {};\n", db))
                .await
                .map_err(|_| ())?;
            tx.send(format!("CREATE DATABASE {};\n", db))
                .await
                .map_err(|_| ())
        }
        _ => Ok(()),
    }
}

async fn resolve_tables_pg(
    conn: &mut sqlx::postgres::PgConnection,
    target: &TableTarget,
) -> Vec<TableDef> {
    match target {
        TableTarget::All => {
            let sql = "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')";
            match sqlx::query(sql).fetch_all(conn).await {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| {
                        r.try_get::<String, _>(0).ok().map(|name| TableDef {
                            name,
                            columns: ColumnTarget::All,
                            filter_sql: String::new(),
                            order_by: String::new(),
                            ddl: true,
                            data: true,
                        })
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
        TableTarget::Selected(list) => list.clone(),
    }
}

async fn resolve_columns_pg(conn: &mut sqlx::postgres::PgConnection, td: &TableDef) -> Vec<String> {
    match &td.columns {
        ColumnTarget::Selected(cols) => cols.clone(),
        ColumnTarget::All => {
            let sql = format!(
                "SELECT column_name FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
                td.name.replace('\'', "''")
            );
            match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_all(conn).await {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| r.try_get::<String, _>(0).ok())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
    }
}

async fn get_create_table_pg(
    conn: &mut sqlx::postgres::PgConnection,
    table: &str,
    _database: &Option<String>,
) -> Option<String> {
    let sql = format!(
        "SELECT column_name || ' ' || data_type FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position",
        table.replace('\'', "''")
    );
    // For PG we generate a simplified CREATE TABLE
    match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_all(conn).await {
        Ok(rows) => {
            let cols: Vec<String> = rows
                .iter()
                .filter_map(|r| r.try_get::<String, _>(0).ok())
                .collect();
            if cols.is_empty() {
                None
            } else {
                Some(format!(
                    "CREATE TABLE {} (\n  {}\n);\n",
                    qualified_table(table, &None),
                    cols.join(",\n  ")
                ))
            }
        }
        Err(_) => None,
    }
}

// ─── SQLite helpers ─────────────────────────────────────────────────

async fn resolve_tables_sqlite(
    conn: &mut sqlx::sqlite::SqliteConnection,
    target: &TableTarget,
) -> Vec<TableDef> {
    match target {
        TableTarget::All => {
            match sqlx::query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(conn)
                .await
            {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| {
                        r.try_get::<String, _>(0).ok().map(|name| TableDef {
                            name,
                            columns: ColumnTarget::All,
                            filter_sql: String::new(),
                            order_by: String::new(),
                            ddl: true,
                            data: true,
                        })
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
        TableTarget::Selected(list) => list.clone(),
    }
}

async fn resolve_columns_sqlite(
    conn: &mut sqlx::sqlite::SqliteConnection,
    td: &TableDef,
) -> Vec<String> {
    match &td.columns {
        ColumnTarget::Selected(cols) => cols.clone(),
        ColumnTarget::All => {
            let sql = format!("PRAGMA table_info('{}')", td.name.replace('\'', "''"));
            match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_all(conn).await {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|r| r.try_get::<String, _>(1).ok())
                    .collect(),
                Err(_) => Vec::new(),
            }
        }
    }
}

async fn get_create_table_sqlite(
    conn: &mut sqlx::sqlite::SqliteConnection,
    table: &str,
    _database: &Option<String>,
) -> Option<String> {
    let sql = format!(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
        table.replace('\'', "''")
    );
    match sqlx::query(AssertSqlSafe(sql.as_str())).fetch_one(conn).await {
        Ok(row) => row
            .try_get::<String, _>(0)
            .ok()
            .map(|s| format!("{};\n", s)),
        Err(_) => None,
    }
}

// ─── Shared helpers ─────────────────────────────────────────────────

fn build_select_sql(
    columns: &[&str],
    table: &str,
    filter_sql: &str,
    order_sql: &str,
    database: &Option<String>,
) -> String {
    let cols = if columns.is_empty() {
        "*".to_string()
    } else {
        columns
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let table_ref = qualified_table(table, database);
    let mut sql = format!("SELECT {} FROM {}", cols, table_ref);
    if !filter_sql.is_empty() {
        sql.push_str(&format!(" WHERE {}", filter_sql));
    }
    if !order_sql.is_empty() {
        sql.push_str(&format!(" ORDER BY {}", order_sql));
    }
    sql
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            items.join(",")
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("{}:{}", k, value_to_string(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

fn format_export_line(
    vals: &[String],
    columns: &[&str],
    table: &str,
    format: &DataFormat,
) -> String {
    match format {
        DataFormat::Sql => {
            let cols: Vec<String> = columns.iter().map(|c| format!("`{}`", c)).collect();
            let table_ref = table;
            let vals_str: Vec<String> = vals
                .iter()
                .map(|v| {
                    if v == "NULL" {
                        "NULL".to_string()
                    } else {
                        format!("'{}'", v.replace('\'', "''"))
                    }
                })
                .collect();
            format!(
                "INSERT INTO {} ({}) VALUES ({});\n",
                table_ref,
                cols.join(", "),
                vals_str.join(", ")
            )
        }
        DataFormat::Csv => vals
            .iter()
            .map(|v| {
                if v == "NULL" {
                    v.clone()
                } else if v.contains(',') || v.contains('"') || v.contains('\n') {
                    format!("\"{}\"", v.replace('"', "\"\""))
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",")
            + "\n",
        DataFormat::CsvSemicolon => vals
            .iter()
            .map(|v| {
                if v == "NULL" {
                    v.clone()
                } else if v.contains(';') || v.contains('"') || v.contains('\n') {
                    format!("\"{}\"", v.replace('"', "\"\""))
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(";")
            + "\n",
        DataFormat::Tsv => vals
            .iter()
            .map(|v| {
                if v == "NULL" {
                    v.clone()
                } else {
                    v.replace('\t', "\\t")
                }
            })
            .collect::<Vec<_>>()
            .join("\t")
            + "\n",
        DataFormat::Skip => String::new(),
    }
}
