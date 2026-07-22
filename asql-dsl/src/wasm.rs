use crate::ddl::*;
use crate::dialect::{Dialect, MySql, PostgreSql, Sqlite};
use crate::dml::*;
use crate::dql::{SelectBuilder, WhereBuilder};
use crate::introspection::{
    ColumnsIntrospection, DatabasesIntrospection, IndexesIntrospection,
    ServerIntrospection, TableNameMatch, TablesIntrospection,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

fn get_dialect(name: &str) -> Box<dyn Dialect> {
    match name {
        "mysql" => Box::new(MySql),
        "postgres" => Box::new(PostgreSql),
        "sqlite" => Box::new(Sqlite),
        _ => Box::new(MySql),
    }
}

#[derive(Deserialize)]
struct WhereJson {
    column: String,
    operator: String,
    value: String,
    #[serde(default)]
    logical: Option<String>,
}

fn make_where(items: &[WhereJson]) -> WhereBuilder {
    let mut wb = WhereBuilder::new();
    for item in items {
        let op = item.logical.as_deref().unwrap_or("AND");
        match op {
            "OR" => {
                wb = wb.or(&item.column, &item.operator, &item.value);
            }
            _ => {
                wb = wb.and(&item.column, &item.operator, &item.value);
            }
        }
    }
    wb
}

// ── Introspection ──────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn list_tables_sql(dialect: &str, database: Option<String>) -> String {
    let d = get_dialect(dialect);
    TablesIntrospection::list_tables(&*d, database.as_deref(), None, TableNameMatch::Contains)
}

/// Returns SQL to get metadata for a single table, including its comment.
#[wasm_bindgen]
pub fn table_info_sql(dialect: &str, database: &str, table: &str) -> String {
    let d = get_dialect(dialect);
    TablesIntrospection::table_info(&*d, Some(database), table)
}

#[wasm_bindgen]
pub fn list_databases_sql(dialect: &str) -> String {
    let d = get_dialect(dialect);
    DatabasesIntrospection::list_databases(&*d)
}

/// Returns SQL to query the current database name for the given dialect.
/// The result column is always named `db`.
#[wasm_bindgen]
pub fn current_database_sql(dialect: &str) -> String {
    let d = get_dialect(dialect);
    DatabasesIntrospection::current_database(&*d)
}

/// Returns SQL to query the database server version for the given dialect.
/// The result column is always named `version`.
#[wasm_bindgen]
pub fn version_sql(dialect: &str) -> String {
    let d = get_dialect(dialect);
    ServerIntrospection::version(&*d)
}

#[wasm_bindgen]
pub fn show_columns_sql(dialect: &str, table: &str, database: Option<String>) -> String {
    let d = get_dialect(dialect);
    ColumnsIntrospection::show_columns(&*d, table, database.as_deref())
}

#[wasm_bindgen]
pub fn show_create_table_sql(dialect: &str, table: &str) -> String {
    let d = get_dialect(dialect);
    ColumnsIntrospection::show_create_table(&*d, table)
}

#[wasm_bindgen]
pub fn list_indexes_sql(dialect: &str, table: &str) -> String {
    let d = get_dialect(dialect);
    IndexesIntrospection::list_indexes(&*d, table)
}

// ── Select ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SelectColumnJson {
    name: String,
    #[serde(default)]
    func: Option<String>,
}

#[derive(Deserialize)]
struct OrderByJson {
    column: String,
    #[serde(default)]
    desc: bool,
}

#[derive(Deserialize)]
struct SelectParams {
    table: String,
    #[serde(default)]
    columns: Vec<SelectColumnJson>,
    #[serde(default, rename = "where")]
    where_: Vec<WhereJson>,
    #[serde(default, rename = "orderBy")]
    order_by: Vec<OrderByJson>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

fn default_dialect() -> String {
    "mysql".to_string()
}

#[wasm_bindgen]
pub fn select_sql(json: &str) -> String {
    let params: SelectParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut sb = SelectBuilder::new();
    for c in &params.columns {
        match &c.func {
            Some(f) => sb = sb.column_with_func(f, &c.name),
            None => sb = sb.column(&c.name),
        }
    }
    sb = sb.from(&params.table);
    let wb = make_where(&params.where_);
    if !wb.is_empty() {
        sb = sb.where_(wb);
    }
    for o in &params.order_by {
        sb = sb.order_by(&o.column, o.desc);
    }
    if let Some(n) = params.limit {
        sb = sb.limit(n);
    }
    if let Some(n) = params.offset {
        sb = sb.offset(n);
    }
    sb.build(&*dialect)
}

// ── Count ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CountParams {
    table: String,
    #[serde(default, rename = "where")]
    where_: Vec<WhereJson>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn count_sql(json: &str) -> String {
    let params: CountParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let wb = make_where(&params.where_);
    let mut sb = SelectBuilder::new();
    if !wb.is_empty() {
        sb = sb.where_(wb);
    }
    sb.from(&params.table).build_count(&*dialect)
}

// ── Insert ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct InsertParams {
    table: String,
    columns: Vec<String>,
    values: Vec<Vec<String>>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn insert_sql(json: &str) -> String {
    let params: InsertParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut ib = InsertBuilder::new().into(&params.table);
    for c in &params.columns {
        ib = ib.column(c);
    }
    for row in &params.values {
        let refs: Vec<&str> = row.iter().map(String::as_str).collect();
        ib = ib.row(refs);
    }
    ib.build(&*dialect)
}

// ── Update ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct UpdateSetJson {
    column: String,
    value: String,
}

#[derive(Deserialize)]
struct UpdateParams {
    table: String,
    sets: Vec<UpdateSetJson>,
    #[serde(default, rename = "where")]
    where_: Vec<WhereJson>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn update_sql(json: &str) -> String {
    let params: UpdateParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut ub = UpdateBuilder::new().table(&params.table);
    for s in &params.sets {
        ub = ub.set(&s.column, &s.value);
    }
    for item in &params.where_ {
        let op = item.logical.as_deref().unwrap_or("AND");
        match op {
            "OR" => ub = ub.or_where(&item.column, &item.operator, &item.value),
            _ => ub = ub.and_where(&item.column, &item.operator, &item.value),
        }
    }
    if let Some(n) = params.limit {
        ub = ub.limit(n);
    }
    ub.build(&*dialect)
}

// ── Delete ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DeleteParams {
    table: String,
    #[serde(default, rename = "where")]
    where_: Vec<WhereJson>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn delete_sql(json: &str) -> String {
    let params: DeleteParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut db = DeleteBuilder::new().from(&params.table);
    for item in &params.where_ {
        let op = item.logical.as_deref().unwrap_or("AND");
        match op {
            "OR" => db = db.or_where(&item.column, &item.operator, &item.value),
            _ => db = db.and_where(&item.column, &item.operator, &item.value),
        }
    }
    if let Some(n) = params.limit {
        db = db.limit(n);
    }
    db.build(&*dialect)
}

// ── CreateTable ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ColumnDefJson {
    name: String,
    #[serde(rename = "type")]
    data_type: String,
    #[serde(default)]
    length: Option<String>,
    #[serde(default)]
    options: String,
    #[serde(default = "default_true")]
    nullable: bool,
    #[serde(default)]
    auto_increment: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    on_update: bool,
}

fn to_column_type(data_type: &str, length: &Option<String>, options: &str) -> asql_types::ColumnType {
    let s = match length {
        Some(len) => format!("{}({})", data_type, len),
        None => data_type.to_string(),
    };
    let s = if options.contains("unsigned") || options.contains("zerofill") {
        let suffix = if options.contains("zerofill") { " zerofill" } else if options.contains("unsigned") { " unsigned" } else { "" };
        format!("{}{}", s, suffix)
    } else {
        s
    };
    asql_types::parse_column_type(&s)
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct CreateTableParams {
    table: String,
    columns: Vec<ColumnDefJson>,
    #[serde(default)]
    engine: Option<String>,
    #[serde(default)]
    collation: Option<String>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn create_table_sql(json: &str) -> String {
    let params: CreateTableParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut cb = CreateTableBuilder::new().table(&params.table);
    for c in &params.columns {
        let col = ColumnDef {
            name: c.name.clone(),
            col_type: to_column_type(&c.data_type, &c.length, &c.options),
            nullable: Some(c.nullable),
            default_value: c.default.clone(),
            comment: c.comment.clone(),
            extra: asql_types::ColumnExtra {
                auto_increment: c.auto_increment,
                on_update: c.on_update,
            },
            collation: None,
            key: None,
        };
        cb = cb.column(col);
    }
    if let Some(engine) = &params.engine {
        cb = cb.engine(engine);
    }
    if let Some(collation) = &params.collation {
        cb = cb.collation(collation);
    }
    cb.build(&*dialect)
}

// ── AlterTable ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AlterColumnDefJson {
    name: String,
    #[serde(rename = "type")]
    data_type: String,
    #[serde(default)]
    length: Option<String>,
    #[serde(default)]
    options: String,
    #[serde(default = "default_true")]
    nullable: bool,
    #[serde(default)]
    auto_increment: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    on_update: bool,
}

fn to_column_def(c: &AlterColumnDefJson) -> ColumnDef {
    ColumnDef {
        name: c.name.clone(),
        col_type: to_column_type(&c.data_type, &c.length, &c.options),
        nullable: Some(c.nullable),
        default_value: c.default.clone(),
        comment: c.comment.clone(),
        extra: asql_types::ColumnExtra {
            auto_increment: c.auto_increment,
            on_update: c.on_update,
        },
        collation: None,
        key: None,
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AlterActionJson {
    AddColumn { column: AlterColumnDefJson },
    ModifyColumn { column: AlterColumnDefJson },
    ChangeColumn { old_name: String, column: AlterColumnDefJson },
    DropColumn { name: String },
    RenameTable { new_name: String },
    AddPrimaryKey { columns: Vec<String> },
    DropPrimaryKey,
    AddIndex { name: String, index_type: String, columns: Vec<String> },
    DropIndex { name: String },
}

#[derive(Deserialize)]
struct AlterTableParams {
    table: String,
    actions: Vec<AlterActionJson>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

fn parse_index_type(s: &str) -> IndexType {
    match s {
        "Primary" => IndexType::Primary,
        "Unique" => IndexType::Unique,
        "Fulltext" => IndexType::Fulltext,
        "Spatial" => IndexType::Spatial,
        _ => IndexType::Index,
    }
}

#[wasm_bindgen]
pub fn alter_table_sql(json: &str) -> String {
    let params: AlterTableParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut ab = AlterTableBuilder::new().table(&params.table);
    for action in &params.actions {
        ab = match action {
            AlterActionJson::AddColumn { column } => ab.add_column(to_column_def(column)),
            AlterActionJson::ModifyColumn { column } => ab.modify_column(to_column_def(column)),
            AlterActionJson::ChangeColumn { old_name, column } => {
                ab.change_column(old_name, to_column_def(column))
            }
            AlterActionJson::DropColumn { name } => ab.drop_column(name),
            AlterActionJson::RenameTable { new_name } => ab.rename_table(new_name),
            AlterActionJson::AddPrimaryKey { columns } => {
                let cols: Vec<(&str, Option<usize>)> =
                    columns.iter().map(|c| (c.as_str(), None)).collect();
                ab.add_primary_key(cols)
            }
            AlterActionJson::DropPrimaryKey => ab.drop_primary_key(),
            AlterActionJson::AddIndex {
                name,
                index_type,
                columns,
            } => {
                let it = parse_index_type(index_type);
                let cols: Vec<(&str, Option<usize>)> =
                    columns.iter().map(|c| (c.as_str(), None)).collect();
                ab.add_index(name, it, cols)
            }
            AlterActionJson::DropIndex { name } => ab.drop_index(name),
        };
    }
    let stmts = ab.build(&*dialect);
    serde_json::to_string(&stmts).unwrap()
}

// ── CreateIndex ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateIndexParams {
    table: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    index_type: String,
    columns: Vec<String>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn create_index_sql(json: &str) -> String {
    let params: CreateIndexParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut ib = IndexBuilder::new().on(&params.table);
    if let Some(name) = &params.name {
        ib = ib.name(name);
    }
    ib = ib.index_type(parse_index_type(&params.index_type));
    for c in &params.columns {
        ib = ib.column(c, None);
    }
    ib.build(&*dialect)
}

// ── CreateDatabase ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateDatabaseParams {
    name: String,
    #[serde(default)]
    collation: Option<String>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn create_database_sql(json: &str) -> String {
    let params: CreateDatabaseParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut db = DatabaseBuilder::create(&params.name);
    if let Some(collation) = &params.collation {
        db = db.collation(collation);
    }
    db.build(&*dialect)
}

// ── Drop ───────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn drop_table_sql(table: &str, if_exists: bool, dialect: &str) -> String {
    let dialect = get_dialect(dialect);
    let mut db = DropBuilder::new(DropTarget::Table).name(table);
    if if_exists {
        db = db.if_exists();
    }
    db.build(&*dialect)
}

/// Returns SQL to drop an index from a table for the given dialect.
///
/// - MySQL: `DROP INDEX \`name\` ON \`table\``
/// - PostgreSQL: `DROP INDEX [IF EXISTS] "name"`
#[wasm_bindgen]
pub fn drop_index_sql(name: &str, table: &str, if_exists: bool, dialect: &str) -> String {
    let dialect = get_dialect(dialect);
    let mut db = DropBuilder::new(DropTarget::Index).name(name).on(table);
    if if_exists {
        db = db.if_exists();
    }
    db.build(&*dialect)
}

/// Returns SQL to drop a database for the given dialect.
///
/// - MySQL: `DROP DATABASE [IF EXISTS] \`name\``
/// - PostgreSQL: `DROP DATABASE [IF EXISTS] "name"`
#[wasm_bindgen]
pub fn drop_database_sql(name: &str, if_exists: bool, dialect: &str) -> String {
    let dialect = get_dialect(dialect);
    let mut db = DropBuilder::new(DropTarget::Database).name(name);
    if if_exists {
        db = db.if_exists();
    }
    db.build(&*dialect)
}

/// Returns SQL to truncate a table for the given dialect.
///
/// - MySQL: `TRUNCATE TABLE \`name\``
/// - PostgreSQL: `TRUNCATE TABLE "name"`
/// - SQLite: `DELETE FROM "name"` (SQLite has no TRUNCATE)
#[wasm_bindgen]
pub fn truncate_sql(table: &str, dialect: &str) -> String {
    let d = get_dialect(dialect);
    DropBuilder::new(DropTarget::Table).name(table).build_truncate(&*d)
}

// ── CreateUser ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateUserParams {
    username: String,
    #[serde(default)]
    host: Option<String>,
    password: String,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn create_user_sql(json: &str) -> String {
    let params: CreateUserParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let mut ub = UserBuilder::new().create_user(&params.username);
    if let Some(host) = &params.host {
        ub = ub.host(host);
    }
    ub = ub.identified_by(&params.password);
    ub.build(&*dialect)
}

// ── Grant ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GrantParams {
    privileges: Vec<String>,
    on: String,
    to: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default = "default_dialect")]
    dialect: String,
}

#[wasm_bindgen]
pub fn grant_sql(json: &str) -> String {
    let params: GrantParams = serde_json::from_str(json).unwrap();
    let dialect = get_dialect(&params.dialect);
    let priv_refs: Vec<&str> = params.privileges.iter().map(String::as_str).collect();
    let mut gb = GrantBuilder::new()
        .grant(priv_refs)
        .on(&params.on)
        .to(&params.to);
    if let Some(host) = &params.host {
        gb = gb.host(host);
    }
    gb.build(&*dialect)
}
