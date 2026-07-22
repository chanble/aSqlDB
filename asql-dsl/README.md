# aqua-dsl

[![crates.io](https://img.shields.io/crates/v/aqua-dsl.svg)](https://crates.io/crates/aqua-dsl)
[![npm](https://img.shields.io/npm/v/@aqua-dsl/wasm)](https://www.npmjs.com/package/@aqua-dsl/wasm)

Type-safe SQL DSL builder for MySQL, PostgreSQL, and SQLite. Generates dialect-aware SQL strings from structured builder calls.

## Design

- Pure SQL string generation — no DB connection, no execution
- Input structured data, output SQL string
- Dialect-aware: MySQL backtick quoting, PG/SQLite double-quote quoting
- Compilable to Wasm (zero runtime deps)

## Installation

### Rust (crates.io)

```toml
[dependencies]
aqua-dsl = "0.1"
```

### JavaScript / TypeScript (npm)

```bash
npm install @aqua-dsl/wasm
```

## Usage

### Rust

```rust
use aqua_dsl::*;

let mysql = MySql;
let pg = PostgreSql;
let sqlite = Sqlite;
```

### DQL — SELECT

```rust
// Basic SELECT
let sql = SelectBuilder::new()
    .column("id")
    .column("name")
    .from("users")
    .where_("status", "=", "active")
    .order_by("created_at", true)  // true = DESC
    .limit(10)
    .offset(5)
    .build(&mysql);

// SELECT COUNT(*)
let count = SelectBuilder::new()
    .from("users")
    .where_("age", ">", "18")
    .build_count(&mysql);

// Complex WHERE with grouping
let sql = SelectBuilder::new()
    .from("orders")
    .where_("status", "=", "pending")
    .and_group(|w| w
        .and("amount", ">", "100")
        .or_where("priority", "=", "high")
    )
    .build(&mysql);
```

### DML — INSERT / UPDATE / DELETE

```rust
// INSERT
let sql = InsertBuilder::new()
    .into("users")
    .column("name")
    .column("email")
    .row(vec!["Alice", "alice@test.com"])
    .row(vec!["Bob", "bob@test.com"])
    .build(&mysql);

// UPDATE
let sql = UpdateBuilder::new()
    .table("users")
    .set("status", "active")
    .where_("id", "=", "1")
    .build(&mysql);

// DELETE
let sql = DeleteBuilder::new()
    .from("logs")
    .where_("created_at", "<", "2024-01-01")
    .limit(1000)
    .build(&mysql);
```

### DDL — CREATE / ALTER / DROP

```rust
// CREATE TABLE
let sql = CreateTableBuilder::new()
    .table("users")
    .column(ColumnDef {
        name: "id".into(),
        data_type: "INT".into(),
        length: None,
        options: None,
        nullable: false,
        auto_increment: true,
        default_value: None,
        comment: None,
    })
    .column(ColumnDef {
        name: "name".into(),
        data_type: "VARCHAR".into(),
        length: Some("255".into()),
        ..Default()
    })
    .engine("InnoDB")
    .build(&mysql);

// ALTER TABLE
let sql = AlterTableBuilder::new()
    .table("users")
    .add_column(...)
    .modify_column(...)
    .drop_column("temp_col")
    .add_index("idx_name", IndexType::Index, vec![("name", None)])
    .using(IndexMethod::BTree)
    .build(&mysql);

// DROP
let sql = DropBuilder::new(DropTarget::Table)
    .name("users")
    .if_exists()
    .build(&mysql);

let sql = DropBuilder::new(DropTarget::Table)
    .name("logs")
    .build_truncate(&mysql);
```

### User & Grant Management

```rust
// CREATE USER
let sql = UserBuilder::new()
    .create_user("app_user")
    .host("localhost")
    .identified_by("secret")
    .build(&mysql);
// CREATE USER 'app_user'@'localhost' IDENTIFIED BY 'secret'

// GRANT with preset role
let sql = GrantBuilder::new()
    .role(GrantRole::ReadWrite)
    .on("mydb.*")
    .to("app_user")
    .build(&mysql);

// Manual GRANT
let sql = GrantBuilder::new()
    .grant(vec!["SELECT", "INSERT"])
    .on("mydb.*")
    .to("app_user")
    .host("localhost")
    .with_grant_option()
    .build(&mysql);

// REVOKE
let sql = GrantBuilder::new()
    .revoke(vec!["DELETE"])
    .on("mydb.*")
    .from("app_user")
    .build(&mysql);
```

### Index Management

```rust
let sql = IndexBuilder::new()
    .on("users")
    .name("idx_email")
    .index_type(IndexType::Unique)
    .column("email", None)
    .using(IndexMethod::Hash)
    .build(&mysql);

let sql = IndexBuilder::new()
    .on("users")
    .name("idx_name")
    .build_drop(&mysql);
```

### Introspection Queries

```rust
// Users
UsersIntrospection::list_users(&mysql);
UsersIntrospection::user_info(&mysql, "app_user", Some("localhost"));

// Server
ServerIntrospection::process_list(&mysql);
ServerIntrospection::variables(&mysql);
ServerIntrospection::status(&mysql);
ServerIntrospection::kill_process(&mysql, &["123", "456"]);

// Databases
DatabasesIntrospection::list_databases(&mysql);
DatabasesIntrospection::use_database(&mysql, "mydb");

// Tables
TablesIntrospection::list_tables(&mysql, None, None, false);

// Columns
ColumnsIntrospection::show_columns(&mysql, "users", None);

// Indexes
IndexesIntrospection::list_indexes(&mysql, "users");
```

### Grant Roles

| Role | Privileges | WITH GRANT OPTION |
|------|-----------|-------------------|
| `SuperAdmin` | ALL PRIVILEGES ON `*.*` | Yes |
| `DBA` | ALL PRIVILEGES | Yes |
| `ReadWrite` | SELECT, INSERT, UPDATE, DELETE, EXECUTE | No |
| `ReadOnly` | SELECT, SHOW VIEW | No |
| `DDL` | CREATE, ALTER, DROP, INDEX | No |

### JavaScript / TypeScript

```typescript
import init, {
  buildSelect, buildCount,
  buildInsert, buildUpdate, buildDelete,
  buildCreateTable, buildAlterTable,
  buildCreateIndex, buildCreateDatabase,
  buildDropTable,
  buildCreateUser, buildGrant,
} from '@aqua-dsl/wasm'

// 必须初始化 Wasm
await init()

// SELECT
const sql = buildSelect({
  table: 'users',
  columns: [{ name: 'id' }, { name: 'name', func: 'COUNT' }],
  where: [{ column: 'status', operator: '=', value: 'active' }],
  orderBy: [{ column: 'created_at', desc: true }],
  limit: 50,
  offset: 0,
})

// INSERT
const sql = buildInsert({
  table: 'users',
  columns: ['name', 'email'],
  values: [['Alice', 'alice@test.com']],
})

// CREATE TABLE
const sql = buildCreateTable({
  table: 'users',
  columns: [
    { name: 'id', type: 'INT', length: '11', options: 'unsigned', nullable: false, auto_increment: true },
    { name: 'name', type: 'VARCHAR', length: '255', nullable: false },
  ],
  engine: 'InnoDB',
  collation: 'utf8mb4_unicode_ci',
})
```

## Cargo Features

| Feature | Description | Default |
|---------|-------------|---------|
| `wasm` | Enables WebAssembly compilation (wasm-bindgen + serde_json) | No |

## License

MIT
