#  asql Core

**asql Core** 是一个基于 Rust 开发的高性能、通用数据库操作核心库。它旨在简化多数据库环境下的连接管理与 SQL 执行流程，为上层应用提供统一的接口来操作 MySQL、PostgreSQL 和 SQLite。

通过封装 `sqlx` 的 `Any` 驱动特性，asql Core 实现了数据库类型的动态适配，让你能够编写与具体数据库无关的代码，同时保持异步、类型安全和零开销抽象的优势。

---

###  核心特性

- **多数据库支持**：一套代码同时支持 **MySQL**、**PostgreSQL** 和 **SQLite**。
- **动态驱动适配**：基于 `sqlx::any`，在运行时根据连接字符串自动识别并加载对应的数据库驱动。
- **异步优先**：完全基于 `tokio` 异步运行时构建，提供高并发处理能力。
- **统一结果集**：屏蔽不同数据库的返回差异，提供标准化的 JSON 兼容结果集（`DbSuccessResult`）。
- **连接池管理**：内置轻量级连接池管理器，支持连接的增删查改及生命周期管理。
- **类型安全**：利用 Rust 的强类型系统，在编译期捕获尽可能多的错误。

---

###  快速开始

#### 1. 添加依赖

在你的 `Cargo.toml` 中添加 `asql-core`：

```toml
[dependencies]
asql-core = "0.1.0"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

#### 2. 初始化与连接

```rust
use asql_core::db_manager::DbManager;
use asql_core::executor::execute_sql;
use asql_core::result::DbSuccessResult;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 安装默认驱动（只需执行一次）
    sqlx::any::install_default_drivers();

    // 2. 创建管理器实例
    let manager = DbManager::new();

    // 3. 添加数据库连接
    // 支持 mysql://, postgres://, sqlite:// 等协议头
    manager.add_connection(
        "my_db".to_string(), 
        "sqlite::memory:?mode=rw&cache=shared".to_string()
    ).await?;

    // 4. 获取连接池
    let pool = manager.get_pool("my_db").await
        .ok_or("Connection not found")?;

    // 5. 执行 SQL
    // 建表
    execute_sql("CREATE TABLE users (id INTEGER, name TEXT)", &pool).await?;
    
    // 插入
    execute_sql("INSERT INTO users VALUES (1, 'Alice')", &pool).await?;

    // 6. 查询数据
    match execute_sql("SELECT * FROM users", &pool).await? {
        DbSuccessResult::Select(result) => {
            println!("查询成功，行数: {}", result.rows.len());
            for row in result.rows {
                println!("{:?}", row);
            }
        },
        _ => println!("非查询操作"),
    }

    Ok(())
}
```

---

###  核心模块说明

#### 1. 连接管理 (`DbManager`)

`DbManager` 是线程安全的（`Arc<RwLock<...>>`），负责维护数据库连接的注册表。

- **`add_connection(name, url)`**: 注册一个新的数据库连接。URL 格式决定了数据库类型（如 `mysql://...`）。
- **`get_pool(name)`**: 获取指定名称的连接池句柄。该句柄是轻量级的引用计数指针，可在线程间安全传递。
- **`list_connections()`**: 获取当前所有已注册的连接信息。

#### 2. SQL 执行器 (`execute_sql`)

这是库的核心函数，它接收 SQL 字符串和连接池，自动判断 SQL 类型并返回相应的结果结构。

- **输入**: `sql: &str`, `pool: &AnyPool`
- **输出**: `Result<DbSuccessResult, DbError>`

**智能路由逻辑：**
- `SELECT`, `SHOW`, `DESCRIBE` → 返回 `DbSuccessResult::Select`
- `INSERT`, `UPDATE`, `DELETE` → 返回 `DbSuccessResult::Modify`
- `CREATE`, `DROP`, `ALTER` → 返回 `DbSuccessResult::Schema`
- `GRANT`, `REVOKE` → 返回 `DbSuccessResult::Privilege`

#### 3. 结果集标准化

asql Core 将不同数据库的原始返回数据统一转换为易于序列化的结构：

- **SetResult (查询)**:
    - `columns`: 列名与类型信息。
    - `rows`: `Vec<HashMap<String, JsonValue>>`，每一行都是一个 JSON 对象，保留了数字、字符串、布尔值和 Null 的类型信息。
- **ModifyResult (修改)**:
    - `rows_affected`: 受影响行数。
    - `last_insert_id`: 最后插入的自增 ID。
- **SchemaResult (结构变更)**:
    - `success`: 操作是否成功。
    - `message`: 操作摘要。

---

### ️ 技术栈

- **Runtime**: `tokio` (Full features)
- **Database Driver**: `sqlx` (0.8+, with `runtime-tokio-native-tls`, `any`, `mysql`, `postgres`, `sqlite`)
- **Serialization**: `serde` & `serde_json`
- **Error Handling**: `thiserror`

---

###  注意事项

- **SQLite 内存数据库**: 如果使用 `sqlite::memory:`，请务必在连接字符串中添加 `?mode=rw&cache=shared`，以确保连接池中的不同连接能共享同一个内存数据库实例，否则会出现 `no such table` 错误。
- **驱动初始化**: 在程序启动时，必须调用 `sqlx::any::install_default_drivers()`，否则无法识别数据库协议。
- **类型转换**: 在 `Select` 结果中，所有数据均尝试转换为 `serde_json::Value`。对于不支持的复杂二进制类型，可能会转换为字符串表示。

---

###  许可证

MIT License
