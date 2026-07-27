<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Vue%203-4FC08D?style=for-the-badge&logo=vue.js&logoColor=white" alt="Vue 3">
  <img src="https://img.shields.io/badge/MySQL-4479A1?style=for-the-badge&logo=mysql&logoColor=white" alt="MySQL">
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=for-the-badge&logo=postgresql&logoColor=white" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite">
</p>

<h1 align="center">aSqlDB</h1>

<p align="center">
  <strong>用 Rust 编写的统一 Web 多数据库管理工具</strong>
  <br>
  通过一个直观的 Web 界面管理 MySQL、PostgreSQL 和 SQLite 数据库。
</p>

<p align="center">
  <a href="#功能特性">功能特性</a> •
  <a href="#快速开始">快速开始</a> •
  <a href="#架构">架构</a> •
  <a href="#API-概览">API 概览</a> •
  <a href="#开发指南">开发指南</a>
</p>

<hr>

<p align="center">
  <a href="README.md">English</a>
</p>

---

## 概述

aSqlDB是一个开源数据库管理工具，提供**统一的 Web 界面**来管理 MySQL、PostgreSQL 和 SQLite 数据库。本软件设计灵感来自 [Adminer](https://www.adminer.org/)，致力于打造轻量、直观的数据库管理体验，从头使用 Rust 构建，追求高性能和高可靠性。

后端全部使用 **Rust**（Axum + SQLx）编写，前端使用 **Vue 3** + **CodeMirror 6**，提供响应式、IDE 般的查询体验，支持语法高亮和自动补全。

## 功能特性

### 多数据库统一管理
一个界面管理所有主流数据库：

| 数据库 | 支持状态 | 说明 |
|--------|---------|------|
| **MySQL** | 完全支持 | 连接池、方言感知 SQL 生成、完整内省 |
| **PostgreSQL** | 完全支持 | 连接池、方言感知 SQL 生成、完整内省 |
| **SQLite** | 完全支持 | 基于文件的连接、功能完整 |

### SQL 查询编辑器
- 执行单条或批量 SQL 语句（分号分隔）
- **智能自动补全** — 通过 WebSocket 提供上下文感知的表、列、关键字和函数建议
- 基于 CodeMirror 6 的语法高亮
- 多语句执行，每条语句独立报告成功/失败

### 数据库与模式管理
- **数据库**：创建、修改、删除、切换当前数据库
- **表**：创建、修改、删除、清空；查看 CREATE TABLE DDL；查看列和索引
- **数据**：浏览、过滤、排序、分页；插入、更新、删除行
- **索引**：查看、创建（INDEX/UNIQUE/FULLTEXT/SPATIAL/PRIMARY KEY）、删除

### 数据导出
- 支持 **SQL**、**CSV**、**TSV**、**分号分隔 CSV** 格式导出表或整个数据库

### 用户与权限管理
- 创建、修改、删除、重命名用户
- 授权和撤销权限，支持预定义角色配置（超级管理员、DBA、读写、只读、DDL）

### 服务器管理
- 查看和终止服务器进程
- 查看服务器变量和状态
- 支持表的 REPAIR、OPTIMIZE、ANALYZE、CHECK 维护操作

### 桌面应用
- 提供 **Tauri v2** 桌面应用，原生体验

## 快速开始

### 前置要求
- Rust（2021 edition）
- Node.js（用于前端构建）

### 从源码运行

#### 开发模式

```bash
# 克隆仓库
git clone https://github.com/anomalyco/aSqlDB.git
cd aSqlDB

# 首先启动后端服务器
cargo run -p asql-web

# 然后在另一个终端启动前端开发服务器
cd asql-web/frontend
npm install
npm run dev
```

服务器默认启动在 `http://0.0.0.0:5173`，在浏览器中打开即可开始使用。

#### 构建运行

```bash
# 克隆仓库
git clone https://github.com/anomalyco/aSqlDB.git
cd aSqlDB

# 构建发布版本（自动构建前端）
cargo build --release -p asql-web

# 运行
./target/release/asql-web
```

### 使用桌面应用

```bash
cargo run -p asql-tauri
```

### 命令行选项

| 选项 | 环境变量 | 默认值 | 说明 |
|------|---------|--------|------|
| `-p`, `--port` | `ASQL_PORT` | `5580` | 监听端口 |
| `-c`, `--config-dir` | `ASQL_CONFIG_DIR` | `~/.aSqlDB` | 配置目录 |

### 添加数据库连接
1. 打开 Web 界面
2. 点击"添加连接"
3. 填写连接信息：
   - **名称**：连接的自定义名称
   - **URL**：数据库 URL（例如 `mysql://user:pass@host:3306/db`、`postgres://user:pass@host:5432/db` 或 SQLite 文件路径）
   - **系统**：MySQL / PostgreSQL / SQLite
4. 测试连接并保存

> **警告：** 账号和密码以明文保存在配置文件中（`~/.aSqlDB/connections.json`），请注意保护该文件的访问权限。

## 架构

aSqlDB 由 7 个 Rust crate 组成的工作空间：

```
aSqlDB/
├── asql-types/        # 共享类型定义和数据库元数据预设
├── asql-dsl/          # 类型安全的 SQL DSL 构建器（支持 WASM，JavaScript 绑定为 @asql-dsl/wasm）
├── asql-sql/          # 基于 sqlparser 的 SQL 自动补全引擎
├── asql-core/         # 核心数据库执行层（连接池、查询分发、结果统一）
├── asql-query/        # 统一查询协议 — 所有操作的单一类型化 API 契约
├── asql-backend/      # 业务逻辑层（配置、连接持久化、BackendHandle）
├── asql-web/          # Web 应用 — Axum HTTP 服务器 + Vue 3 SPA 前端
└── asql-tauri/        # Tauri v2 桌面应用封装
```

### 数据流

```
浏览器 (Vue 3 SPA)
    │
    ▼ HTTP / WebSocket
asql-web (Axum REST API)
    │
    ▼
asql-backend (业务逻辑)
    │
    ▼
asql-query (统一查询协议)
    │
    ├──▶ asql-dsl (SQL 构建器)
    │
    ▼
asql-core (数据库执行)
    │
    ├──▶ MySQL (sqlx)
    ├──▶ PostgreSQL (sqlx)
    └──▶ SQLite (sqlx)
```

## API 概览

REST API 位于 `/api/` 下，涵盖以下分类：

| 分类 | 接口 |
|------|------|
| **连接管理** | 增删改查、测试、Ping、执行查询 |
| **数据库** | 创建、修改、删除、切换 |
| **表** | 增删改查、元数据、列信息、索引 |
| **数据** | ��询（过滤/排序/分页）、插入、更新、删除、计数 |
| **导出** | 表或数据库导出（SQL/CSV/TSV） |
| **用户** | 增删改查、授权、撤销权限 |
| **服务器管理** | 进程、变量、状态、服务器版本 |
| **自动补全** | 基于 WebSocket 的 SQL 智能补全 |

## 开发指南

### 前置要求
- Rust 1.80+
- Node.js 18+

### 开发模式运行

```bash
# 启动后端（自动重载）
cargo watch -x 'run -p asql-web'

# 另开终端，启动前端开发服务器
cd asql-web/frontend
npm run dev
```

### 运行测试

```bash
cargo test -p asql-web
cargo test -p asql-core
cargo test -p asql-query
```

### WASM 构建（供 JavaScript 使用）

```bash
cd asql-dsl
wasm-pack build --target bundler
# 发布为 @asql-dsl/wasm npm 包
```

## 技术栈

### 后端
| 技术 | 用途 |
|------|------|
| **Rust** | 核心语言 |
| **Axum 0.8** | HTTP 框架（支持 WebSocket） |
| **SQLx 0.8** | 数据库驱动（MySQL、PostgreSQL、SQLite） |
| **sqlparser 0.61** | SQL 解析（用于自动补全） |
| **Tokio** | 异步运行时 |
| **Clap 4** | CLI 参数解析 |

### 前端
| 技术 | 用途 |
|------|------|
| **Vue 3** | UI 框架 |
| **TypeScript** | 前端语言 |
| **CodeMirror 6** | SQL 编辑器（语法高亮） |
| **Bulma** | CSS 框架 |
| **vue-i18n** | 国际化 |
| **Vite** | 构建工具 |

## 许可证

本项目使用 MIT 许可证。
