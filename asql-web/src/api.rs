use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use axum::extract::DefaultBodyLimit;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_stream::StreamExt;

use asql_backend::BackendHandle;
use asql_query::{
    get_suggestions, parse_column_type, split_sql_statements, AlterTableBuilder, ColumnDef,
    ColumnExtra, ColumnTarget, CreateTableBuilder, DataFormat, DatabaseOption, DatabaseType,
    DbManager, DbSchemaProvider, DeleteBuilder, Dialect, ExportBuilder, ExportTable, ExportTableDef,
    IndexType, InsertBuilder, MySql, OrderBy, PostgreSql, QueryBuilder, SelectBuilder, Sqlite,
    SuggestionKind, TableNameMatch, TableOption, UpdateBuilder, WhereBuilder,
};

type AppState = BackendHandle;

fn ok_json(v: Value) -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(v))
}

fn err_json(msg: impl ToString) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": msg.to_string()})),
    )
}

async fn create_qb(
    bk: &BackendHandle,
    conn: &str,
) -> Result<QueryBuilder, (StatusCode, Json<Value>)> {
    let dm = bk.db_manager().await;
    let url = dm
        .get_connection_url(conn)
        .await
        .ok_or_else(|| err_json(format!("Connection '{}' not found", conn)))?;
    let db_type = DatabaseType::from_url(&url);
    let dialect: Box<dyn Dialect + Send + Sync> = match db_type {
        DatabaseType::MySql => Box::new(MySql),
        DatabaseType::Postgres => Box::new(PostgreSql),
        DatabaseType::Sqlite => Box::new(Sqlite),
    };
    Ok(QueryBuilder::new_boxed(dialect, dm))
}

// ─── Path params ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct ConnPath {
    name: String,
}

#[derive(Deserialize)]
struct DbPath {
    name: String,
    db: String,
}

#[derive(Deserialize)]
struct TablePath {
    name: String,
    table: String,
}

#[derive(Deserialize)]
struct IndexPath {
    name: String,
    table: String,
    index: String,
}

#[derive(Deserialize)]
struct UserPath {
    name: String,
    username: String,
}

// ─── WebSocket completion ──────────────────────────────────────────

#[derive(Deserialize)]
struct CompletionRequestMsg {
    sql: String,
    cursor: usize,
    seq: u64,
}

// ─── Request bodies ────────────────────────────────────────────────

#[derive(Deserialize)]
struct QueryBody {
    sql: String,
    #[serde(default)]
    stop_on_error: bool,
}

#[derive(Deserialize)]
struct CreateDbBody {
    name: String,
    character_set: Option<String>,
    collation: Option<String>,
}

#[derive(Deserialize)]
struct AlterDbBody {
    character_set: Option<String>,
    collation: Option<String>,
}

#[derive(Deserialize)]
struct CreateTableBody {
    table: String,
    columns: Vec<ColumnDefBody>,
    engine: Option<String>,
    collation: Option<String>,
    comment: Option<String>,
}

#[derive(Deserialize)]
struct ColumnDefBody {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
    length: Option<String>,
    #[serde(default)]
    options: String,
    nullable: bool,
    auto_increment: bool,
    #[serde(default)]
    primary_key: bool,
    #[serde(rename = "default_value")]
    default: Option<String>,
    comment: Option<String>,
    #[serde(default)]
    after_column: Option<String>,
}

#[derive(Deserialize)]
struct AlterTableBody {
    rename_table: Option<String>,
    engine: Option<String>,
    collation: Option<String>,
    comment: Option<String>,
    #[serde(default)]
    add_columns: Vec<ColumnDefBody>,
    #[serde(default)]
    modify_columns: Vec<ColumnDefBody>,
    #[serde(default)]
    change_columns: Vec<ChangeColumnBody>,
    #[serde(default)]
    drop_columns: Vec<String>,
    #[serde(default)]
    add_indexes: Vec<AddIndexBody>,
    #[serde(default)]
    drop_indexes: Vec<String>,
}

#[derive(Deserialize)]
struct ChangeColumnBody {
    old_name: String,
    new_def: ColumnDefBody,
}

#[derive(Deserialize)]
struct AddIndexBody {
    name: String,
    index_type: String,
    columns: Vec<String>,
}

#[derive(Deserialize)]
struct CountBody {
    where_conditions: Vec<WhereConditionBody>,
}

#[derive(Deserialize)]
struct SelectBody {
    columns: Vec<SelectColumnBody>,
    where_conditions: Vec<WhereConditionBody>,
    order_by: Vec<OrderByBody>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct SelectColumnBody {
    name: String,
    func: Option<String>,
}

#[derive(Deserialize)]
struct WhereConditionBody {
    column: String,
    operator: String,
    value: String,
}

#[derive(Deserialize)]
struct OrderByBody {
    column: String,
    desc: bool,
}

#[derive(Deserialize)]
struct ExportBody {
    method: String,
    format: String,
    columns: Vec<String>,
    where_conditions: Vec<WhereConditionBody>,
    order_by: Vec<OrderByBody>,
    database: Option<String>,
}

#[derive(Deserialize)]
struct ExportRequestBody {
    method: String,
    database: Option<String>,
    #[serde(default)]
    db_option: String,
    #[serde(default)]
    tables_all: bool,
    #[serde(default)]
    tables: Vec<ApiTableDef>,
    #[serde(default)]
    table_option: String,
    data_format: String,
}

#[derive(Deserialize)]
struct ApiTableDef {
    name: String,
    #[serde(default)]
    columns: serde_json::Value,
    #[serde(default)]
    where_conditions: Vec<WhereConditionBody>,
    #[serde(default)]
    order_by: Vec<OrderByBody>,
    #[serde(default = "default_true")]
    ddl: bool,
    #[serde(default = "default_true")]
    data: bool,
}

fn default_true() -> bool {
    true
}

fn parse_db_option(s: &str) -> Result<DatabaseOption, String> {
    match s {
        "skip" => Ok(DatabaseOption::Skip),
        "use" => Ok(DatabaseOption::Use),
        "create" => Ok(DatabaseOption::Create),
        "drop_create" => Ok(DatabaseOption::DropCreate),
        _ => Err(format!("Invalid db_option: {}", s)),
    }
}

fn parse_table_option(s: &str) -> Result<TableOption, String> {
    match s {
        "skip" => Ok(TableOption::Skip),
        "create" => Ok(TableOption::Create),
        "drop_create" => Ok(TableOption::DropCreate),
        _ => Err(format!("Invalid table_option: {}", s)),
    }
}

fn parse_data_format(s: &str) -> Result<DataFormat, String> {
    match s {
        "skip" => Ok(DataFormat::Skip),
        "sql" => Ok(DataFormat::Sql),
        "csv" => Ok(DataFormat::Csv),
        "csv;" => Ok(DataFormat::CsvSemicolon),
        "tsv" => Ok(DataFormat::Tsv),
        _ => Err(format!("Invalid data_format: {}", s)),
    }
}

fn parse_column_target(v: &serde_json::Value) -> Result<ColumnTarget, String> {
    match v {
        serde_json::Value::String(s) if s == "all" => Ok(ColumnTarget::All),
        serde_json::Value::Array(arr) => {
            let cols: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if cols.is_empty() {
                Ok(ColumnTarget::All)
            } else {
                Ok(ColumnTarget::Selected(cols))
            }
        }
        serde_json::Value::Null => Ok(ColumnTarget::All),
        _ => Err("columns must be \"all\" or an array of strings".to_string()),
    }
}

#[derive(Deserialize)]
struct InsertBody {
    columns: Vec<String>,
    values: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct UpdateBody {
    sets: Vec<UpdateSetBody>,
    where_conditions: Vec<WhereConditionBody>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct UpdateSetBody {
    column: String,
    value: String,
}

#[derive(Deserialize)]
struct DeleteBody {
    where_conditions: Vec<WhereConditionBody>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct CreateIndexBody {
    name: String,
    index_type: String,
    columns: Vec<IndexColumnBody>,
    method: Option<String>,
}

#[derive(Deserialize)]
struct IndexColumnBody {
    name: String,
    prefix_len: Option<usize>,
}

#[derive(Deserialize)]
struct CreateUserBody {
    username: String,
    password: Option<String>,
    host: Option<String>,
}

#[derive(Deserialize)]
struct AlterUserBody {
    password: String,
    host: Option<String>,
}

#[derive(Deserialize)]
struct DropUserBody {
    host: Option<String>,
}

#[derive(Deserialize)]
struct RenameUserBody {
    new_username: String,
    new_host: Option<String>,
}

#[derive(Deserialize)]
struct GrantBody {
    privileges: Vec<String>,
    on: String,
    host: Option<String>,
    with_grant_option: bool,
}

#[derive(Deserialize)]
struct RevokeBody {
    privileges: Vec<String>,
    on: String,
    host: Option<String>,
}

#[derive(Deserialize)]
struct KillBody {
    pids: Vec<String>,
}

#[derive(Deserialize)]
struct UseDatabaseBody {
    database: String,
}

#[derive(Deserialize)]
struct TablesBody {
    tables: Vec<String>,
}

#[derive(Deserialize)]
struct ImportPreviewBody {
    file_path: String,
}

#[derive(Deserialize)]
struct ImportExecuteBody {
    file_path: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    stop_on_error: bool,
    #[serde(default)]
    database: Option<String>,
    #[serde(default)]
    single_transaction: bool,
}

#[derive(Deserialize)]
struct ImportTaskPath {
    task_id: String,
}

// ─── Import tasks (background execution + polling) ──────────────────

#[derive(Clone, Serialize)]
struct ImportTask {
    id: String,
    status: String, // "running" | "completed" | "failed" | "cancelled"
    total: usize,
    current: usize,
    succeeded: usize,
    failed: usize,
    errors: Vec<Value>,
    duration_ms: u64,
    created_at: u64,
    finished_at: Option<u64>,
    error: Option<String>,
    cancel_requested: bool,
    connection: String,
    database: Option<String>,
    file_name: String,
    file_path: String,
    total_lines: usize,
    file_size: usize,
    preview_head: String,
    preview_tail: String,
    preview_omitted: usize,
    stop_on_error: bool,
    single_transaction: bool,
}

fn import_tasks() -> &'static Mutex<HashMap<String, ImportTask>> {
    static IMPORT_TASKS: OnceLock<Mutex<HashMap<String, ImportTask>>> = OnceLock::new();
    IMPORT_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

static IMPORT_TASK_COUNTER: AtomicU64 = AtomicU64::new(0);

fn update_import_task(
    tasks: &Mutex<HashMap<String, ImportTask>>,
    id: &str,
    f: impl FnOnce(&mut ImportTask),
) {
    if let Ok(mut guard) = tasks.lock() {
        if let Some(task) = guard.get_mut(id) {
            f(task);
        }
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn import_cancel_requested(tasks: &Mutex<HashMap<String, ImportTask>>, id: &str) -> bool {
    tasks
        .lock()
        .map(|g| g.get(id).map(|t| t.cancel_requested).unwrap_or(false))
        .unwrap_or(false)
}

// ─── Query params ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct TableQuery {
    database: Option<String>,
}

#[derive(Deserialize)]
struct UserQuery {
    host: Option<String>,
}

#[derive(Deserialize)]
struct ListTablesQuery {
    table_name: Option<String>,
    #[serde(default)]
    match_mode: Option<String>,
}

// ─── Helpers ───────────────────────────────────────────────────────

fn column_def(b: &ColumnDefBody) -> ColumnDef {
    let col_type = {
        let s = match &b.length {
            Some(len) => format!("{}({})", b.col_type, len),
            None => b.col_type.clone(),
        };
        let s = if b.options.contains("unsigned") || b.options.contains("zerofill") {
            let suffix = if b.options.contains("zerofill") {
                " zerofill"
            } else if b.options.contains("unsigned") {
                " unsigned"
            } else {
                ""
            };
            format!("{}{}", s, suffix)
        } else {
            s
        };
        parse_column_type(&s)
    };
    ColumnDef {
        name: b.name.clone(),
        col_type,
        nullable: Some(b.nullable),
        default_value: b.default.clone(),
        comment: b.comment.clone(),
        extra: ColumnExtra {
            auto_increment: b.auto_increment,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn index_type(s: &str) -> IndexType {
    match s.to_uppercase().as_str() {
        "UNIQUE" => IndexType::Unique,
        "FULLTEXT" => IndexType::Fulltext,
        "SPATIAL" => IndexType::Spatial,
        "PRIMARY" => IndexType::Primary,
        _ => IndexType::Index,
    }
}

fn select_builder(table: &str, body: &SelectBody) -> SelectBuilder {
    let mut sb = SelectBuilder::new().from(table);
    for c in &body.columns {
        sb = match &c.func {
            Some(f) => sb.column_with_func(f, &c.name),
            None => sb.column(&c.name),
        };
    }
    for w in &body.where_conditions {
        sb = sb.and_where(&w.column, &w.operator, &w.value);
    }
    for o in &body.order_by {
        sb = sb.order_by(&o.column, o.desc);
    }
    if let Some(l) = body.limit {
        sb = sb.limit(l);
    }
    if let Some(o) = body.offset {
        sb = sb.offset(o);
    }
    sb
}

// ═══════════════════════════════════════════════════════════════════
//  Router
// ═══════════════════════════════════════════════════════════════════

pub fn build_router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/health", get(health_check))
        .route("/systems", get(list_systems))
        .route("/connections", get(list_connections).post(add_connection))
        .route("/connections/{name}", delete(remove_connection))
        .route("/connections/{name}/test", post(test_connection))
        .route("/connections/{name}/query", post(execute_query))
        .route(
            "/connections/{name}/databases",
            get(list_databases).post(create_database),
        )
        .route(
            "/connections/{name}/databases/{db}",
            delete(drop_database).patch(alter_database),
        )
        .route(
            "/connections/{name}/current-database",
            get(current_database),
        )
        .route(
            "/connections/{name}/use-database",
            post(use_database_handler),
        )
        .route(
            "/connections/{name}/column-types",
            get(column_types_handler),
        )
        .route("/connections/{name}/charsets", get(charsets_handler))
        .route("/connections/{name}/functions", get(functions_handler))
        .route("/connections/{name}/export", post(export_data_handler))
        .route("/connections/{name}/import/preview", post(import_preview))
        .route("/connections/{name}/import", post(import_execute))
        .route("/import/tasks", get(import_tasks_list))
        .route("/import/tasks/{task_id}", get(import_task_detail))
        .route("/import/tasks/{task_id}/cancel", post(import_cancel))
        .route(
            "/connections/{name}/import/upload",
            post(upload_import_file).layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/connections/{name}/databases/{db}/tables",
            get(list_tables).post(create_table_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/count",
            get(table_count),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/sizes",
            get(db_sizes),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/drop",
            post(drop_tables_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/truncate",
            post(truncate_tables_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/repair",
            post(repair_tables_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/optimize",
            post(optimize_tables_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/analyze",
            post(analyze_tables_handler),
        )
        .route(
            "/connections/{name}/databases/{db}/tables/check",
            post(check_tables_handler),
        )
        .route("/connections/{name}/tables/{table}/info", get(table_info))
        .route("/connections/{name}/tables/{table}", delete(drop_table))
        .route(
            "/connections/{name}/tables/{table}/truncate",
            post(truncate_table),
        )
        .route(
            "/connections/{name}/tables/{table}/alter",
            post(alter_table),
        )
        .route(
            "/connections/{name}/tables/{table}/columns",
            get(show_columns),
        )
        .route(
            "/connections/{name}/tables/{table}/create-table",
            get(show_create_table),
        )
        .route(
            "/connections/{name}/tables/{table}/select",
            post(select_data),
        )
        .route(
            "/connections/{name}/tables/{table}/export",
            post(export_data),
        )
        .route("/connections/{name}/tables/{table}/count", post(count_data))
        .route(
            "/connections/{name}/tables/{table}/insert",
            post(insert_data),
        )
        .route(
            "/connections/{name}/tables/{table}/data",
            patch(update_data).delete(delete_data),
        )
        .route(
            "/connections/{name}/tables/{table}/indexes",
            get(list_indexes).post(create_index_handler),
        )
        .route(
            "/connections/{name}/tables/{table}/indexes/{index}",
            delete(drop_index_handler),
        )
        .route(
            "/connections/{name}/users",
            get(list_users).post(create_user),
        )
        .route(
            "/connections/{name}/users/{username}",
            get(user_info_handler)
                .put(alter_user_handler)
                .delete(drop_user_handler),
        )
        .route(
            "/connections/{name}/users/{username}/rename",
            post(rename_user),
        )
        .route(
            "/connections/{name}/users/{username}/grant",
            post(grant_handler),
        )
        .route(
            "/connections/{name}/users/{username}/revoke",
            post(revoke_handler),
        )
        .route("/connections/{name}/processes", get(process_list))
        .route(
            "/connections/{name}/processes/kill",
            post(kill_process_handler),
        )
        .route("/connections/{name}/variables", get(variables_handler))
        .route("/connections/{name}/status", get(status_handler))
        .route("/connections/{name}/version", get(version_handler))
        .route("/connections/{name}/ping", post(ping_connection))
        .route("/connections/{name}/complete", get(complete_handler))
}

// ═══════════════════════════════════════════════════════════════════
//  Health & Systems
// ═══════════════════════════════════════════════════════════════════

async fn health_check() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

async fn list_systems() -> Json<Value> {
    let systems: Vec<Value> = asql_query::database_types()
        .into_iter()
        .map(|(info, params)| {
            let value = info.enum_name.to_lowercase();
            json!({
                "value": value,
                "label": info.label,
                "defaultPort": info.default_port,
                "params": params,
            })
        })
        .collect();
    Json(Value::Array(systems))
}

// ═══════════════════════════════════════════════════════════════════
//  Connections
// ═══════════════════════════════════════════════════════════════════

async fn list_connections(State(bk): State<AppState>) -> Json<Value> {
    let conns = bk.list_connections().await;
    let result: Vec<Value> = conns
        .into_iter()
        .map(|(name, url, db_type)| json!({"name": name, "url": url, "db_type": format!("{:?}", db_type)}))
        .collect();
    Json(Value::Array(result))
}

async fn add_connection(
    State(bk): State<AppState>,
    Json(config): Json<asql_backend::ConnectionConfig>,
) -> (StatusCode, Json<Value>) {
    match bk.add_connection(config).await {
        Ok(()) => {
            bk.save_and_reload_connections().await;
            ok_json(json!({"success": true}))
        }
        Err(e) => err_json(e),
    }
}

async fn remove_connection(State(bk): State<AppState>, Path(p): Path<ConnPath>) -> Json<Value> {
    bk.remove_connection(&p.name).await;
    bk.save_and_reload_connections().await;
    Json(json!({"success": true}))
}

async fn test_connection(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    match bk.test_connection(&p.name).await {
        Ok(()) => ok_json(json!({"success": true})),
        Err(e) => err_json(e),
    }
}

async fn execute_query(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(body): Json<QueryBody>,
) -> Json<Value> {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(_) => return Json(json!({"error": "connection not found"})),
    };
    let results = qb.execute_raw_batch(&p.name, &body.sql, body.stop_on_error).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{}", e)}),
        })
        .collect();
    Json(Value::Array(json_results))
}

// ═══════════════════════════════════════════════════════════════════
//  Import (server-side file)
// ═══════════════════════════════════════════════════════════════════

const PREVIEW_LINES: usize = 100;

async fn import_preview(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(body): Json<ImportPreviewBody>,
) -> (StatusCode, Json<Value>) {
    let _ = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let content = match tokio::fs::read_to_string(&body.file_path).await {
        Ok(c) => c,
        Err(e) => return err_json(format!("Failed to read file: {}", e)),
    };
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let file_size = content.len();
    let (head, tail) = if total_lines <= PREVIEW_LINES * 2 {
        (content.clone(), String::new())
    } else {
        let head = lines[..PREVIEW_LINES].join("\n");
        let tail = lines[total_lines - PREVIEW_LINES..].join("\n");
        (head, tail)
    };
    ok_json(json!({
        "total_lines": total_lines,
        "file_size": file_size,
        "head": head,
        "tail": tail,
        "omitted": total_lines.saturating_sub(PREVIEW_LINES * 2),
    }))
}

async fn import_execute(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(body): Json<ImportExecuteBody>,
) -> (StatusCode, Json<Value>) {
    let dm = bk.db_manager().await;
    if dm.get_connection_url(&p.name).await.is_none() {
        return err_json(format!("Connection '{}' not found", p.name));
    }
    if tokio::fs::metadata(&body.file_path).await.is_err() {
        return err_json(format!("File not found: {}", body.file_path));
    }

    let task_id = format!(
        "import_{}_{}",
        now_millis(),
        IMPORT_TASK_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let created_at = now_millis();
    let file_name = body.file_name.clone().unwrap_or_else(|| {
        std::path::Path::new(&body.file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| body.file_path.clone())
    });
    let task = ImportTask {
        id: task_id.clone(),
        status: "running".to_string(),
        total: 0,
        current: 0,
        succeeded: 0,
        failed: 0,
        errors: Vec::new(),
        duration_ms: 0,
        created_at,
        finished_at: None,
        error: None,
        cancel_requested: false,
        connection: p.name.clone(),
        database: body.database.clone(),
        file_name,
        file_path: body.file_path.clone(),
        total_lines: 0,
        file_size: 0,
        preview_head: String::new(),
        preview_tail: String::new(),
        preview_omitted: 0,
        stop_on_error: body.stop_on_error,
        single_transaction: body.single_transaction,
    };
    {
        let tasks = import_tasks();
        let mut guard = tasks.lock().unwrap();
        guard.retain(|_, t| t.status == "running" || created_at.saturating_sub(t.created_at) < 3600_000);
        guard.insert(task_id.clone(), task);
    }

    let conn_name = p.name.clone();
    let file_path = body.file_path.clone();
    let stop_on_error = body.stop_on_error;
    let database = body.database.clone();
    let single_transaction = body.single_transaction;
    let spawn_id = task_id.clone();
    tokio::spawn(async move {
        run_import_task(
            dm,
            conn_name,
            database,
            file_path,
            stop_on_error,
            single_transaction,
            spawn_id,
        )
        .await;
    });

    ok_json(json!({ "task_id": task_id }))
}

async fn run_import_task(
    dm: Arc<DbManager>,
    conn: String,
    database: Option<String>,
    file_path: String,
    stop_on_error: bool,
    single_transaction: bool,
    task_id: String,
) {
    let tasks = import_tasks();
    let start = std::time::Instant::now();

    let content = match tokio::fs::read_to_string(&file_path).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[IMPORT TASK {}] read failed: {}", task_id, e);
            update_import_task(&tasks, &task_id, |t| {
                t.status = "failed".to_string();
                t.error = Some(format!("Failed to read file: {}", e));
            });
            return;
        }
    };
    tracing::info!(
        "[IMPORT TASK {}] read file ({} bytes) in {:?}",
        task_id,
        content.len(),
        start.elapsed()
    );
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let file_size = content.len();
    let (preview_head, preview_tail, preview_omitted) = if total_lines <= PREVIEW_LINES * 2 {
        (content.clone(), String::new(), 0usize)
    } else {
        (
            lines[..PREVIEW_LINES].join("\n"),
            lines[total_lines - PREVIEW_LINES..].join("\n"),
            total_lines - PREVIEW_LINES * 2,
        )
    };
    update_import_task(&tasks, &task_id, |t| {
        t.total_lines = total_lines;
        t.file_size = file_size;
        t.preview_head = preview_head;
        t.preview_tail = preview_tail;
        t.preview_omitted = preview_omitted;
    });
    let split_start = std::time::Instant::now();
    let statements: Vec<String> = match tokio::task::spawn_blocking(move || {
        split_sql_statements(&content)
            .iter()
            .map(|s| s.to_string())
            .collect()
    })
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[IMPORT TASK {}] split failed: {}", task_id, e);
            update_import_task(&tasks, &task_id, |t| {
                t.status = "failed".to_string();
                t.error = Some(format!("Failed to split SQL: {}", e));
            });
            return;
        }
    };
    tracing::info!(
        "[IMPORT TASK {}] split into {} statements in {:?}",
        task_id,
        statements.len(),
        split_start.elapsed()
    );

    let total = statements.len();
    update_import_task(&tasks, &task_id, |t| t.total = total);

    if import_cancel_requested(&tasks, &task_id) {
        update_import_task(&tasks, &task_id, |t| {
            t.status = "cancelled".to_string();
            t.finished_at = Some(now_millis());
        });
        tracing::info!("[IMPORT TASK {}] cancelled before execution", task_id);
        return;
    }

    let mut last_error_count = 0usize;
    let progress_task_id = task_id.clone();
    let progress = DbManager::execute_sql_batch_owned_send(
        dm,
        &conn,
        database,
        statements,
        stop_on_error,
        true,
        single_transaction,
        move |p| {
            let cancel = import_cancel_requested(&tasks, &progress_task_id);
            update_import_task(&tasks, &progress_task_id, |t| {
                t.current = p.done;
                t.succeeded = p.succeeded;
                t.failed = p.failed;
                for (idx, err) in p.errors.iter().skip(last_error_count) {
                    t.errors.push(json!({
                        "index": *idx,
                        "error": err,
                    }));
                }
                last_error_count = p.errors.len();
            });
            !cancel
        },
    )
    .await;

    if let Some(fatal) = progress.fatal_error {
        update_import_task(&tasks, &task_id, |t| {
            t.status = "failed".to_string();
            t.error = Some(fatal.clone());
            t.finished_at = Some(now_millis());
        });
        tracing::error!("[IMPORT TASK {}] failed: {}", task_id, fatal);
        return;
    }

    let status = if progress.cancelled {
        "cancelled"
    } else {
        "completed"
    };
    update_import_task(&tasks, &task_id, |t| {
        t.status = status.to_string();
        t.duration_ms = start.elapsed().as_millis() as u64;
        t.finished_at = Some(now_millis());
    });
    tracing::info!(
        "[IMPORT TASK {}] {}: {} succeeded, {} failed, total {:?}",
        task_id,
        status,
        progress.succeeded,
        progress.failed,
        start.elapsed()
    );
}

async fn import_tasks_list(State(_bk): State<AppState>) -> (StatusCode, Json<Value>) {
    let tasks = import_tasks();
    let guard = tasks.lock().unwrap();
    let mut list: Vec<Value> = guard
        .values()
        .map(|t| {
            json!({
                "id": t.id,
                "connection": t.connection,
                "database": t.database,
                "file_name": t.file_name,
                "file_path": t.file_path,
                "status": t.status,
                "total": t.total,
                "total_lines": t.total_lines,
                "file_size": t.file_size,
                "current": t.current,
                "succeeded": t.succeeded,
                "failed": t.failed,
                "error_count": t.errors.len(),
                "duration_ms": t.duration_ms,
                "created_at": t.created_at,
                "finished_at": t.finished_at,
                "stop_on_error": t.stop_on_error,
                "single_transaction": t.single_transaction,
            })
        })
        .collect();
    list.sort_by_key(|v| v["created_at"].as_u64().unwrap_or(0));
    list.reverse();
    list.truncate(50);
    ok_json(json!({ "tasks": list }))
}

async fn import_task_detail(
    State(_bk): State<AppState>,
    Path(p): Path<ImportTaskPath>,
) -> (StatusCode, Json<Value>) {
    let tasks = import_tasks();
    match tasks.lock().unwrap().get(&p.task_id) {
        Some(task) => ok_json(json!(task)),
        None => err_json("Import task not found"),
    }
}

async fn import_cancel(
    State(_bk): State<AppState>,
    Path(p): Path<ImportTaskPath>,
) -> (StatusCode, Json<Value>) {
    let tasks = import_tasks();
    let mut guard = tasks.lock().unwrap();
    match guard.get_mut(&p.task_id) {
        Some(t) if t.status == "running" => {
            t.cancel_requested = true;
            ok_json(json!({ "ok": true }))
        }
        Some(_) => err_json("Task is not running"),
        None => err_json("Import task not found"),
    }
}

async fn upload_import_file(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    mut multipart: Multipart,
) -> (StatusCode, Json<Value>) {
    let _ = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => return err_json("No file uploaded"),
        Err(e) => return err_json(format!("Multipart error: {}", e)),
    };
    let original_name = field
        .file_name()
        .unwrap_or("upload.sql")
        .to_string();
    let data = match field.bytes().await {
        Ok(d) => d.to_vec(),
        Err(e) => return err_json(format!("Failed to read file body: {}", e)),
    };
    let file_size = data.len();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_path = std::env::temp_dir().join(format!("asql_import_{}.sql", timestamp));
    if let Err(e) = tokio::fs::write(&temp_path, &data).await {
        return err_json(format!("Failed to save temp file: {}", e));
    }
    tracing::info!(
        "[IMPORT UPLOAD] {} -> {} ({} bytes)",
        original_name,
        temp_path.display(),
        file_size
    );
    ok_json(json!({
        "file_path": temp_path.to_string_lossy(),
        "original_name": original_name,
        "file_size": file_size,
    }))
}

// ═══════════════════════════════════════════════════════════════════

async fn list_databases(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.list_databases(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn create_database(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(b): Json<CreateDbBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .create_database(
            &p.name,
            &b.name,
            false,
            b.character_set.as_deref(),
            b.collation.as_deref(),
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn drop_database(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.drop_database(&p.name, &p.db, true).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn alter_database(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<AlterDbBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .alter_database(
            &p.name,
            &p.db,
            b.character_set.as_deref(),
            b.collation.as_deref(),
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn current_database(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.current_database(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn use_database_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(b): Json<UseDatabaseBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.use_database(&p.name, &b.database).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Tables
// ═══════════════════════════════════════════════════════════════════

async fn list_tables(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Query(q): Query<ListTablesQuery>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mode = match q.match_mode.as_deref() {
        Some("exact") => TableNameMatch::Exact,
        Some("starts_with") => TableNameMatch::StartsWith,
        _ => TableNameMatch::Contains,
    };
    match qb
        .list_tables(&p.name, Some(&p.db), q.table_name.as_deref(), mode)
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn table_count(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.table_count(&p.name, Some(&p.db)).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn db_sizes(State(bk): State<AppState>, Path(p): Path<DbPath>) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.db_sizes(&p.name, Some(&p.db)).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn table_info(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Query(q): Query<TableQuery>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .table_info(&p.name, q.database.as_deref(), &p.table)
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn create_table_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<CreateTableBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut tb = CreateTableBuilder::new().table(&b.table);
    for col in &b.columns {
        tb = tb.column(column_def(col));
        if col.primary_key {
            tb = tb.primary_key(vec![col.name.as_str()]);
        }
    }
    if let Some(e) = &b.engine {
        tb = tb.engine(e);
    }
    if let Some(c) = &b.collation {
        tb = tb.collation(c);
    }
    if let Some(c) = &b.comment {
        tb = tb.comment(c);
    }
    match qb.create_table(&p.name, &tb).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn drop_table(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.drop_table(&p.name, &p.table, true).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn truncate_table(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.truncate_table(&p.name, &p.table).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn drop_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.drop_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn truncate_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.truncate_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn repair_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.repair_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn optimize_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.optimize_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn analyze_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.analyze_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn check_tables_handler(
    State(bk): State<AppState>,
    Path(p): Path<DbPath>,
    Json(b): Json<TablesBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let table_refs: Vec<&str> = b.tables.iter().map(|s| s.as_str()).collect();
    let results = qb.check_tables(&p.name, &table_refs).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

/// alter_table: returns array of results, one per action
async fn alter_table(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<AlterTableBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut ab = AlterTableBuilder::new().table(&p.table);
    for c in &b.add_columns {
        let cd = column_def(c);
        match &c.after_column {
            Some(pos) if pos == "FIRST" => ab = ab.add_column_first(cd),
            Some(after) => ab = ab.add_column_after(cd, after),
            None => ab = ab.add_column(cd),
        }
    }
    for c in &b.modify_columns {
        let cd = column_def(c);
        match &c.after_column {
            Some(pos) if pos == "FIRST" => ab = ab.modify_column_first(cd),
            Some(after) => ab = ab.modify_column_after(cd, after),
            None => ab = ab.modify_column(cd),
        }
    }
    for c in &b.change_columns {
        let cd = column_def(&c.new_def);
        match &c.new_def.after_column {
            Some(pos) if pos == "FIRST" => ab = ab.change_column_first(&c.old_name, cd),
            Some(after) => ab = ab.change_column_after(&c.old_name, cd, after),
            None => ab = ab.change_column(&c.old_name, cd),
        }
    }
    for c in &b.drop_columns {
        ab = ab.drop_column(c);
    }
    for idx in &b.add_indexes {
        let it = index_type(&idx.index_type);
        let cols: Vec<(&str, Option<usize>)> =
            idx.columns.iter().map(|c| (c.as_str(), None)).collect();
        ab = ab.add_index(&idx.name, it, cols);
    }
    for idx in &b.drop_indexes {
        ab = ab.drop_index(idx);
    }
    if let Some(n) = &b.rename_table {
        ab = ab.rename_table(n);
    }
    if let Some(e) = &b.engine {
        ab = ab.engine(e);
    }
    if let Some(c) = &b.collation {
        ab = ab.collation(c);
    }
    if let Some(c) = &b.comment {
        ab = ab.comment(c);
    }
    let results = qb.alter_table(&p.name, &ab).await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!(s),
            Err(e) => json!({"error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

// ═══════════════════════════════════════════════════════════════════
//  Columns
// ═══════════════════════════════════════════════════════════════════

async fn show_columns(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Query(q): Query<TableQuery>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .show_columns(&p.name, &p.table, q.database.as_deref())
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn show_create_table(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.show_create_table(&p.name, &p.table).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Data
// ═══════════════════════════════════════════════════════════════════

async fn select_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<SelectBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let sb = select_builder(&p.table, &b);
    match qb.select(&p.name, &sb).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn export_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<ExportBody>,
) -> Response {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e.into_response(),
    };

    let columns = if b.columns.is_empty() {
        ColumnTarget::All
    } else {
        ColumnTarget::Selected(b.columns.clone())
    };

    let mut wb = WhereBuilder::new();
    for wc in &b.where_conditions {
        wb = wb.and(&wc.column, &wc.operator, &wc.value);
    }

    let obs = b
        .order_by
        .iter()
        .map(|o| OrderBy {
            column: o.column.clone(),
            desc: o.desc,
        })
        .collect::<Vec<_>>();
    let data_format = parse_data_format(&b.format).unwrap_or(DataFormat::Sql);

    let export_conf = ExportBuilder {
        database: b.database.clone(),
        db_option: DatabaseOption::Skip,
        tables: ExportTable::Selected(vec![ExportTableDef {
            name: p.table.clone(),
            columns,
            filter_sql: wb,
            order_by: obs,
            ddl: false,
            data: true,
        }]),
        table_option: TableOption::Skip,
        data_format,
    };

    let rx = match qb.export_select(&p.name, export_conf).await {
        Ok(rx) => rx,
        Err(e) => return err_json(e).into_response(),
    };

    let content_type = match b.format.as_str() {
        "csv" => "text/csv; charset=utf-8",
        "sql" => "application/sql; charset=utf-8",
        _ => "text/tab-separated-values; charset=utf-8",
    };
    let filename = format!("{}.{}", p.table, b.format);
    let disposition = match b.method.as_str() {
        "save" => format!("attachment; filename=\"{}\"", filename),
        _ => format!("inline; filename=\"{}\"", filename),
    };

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|line| Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)));
    let body = Body::from_stream(stream);

    Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Disposition", disposition)
        .body(body)
        .unwrap()
}

async fn export_data_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(body): Json<ExportRequestBody>,
) -> Response {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e.into_response(),
    };

    let db_option = match parse_db_option(&body.db_option) {
        Ok(v) => v,
        Err(e) => return err_json(e).into_response(),
    };
    let table_option = match parse_table_option(&body.table_option) {
        Ok(v) => v,
        Err(e) => return err_json(e).into_response(),
    };
    let data_format = match parse_data_format(&body.data_format) {
        Ok(v) => v,
        Err(e) => return err_json(e).into_response(),
    };

    let tables = if body.tables_all {
        ExportTable::All
    } else {
        let mut tds = Vec::new();
        for t in body.tables {
            let columns = match parse_column_target(&t.columns) {
                Ok(v) => v,
                Err(e) => return err_json(e).into_response(),
            };
            let mut wb = WhereBuilder::new();
            for wc in &t.where_conditions {
                wb = wb.and(&wc.column, &wc.operator, &wc.value);
            }
            let obs = t
                .order_by
                .into_iter()
                .map(|o| OrderBy {
                    column: o.column,
                    desc: o.desc,
                })
                .collect::<Vec<_>>();
            tds.push(ExportTableDef {
                name: t.name,
                columns,
                filter_sql: wb,
                order_by: obs,
                ddl: t.ddl,
                data: t.data,
            });
        }
        ExportTable::Selected(tds)
    };

    let export_conf = ExportBuilder {
        database: body.database,
        db_option,
        tables,
        table_option,
        data_format,
    };

    let rx = match qb.export_select(&p.name, export_conf).await {
        Ok(rx) => rx,
        Err(e) => return err_json(e).into_response(),
    };

    let content_type = match body.data_format.as_str() {
        "csv" => "text/csv; charset=utf-8",
        "sql" => "application/sql; charset=utf-8",
        _ => "text/tab-separated-values; charset=utf-8",
    };
    let filename = format!("export.{}", body.data_format);
    let disposition = match body.method.as_str() {
        "save" => format!("attachment; filename=\"{}\"", filename),
        _ => format!("inline; filename=\"{}\"", filename),
    };

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(|line| Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)));
    let body_stream = Body::from_stream(stream);

    Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Disposition", disposition)
        .body(body_stream)
        .unwrap()
}

async fn count_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<CountBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut sb = SelectBuilder::new().from(&p.table);
    for w in &b.where_conditions {
        sb = sb.and_where(&w.column, &w.operator, &w.value);
    }
    match qb.select_count(&p.name, &sb).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn insert_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<InsertBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut ib = InsertBuilder::new().into(&p.table);
    for c in &b.columns {
        ib = ib.column(c);
    }
    for row in &b.values {
        ib = ib.row(row.iter().map(|s| s.as_str()).collect());
    }
    match qb.insert(&p.name, &ib).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn update_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<UpdateBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut ub = UpdateBuilder::new().table(&p.table);
    for s in &b.sets {
        ub = ub.set(&s.column, &s.value);
    }
    for w in &b.where_conditions {
        ub = ub.and_where(&w.column, &w.operator, &w.value);
    }
    if let Some(l) = b.limit {
        ub = ub.limit(l);
    }
    match qb.update(&p.name, &ub).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn delete_data(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<DeleteBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let mut dbl = DeleteBuilder::new().from(&p.table);
    for w in &b.where_conditions {
        dbl = dbl.and_where(&w.column, &w.operator, &w.value);
    }
    if let Some(l) = b.limit {
        dbl = dbl.limit(l);
    }
    match qb.delete(&p.name, &dbl).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Indexes
// ═══════════════════════════════════════════════════════════════════

async fn list_indexes(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.list_indexes(&p.name, &p.table).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn create_index_handler(
    State(bk): State<AppState>,
    Path(p): Path<TablePath>,
    Json(b): Json<CreateIndexBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let it = index_type(&b.index_type);
    let cols: Vec<(&str, Option<usize>)> = b
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.prefix_len))
        .collect();
    match qb
        .create_index(
            &p.name,
            &p.table,
            &b.name,
            it,
            cols,
            b.method
                .as_deref()
                .map(|m| match m.to_uppercase().as_str() {
                    "HASH" => asql_query::IndexMethod::Hash,
                    _ => asql_query::IndexMethod::BTree,
                }),
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn drop_index_handler(
    State(bk): State<AppState>,
    Path(p): Path<IndexPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.drop_index(&p.name, &p.index, &p.table, true).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Users & Privileges
// ═══════════════════════════════════════════════════════════════════

async fn list_users(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.list_users(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn user_info_handler(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Query(q): Query<UserQuery>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.user_info(&p.name, &p.username, q.host.as_deref()).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn create_user(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(b): Json<CreateUserBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .create_user(
            &p.name,
            &b.username,
            b.password.as_deref(),
            b.host.as_deref(),
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn alter_user_handler(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Json(b): Json<AlterUserBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .alter_user(&p.name, &p.username, &b.password, b.host.as_deref())
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn drop_user_handler(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Json(b): Json<DropUserBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.drop_user(&p.name, &p.username, b.host.as_deref()).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn rename_user(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Json(b): Json<RenameUserBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb
        .rename_user(
            &p.name,
            &p.username,
            &b.new_username,
            None,
            b.new_host.as_deref(),
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn grant_handler(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Json(b): Json<GrantBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let privs: Vec<&str> = b.privileges.iter().map(|s| s.as_str()).collect();
    match qb
        .grant(
            &p.name,
            privs,
            &b.on,
            &p.username,
            b.host.as_deref(),
            b.with_grant_option,
        )
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn revoke_handler(
    State(bk): State<AppState>,
    Path(p): Path<UserPath>,
    Json(b): Json<RevokeBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let privs: Vec<&str> = b.privileges.iter().map(|s| s.as_str()).collect();
    match qb
        .revoke(&p.name, privs, &b.on, &p.username, b.host.as_deref())
        .await
    {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Server
// ═══════════════════════════════════════════════════════════════════

async fn process_list(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.process_list(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

/// kill_process_handler: returns array per PID
async fn kill_process_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
    Json(b): Json<KillBody>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    let results = qb
        .kill_process(
            &p.name,
            &b.pids.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
        )
        .await;
    let json_results: Vec<Value> = results
        .into_iter()
        .map(|r| match r {
            Ok(s) => json!({"success": true, "data": s}),
            Err(e) => json!({"success": false, "error": format!("{e}")}),
        })
        .collect();
    ok_json(Value::Array(json_results))
}

async fn variables_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.variables(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn status_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.status(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn column_types_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let dm = bk.db_manager().await;
    let url = match dm.get_connection_url(&p.name).await {
        Some(url) => url,
        None => return err_json(format!("Connection '{}' not found", p.name)),
    };
    let db_type = DatabaseType::from_url(&url);
    let types = db_type.data_types_info();

    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for dt in types {
        let cat = format!("{:?}", dt.category);
        map.entry(cat).or_default().push(dt.name.to_string());
    }

    let result: Vec<Value> = map
        .into_iter()
        .map(|(category, types)| json!({"category": category, "types": types}))
        .collect();

    ok_json(json!(result))
}

async fn charsets_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let dm = bk.db_manager().await;
    let url = match dm.get_connection_url(&p.name).await {
        Some(url) => url,
        None => return err_json(format!("Connection '{}' not found", p.name)),
    };
    let db_type = DatabaseType::from_url(&url);
    let charsets = db_type.charsets();

    let result: Vec<Value> = charsets
        .iter()
        .map(|cs| {
            json!({
                "charset": cs.name,
                "collations": cs.collations,
            })
        })
        .collect();

    ok_json(json!(result))
}

async fn functions_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let dm = bk.db_manager().await;
    let url = match dm.get_connection_url(&p.name).await {
        Some(url) => url,
        None => return err_json(format!("Connection '{}' not found", p.name)),
    };
    let db_type = DatabaseType::from_url(&url);
    let functions = asql_query::functions_of(db_type);
    ok_json(json!(functions))
}

async fn version_handler(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.version(&p.name).await {
        Ok(r) => ok_json(json!(r)),
        Err(e) => err_json(e),
    }
}

async fn ping_connection(
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> (StatusCode, Json<Value>) {
    let qb = match create_qb(&bk, &p.name).await {
        Ok(qb) => qb,
        Err(e) => return e,
    };
    match qb.execute_raw(&p.name, "SELECT 1").await {
        Ok(_) => ok_json(json!({"success": true})),
        Err(e) => err_json(e),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  WebSocket — SQL completion channel
// ═══════════════════════════════════════════════════════════════════

async fn complete_handler(
    ws: WebSocketUpgrade,
    State(bk): State<AppState>,
    Path(p): Path<ConnPath>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, bk, p.name))
}

async fn handle_socket(mut socket: WebSocket, bk: BackendHandle, conn_name: String) {
    tracing::info!(conn = %conn_name, "ws completion connected");

    let (db_type, provider) = {
        let dm = bk.db_manager().await;
        let url = match dm.get_connection_url(&conn_name).await {
            Some(u) => u,
            None => {
                tracing::warn!(conn = %conn_name, "connection not found, closing ws");
                return;
            }
        };
        let db_type = DatabaseType::from_url(&url);
        let qb = match create_qb(&bk, &conn_name).await {
            Ok(qb) => qb,
            Err(e) => {
                tracing::warn!(conn = %conn_name, error = ?e, "create_qb failed, closing ws");
                return;
            }
        };
        let provider = Arc::new(DbSchemaProvider::new(Arc::new(qb), conn_name.clone()).await);
        (db_type, provider)
    };

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => match serde_json::from_str::<CompletionRequestMsg>(&text) {
                Ok(req) => {
                    let suggestions =
                        get_suggestions(db_type, provider.clone(), &req.sql, req.cursor).await;
                    tracing::info!(
                        conn = %conn_name,
                        seq = req.seq,
                        sql = %req.sql,
                        cursor = req.cursor,
                        hint_count = suggestions.len(),
                        hints = ?suggestions.iter().map(|s| &s.text).collect::<Vec<_>>(),
                        "completion result"
                    );
                    let items: Vec<Value> = suggestions
                        .iter()
                        .map(|s| {
                            let kind = match s.kind {
                                SuggestionKind::Keyword => "keyword",
                                SuggestionKind::Table => "table",
                                SuggestionKind::Column => "column",
                                SuggestionKind::Function => "function",
                                SuggestionKind::Alias => "alias",
                            };
                            json!({"text": s.text, "kind": kind})
                        })
                        .collect();
                    let resp = json!({
                        "seq": req.seq,
                        "cursor": req.cursor,
                        "items": items,
                    });
                    let _ = socket.send(Message::Text(resp.to_string().into())).await;
                }
                Err(_) => {
                    tracing::debug!("invalid ws message: {}", text);
                }
            },
            Message::Close(_) => break,
            _ => {}
        }
    }
    tracing::info!(conn = %conn_name, "ws completion disconnected");
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_backend() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        asql_backend::BackendHandle::new(dir.path().to_path_buf())
    }

    fn test_app() -> axum::Router {
        build_router().with_state(test_backend())
    }

    async fn get_body(app: axum::Router, uri: &str) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn post_body(app: axum::Router, uri: &str, body_val: Value) -> Value {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_vec(&body_val).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_health() {
        assert_eq!(get_body(test_app(), "/health").await["status"], "ok");
    }

    #[tokio::test]
    async fn test_list_connections_empty() {
        assert!(get_body(test_app(), "/connections")
            .await
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_add_connection_invalid_body() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/connections")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_query_nonexistent() {
        let result = post_body(
            test_app(),
            "/connections/nonexist/query",
            json!({"sql": "SELECT 1"}),
        )
        .await;
        assert!(result.is_object());
        assert!(result.get("error").is_some());
    }
}
