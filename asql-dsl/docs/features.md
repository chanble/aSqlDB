# asql-dsl 功能规划

## 项目定位

将当前散落在前端 Vue（TypeScript）和后端 Rust 中的 SQL 字符串拼接逻辑，抽离为一个独立的、类型安全的 SQL 领域语言构建库。
支持 **MySQL / PostgreSQL / SQLite** 三种方言。

---

## 一、DQL — 数据查询

| # | 功能 | 来源 | 说明 |
|---|------|------|------|
| 1 | **SELECT 构建** | `TableDataPage.vue:179` `buildSql()` | 列选择、函数包裹、WHERE 条件、ORDER BY、LIMIT/OFFSET |
| 2 | **WHERE 子句** | `TableDataPage.vue:163` `buildWhereClause()` | 多条件 AND 连接，支持 = > < >= <= != LIKE NOT LIKE |
| 3 | **SELECT COUNT** | `TableDataPage.vue:129` | `SELECT COUNT(*) as cnt FROM tbl WHERE ...` |
| 4 | **选择列 + 聚合函数** | `TableDataPage.vue:184-187` | 支持 COUNT/SUM/AVG/MIN/MAX/CONCAT/LENGTH/UPPER/LOWER/TRIM/SUBSTRING/REPLACE/DATE/NOW/YEAR/MONTH/DAY/IFNULL/COALESCE/CAST/CONVERT/GROUP_CONCAT/ROUND/FLOOR/CEIL/ABS/CURRENT_TIMESTAMP/FROM_UNIXTIME/UNIX_TIMESTAMP 等 20+ 函数包裹列 |

---

## 二、DML — 数据操作

| # | 功能 | 来源 | 说明 |
|---|------|------|------|
| 5 | **INSERT** | `TableDataPage.vue:342` `InsertDataPage.vue:78` | `INSERT INTO tbl (cols) VALUES (vals)`，自动引用与转义 |
| 6 | **UPDATE** | `TableDataPage.vue:361` | `UPDATE tbl SET col=val WHERE ... LIMIT 1`，支持对比新旧值生成 SET 子句 |
| 7 | **DELETE** | `TableDataPage.vue:279` | `DELETE FROM tbl WHERE ... LIMIT 1` |
| 8 | **导出 INSERT** | `TableDataPage.vue:393` `query.rs:76` | 将行数据批量生成 INSERT 语句，前后端均有实现 |

---

## 三、DDL — 表定义

| # | 功能 | 来源 | 说明 |
|---|------|------|------|
| 9 | **CREATE TABLE** | `CreateTablePage.vue:108` | 列定义（类型/长度/options/NULL/AI/默认值/注释），ENGINE、COLLATE |
| 10 | **CREATE TABLE 内联索引** | `CreateTablePage.vue:108` | 复合主键、UNIQUE KEY / INDEX / FULLTEXT INDEX / SPATIAL INDEX 内联定义，支持前缀长度 |
| 11 | **ALTER TABLE ADD COLUMN** | `AlterTablePage.vue:168` `TableStructurePage.vue:168` | 与 CREATE TABLE 列定义同构 |
| 12 | **ALTER TABLE MODIFY COLUMN** | `AlterTablePage.vue:165` | 修改列定义 |
| 13 | **ALTER TABLE CHANGE COLUMN** | `AlterTablePage.vue:162` | 重命名列 |
| 14 | **ALTER TABLE DROP COLUMN** | `TableStructurePage.vue:111` | `ALTER TABLE tbl DROP COLUMN col` |
| 15 | **RENAME TABLE** | `AlterTablePage.vue:144` | `RENAME TABLE old TO new` |
| 16 | **DROP TABLE** | `AlterTablePage.vue:198` `TableStructurePage.vue:182` | `DROP TABLE tbl` |
| 17 | **TRUNCATE TABLE** | `TableStructurePage.vue:194` | `TRUNCATE TABLE tbl` |

---

## 四、DDL — 数据库

| # | 功能 | 说明 |
|---|------|------|
| 18 | **CREATE DATABASE** | `CREATE DATABASE [IF NOT EXISTS] name [CHARACTER SET cs] [COLLATE collation] [ENCRYPTION 'Y/N']` |
| 19 | **ALTER DATABASE** | `ALTER DATABASE name [CHARACTER SET cs] [COLLATE collation] [ENCRYPTION 'Y/N']` |
| 20 | **DROP DATABASE** | `DROP DATABASE [IF EXISTS] name` |

---

## 五、DDL — 索引

| # | 功能 | 来源 | 说明 |
|---|------|------|------|
| 21 | **CREATE INDEX** | `IndexesPage.vue:100-104` `TableStructurePage.vue:212` | INDEX / UNIQUE / PRIMARY KEY / FULLTEXT / SPATIAL |
| 22 | **前缀索引** | `IndexesPage.vue:95` | `col(length)` 前缀索引 |
| 23 | **DROP INDEX** | `IndexesPage.vue:125` `TableStructurePage.vue:225` | `DROP INDEX name ON tbl` |
| 24 | **ALTER TABLE ADD PRIMARY KEY** | `IndexesPage.vue:100` | `ALTER TABLE t ADD PRIMARY KEY (cols)` |
| 25 | **ALTER TABLE DROP PRIMARY KEY** | (隐含) | `ALTER TABLE t DROP PRIMARY KEY` |
| 26 | **ALTER TABLE ADD INDEX** | (MySQL 等价形式) | `ALTER TABLE t ADD [UNIQUE\|FULLTEXT\|SPATIAL] INDEX name (cols)` |
| 27 | **ALTER TABLE DROP INDEX** | `IndexesPage.vue:125` `TableStructurePage.vue:225` | MySQL: `ALTER TABLE t DROP INDEX name` / PG: `DROP INDEX name` |

---

## 六、Schema 反射（Introspection SQL）

| # | 功能 | 来源 | 说明 |
|---|------|------|------|
| 28 | **SHOW COLUMNS** | 多文件 + `schema.rs:188` | MySQL: `SHOW FULL COLUMNS`（含 Comment 列）；PG: `information_schema.columns` + `pg_description`（含 collation、列注释、自增标识）；SQLite: `pragma_table_info` |
| 29 | **SHOW TABLES** | `schema.rs:56` | MySQL / pg_tables / sqlite_master |
| 30 | **SHOW DATABASES** | `schema.rs:24` | MySQL SCHEMATA / pg_database |
| 31 | **USE DATABASE** | `api.rs` / 前端 | `USE dbname` |
| 32 | **SHOW CREATE TABLE** | `query.rs:48` | MySQL / sqlite_master |
| 33 | **SHOW INDEX** | `api.rs:203` | MySQL SHOW INDEX / pg_indexes / PRAGMA index_list |
| 34 | **information_schema.TABLES** | `schema.rs:82` | 查询表注释、ENGINE、COLLATION |
| 35 | **TABLE COUNT** | 新增 | `SELECT COUNT(*)` 表总数 |
| 36 | **TABLE SIZES** | 新增 | MySQL: `DATA_LENGTH + INDEX_LENGTH`; PG: `pg_total_relation_size`; SQLite: `dbstat.pgsize` |

---

## 六、跨方言支持

每个功能都必须根据 `DatabaseType` 生成不同方言的 SQL：

| 维度 | MySQL | PostgreSQL | SQLite |
|------|-------|------------|--------|
| 标识符引用 | `` `backtick` `` | `"double quote"` | 无 / `"quote"` |
| 自增 | `AUTO_INCREMENT` | `SERIAL` / `GENERATED AS IDENTITY` | `AUTOINCREMENT` |
| 类型系统 | `INT(11)` / `VARCHAR(255)` | `INTEGER` / `VARCHAR` | `INTEGER` / `TEXT` |
| 时间函数 | `NOW()` | `CURRENT_TIMESTAMP` | `CURRENT_TIMESTAMP` |
| SHOW TABLES | `SHOW TABLES` | `SELECT tablename FROM pg_tables` | `SELECT name FROM sqlite_master` |
| SHOW COLUMNS | `SHOW COLUMNS FROM tbl` | `SELECT ... FROM information_schema.columns` | `SELECT ... FROM pragma_table_info` |
| SHOW INDEX | `SHOW INDEX FROM tbl` | `SELECT ... FROM pg_indexes` | `PRAGMA index_list(tbl)` |
| SHOW CREATE TABLE | `SHOW CREATE TABLE tbl` | (查询 information_schema) | `SELECT sql FROM sqlite_master` |
| USE DATABASE | `USE dbname` | `SET search_path TO dbname` | 不支持（SQLite 单文件） |
| CREATE DATABASE | `CREATE DATABASE [IF NOT EXISTS] name DEFAULT CHARACTER SET cs COLLATE collation ENCRYPTION 'Y/N'` | `CREATE DATABASE name ENCODING 'cs' LC_COLLATE 'collation'` | 不支持 |
| DROP DATABASE | `DROP DATABASE [IF EXISTS] name` | `DROP DATABASE [IF EXISTS] name` | 不支持 |
| LIMIT | `LIMIT n OFFSET m` | `LIMIT n OFFSET m` | `LIMIT n OFFSET m` |
| DELETE/UPDATE LIMIT | 支持 `LIMIT 1` | 不支持（需 USING 或子查询） | 支持 `LIMIT 1` |
| CREATE TABLE 内联索引 | `PRIMARY KEY` / `INDEX` / `UNIQUE KEY` / `FULLTEXT INDEX` / `SPATIAL INDEX` | `PRIMARY KEY` / `UNIQUE`（CONSTRAINT 形式） | `PRIMARY KEY` |
| ALTER TABLE ADD INDEX | `ALTER TABLE ... ADD INDEX/UNIQUE/FULLTEXT/SPATIAL` | 不支持，回退为 `CREATE INDEX` | 不支持，回退为 `CREATE INDEX` |
| ALTER TABLE DROP INDEX | `ALTER TABLE ... DROP INDEX name` | `DROP INDEX name` | `DROP INDEX name` |
| ALTER TABLE PRIMARY KEY | `ADD PRIMARY KEY` / `DROP PRIMARY KEY` | `ADD PRIMARY KEY` / `DROP CONSTRAINT tablename_pkey` | `ADD PRIMARY KEY` |

---

## 七、设计原则

1. **纯构建器模式** — 无副作用，输入结构化数据，输出 SQL 字符串
2. **类型安全** — Rust `enum` + `struct` 约束，防止 SQL 注入（参数化而非字符串拼接）
3. **Zero-copy 友好** — 尽量使用 `Cow<str>` / 引用
4. **无运行时依赖** — 轻量，只依赖 `serde`（可选）
5. **可编译为 Wasm** — 以便前后端复用同一套 SQL 生成逻辑
6. **可测试** — 每个构建器方法都应有快照测试，覆盖三种方言

---

## 八、模块组织（建议）

```
asql-dsl/
├── Cargo.toml
├── docs/
│   └── features.md
├── src/
│   ├── lib.rs
│   ├── dialect/         # 方言抽象
│   │   ├── mod.rs
│   │   ├── mysql.rs
│   │   ├── postgres.rs
│   │   └── sqlite.rs
│   ├── dql/             # 查询构建
│   │   ├── mod.rs
│   │   ├── select.rs
│   │   └── where.rs
│   ├── dml/             # 数据操作
│   │   ├── mod.rs
│   │   ├── insert.rs
│   │   ├── update.rs
│   │   └── delete.rs
│   ├── ddl/             # 表/索引/触发器
│   │   ├── mod.rs
│   │   ├── create_table.rs
│   │   ├── alter_table.rs
│   │   ├── index.rs
│   │   ├── trigger.rs
│   │   └── drop.rs
│   └── introspection/   # Schema 反射
│       ├── mod.rs
│       ├── columns.rs
│       ├── tables.rs
│       ├── databases.rs
│       ├── indexes.rs
│       └── triggers.rs
```
