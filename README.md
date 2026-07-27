<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Vue%203-4FC08D?style=for-the-badge&logo=vue.js&logoColor=white" alt="Vue 3">
  <img src="https://img.shields.io/badge/MySQL-4479A1?style=for-the-badge&logo=mysql&logoColor=white" alt="MySQL">
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?style=for-the-badge&logo=postgresql&logoColor=white" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/SQLite-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite">
</p>

<h1 align="center">aSqlDB</h1>

<p align="center">
  <strong>A unified, web-based multi-database management tool written in Rust.</strong>
  <br>
  Manage MySQL, PostgreSQL, and SQLite databases through a single, intuitive web interface.
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#screenshot">Screenshot</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#api-overview">API Overview</a> •
  <a href="#development">Development</a>
</p>

<hr>

<p align="center">
  <a href="README.zh-CN.md">中文文档</a>
</p>

---

## Overview

aSqlDB is an open-source database management tool that provides a **unified web interface** for administering MySQL, PostgreSQL, and SQLite databases. Inspired by [Adminer](https://www.adminer.org/), it aims to deliver a lightweight, intuitive database management experience — but built from the ground up in Rust for performance and reliability.

The entire backend is written in **Rust** (using Axum + SQLx), with a **Vue 3** + **CodeMirror 6** frontend that delivers a responsive, IDE-like query experience with syntax highlighting and auto-completion.

## Features

### Universal Database Support
Manage all three major database types from one interface:

| Database | Full Support | Details |
|----------|-------------|---------|
| **MySQL** | Yes | Connection pools, dialect-aware SQL generation, full introspection |
| **PostgreSQL** | Yes | Connection pools, dialect-aware SQL generation, full introspection |
| **SQLite** | Yes | File-based connections, full feature parity |

### SQL Query Editor
- Execute single or batched SQL statements (semicolon-separated)
- **Auto-completion** via WebSocket — context-aware suggestions for tables, columns, keywords, and functions
- Syntax highlighting powered by CodeMirror 6
- Multi-statement execution with per-statement success/error reporting

### Database & Schema Management
- **Databases**: List, create, alter, drop; switch current database
- **Tables**: List, create, alter, drop, truncate; view CREATE TABLE DDL; inspect columns and indexes
- **Data**: Browse, filter, sort, paginate; insert, update, delete rows
- **Indexing**: List, create (INDEX/UNIQUE/FULLTEXT/SPATIAL/PRIMARY KEY), drop indexes

### Data Export
- Export tables or entire databases in **SQL**, **CSV**, **TSV**, or **semicolon-delimited CSV** format

### User & Privilege Management
- Create, alter, drop, rename users
- Grant and revoke privileges with predefined role profiles (SuperAdmin, DBA, ReadWrite, ReadOnly, DDL)

### Server Administration
- View and kill server processes
- Inspect server variables and status
- Support for REPAIR, OPTIMIZE, ANALYZE, and CHECK table maintenance operations

### Desktop App
- Available as a **Tauri v2** desktop application for a native experience

## Quick Start

### Prerequisites
- Rust (edition 2021)
- Node.js (for frontend build)

### From Source

#### Development Mode

```bash
# Clone the repository
git clone https://github.com/chanble/aSqlDB.git
# git clone https://gitee.com/chanble/aSqlDB.git
cd aSqlDB

# Start the backend server first
cargo run -p asql-web

# Then in another terminal, start the frontend dev server
cd asql-web/frontend
npm install
npm run dev
```

The server starts on `http://0.0.0.0:5173` by default. Open it in your browser to get started.

#### Build & Run

```bash
# Clone the repository
git clone https://github.com/chanble/aSqlDB.git
# git clone https://gitee.com/chanble/aSqlDB.git
cd aSqlDB

# Build the release binary (automatically builds the frontend)
cargo build --release -p asql-web

# Run
./target/release/asql-web
```

### Using the Desktop App

```bash
cargo run -p asql-tauri
```

### CLI Options

| Option | Env Variable | Default | Description |
|--------|-------------|---------|-------------|
| `-p`, `--port` | `ASQL_PORT` | `5580` | Port to listen on |
| `-c`, `--config-dir` | `ASQL_CONFIG_DIR` | `~/.aSqlDB` | Configuration directory |

### Adding a Connection
1. Open the web interface
2. Click "Add Connection"
3. Enter the connection details:
   - **Name**: A friendly name for the connection
   - **URL**: Database URL (e.g., `mysql://user:pass@host:3306/db`, `postgres://user:pass@host:5432/db`, or a SQLite file path)
   - **System**: MySQL / PostgreSQL / SQLite
4. Test the connection and save

> **Warning:** Connection credentials (username and password) are stored in plaintext in the config file (`~/.aSqlDB/connections.json`). Protect access to this file accordingly.

## Architecture

aSqlDB is organized as a Rust workspace with 7 crates:

```
aSqlDB/
├── asql-types/        # Shared type definitions and database metadata presets
├── asql-dsl/          # Type-safe SQL DSL builder (WASM-compatible, JavaScript bindings available as @asql-dsl/wasm)
├── asql-sql/          # SQL auto-completion engine based on sqlparser
├── asql-core/         # Core database execution layer (connection pools, query dispatch, result unification)
├── asql-query/        # Unified query protocol — the single typed API contract for all operations
├── asql-backend/      # Business logic layer (config, connection persistence, BackendHandle)
├── asql-web/          # Web application — Axum HTTP server + Vue 3 SPA frontend
└── asql-tauri/        # Tauri v2 desktop application wrapper
```

### Data Flow

```
Browser (Vue 3 SPA)
    │
    ▼ HTTP / WebSocket
asql-web (Axum REST API)
    │
    ▼
asql-backend (Business Logic)
    │
    ▼
asql-query (Unified Query Protocol)
    │
    ├──▶ asql-dsl (SQL Builder)
    │
    ▼
asql-core (Database Execution)
    │
    ├──▶ MySQL (sqlx)
    ├──▶ PostgreSQL (sqlx)
    └──▶ SQLite (sqlx)
```

## API Overview

The REST API is available at `/api/` and covers:

| Category | Endpoints |
|----------|-----------|
| **Connections** | CRUD, test, ping, query execution |
| **Databases** | List, create, alter, drop, switch |
| **Tables** | CRUD, metadata, columns, indexes |
| **Data** | Select (filtered/sorted/paginated), insert, update, delete, count |
| **Export** | Table or database export (SQL/CSV/TSV) |
| **Users** | CRUD, grant, revoke privileges |
| **Admin** | Processes, variables, status, server version |
| **Completion** | WebSocket-based SQL auto-completion |

## Development

### Prerequisites
- Rust 1.80+
- Node.js 18+

### Running in Development Mode

```bash
# Start the backend with auto-reload
cargo watch -x 'run -p asql-web'

# In another terminal, start the frontend dev server
cd asql-web/frontend
npm run dev
```

### Running Tests

```bash
cargo test -p asql-web
cargo test -p asql-core
cargo test -p asql-query
```

### WASM Build (for JavaScript consumers)

```bash
cd asql-dsl
wasm-pack build --target bundler
# Published as @asql-dsl/wasm on npm
```

## Technology Stack

### Backend
| Technology | Purpose |
|------------|---------|
| **Rust** | Core language |
| **Axum 0.8** | HTTP framework (WebSocket support) |
| **SQLx 0.8** | Database driver (MySQL, PostgreSQL, SQLite) |
| **sqlparser 0.61** | SQL parsing for auto-completion |
| **Tokio** | Async runtime |
| **Clap 4** | CLI argument parsing |

### Frontend
| Technology | Purpose |
|------------|---------|
| **Vue 3** | UI framework |
| **TypeScript** | Frontend language |
| **CodeMirror 6** | SQL editor with syntax highlighting |
| **Bulma** | CSS framework |
| **vue-i18n** | Internationalization |
| **Vite** | Build tool |

## License

This project is licensed under the MIT License.
