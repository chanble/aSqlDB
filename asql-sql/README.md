# asql-sql

A SQL auto-completion and syntax tokenization library written in Rust.

## Features

- Context-aware SQL completion based on cursor position
- Multiple database dialect support (MySQL, PostgreSQL, SQLite3)
- Table alias resolution (`SELECT u. FROM users AS u`)
- Subquery support (nested `SELECT` completion)
- DDL statement completion (`CREATE TABLE`, `ALTER TABLE`, `DROP TABLE`)
- Column type and constraint suggestions
- Pre-loaded database schema for offline completion
- SQL tokenizer for syntax highlighting (UI-agnostic)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
asql-sql = "0.1.0"
```

### Auto-Completion

```rust
use asql_sql::{get_suggestions, DatabaseType, DatabaseSchema, Table, Column};

let mut schema = DatabaseSchema::new();
schema.add_table(Table {
    name: "users".into(),
    columns: vec![
        Column { name: "id".into(), data_type: "INTEGER".into() },
        Column { name: "name".into(), data_type: "VARCHAR".into() },
        Column { name: "email".into(), data_type: "VARCHAR".into() },
    ],
});

let suggestions = get_suggestions(
    DatabaseType::MySQL,
    &schema,
    "SELECT * FROM ",
    14,
);

for s in &suggestions {
    println!("{:?}: {}", s.kind, s.text);
}
```

### Table Alias

```rust
let suggestions = get_suggestions(
    DatabaseType::MySQL,
    &schema,
    "SELECT u. FROM users AS u",
    10, // cursor after "u."
);
// Returns: id, name, email (columns of users table)
```

### Subquery

```rust
let suggestions = get_suggestions(
    DatabaseType::MySQL,
    &schema,
    "SELECT * FROM users WHERE id IN (SELECT ",
    40,
);
// Returns: columns, functions, keywords for inner SELECT
```

### DDL

```rust
let suggestions = get_suggestions(
    DatabaseType::MySQL,
    &schema,
    "CREATE TABLE foo (id ",
    21,
);
// Returns: INTEGER, VARCHAR, TEXT, PRIMARY, NOT, NULL, ...

let suggestions = get_suggestions(
    DatabaseType::MySQL,
    &schema,
    "ALTER TABLE users ADD ",
    22,
);
// Returns: column types like INTEGER, VARCHAR, etc.
```

### Syntax Tokenization (for highlighting)

```rust
use asql_sql::{tokenize_sql, TokenKind, Span};

let sql = "SELECT u.name, COUNT(o.id) FROM users u";
let spans: Vec<Span> = tokenize_sql(sql);

for span in &spans {
    let text = span.text(sql);
    match span.kind {
        TokenKind::Keyword   => print!("[KW:{}]", text),    // UI: blue
        TokenKind::Function  => print!("[FN:{}]", text),    // UI: yellow
        TokenKind::String    => print!("[ST:{}]", text),    // UI: green
        TokenKind::Number    => print!("[NU:{}]", text),    // UI: cyan
        TokenKind::Identifier => print!("[ID:{}]", text),   // UI: white
        TokenKind::Operator  => print!("[OP:{}]", text),    // UI: gray
        TokenKind::Punctuation => print!("[PU:{}]", text),  // UI: gray
        TokenKind::Comment   => print!("[CM:{}]", text),    // UI: dark green
        TokenKind::Variable  => print!("[VA:{}]", text),    // UI: magenta
    }
}
```

Output:

```
[KW:SELECT] [ID:u][PU:.][ID:name][PU:,] [FN:COUNT][PU:][OP:*][PU:] [KW:FROM] [ID:users] [ID:u]
```

## API

### `get_suggestions`

```rust
pub fn get_suggestions(
    db_type: DatabaseType,
    schema: &DatabaseSchema,
    sql: &str,
    cursor_position: usize,
) -> Vec<Suggestion>
```

### `tokenize_sql`

```rust
pub fn tokenize_sql(sql: &str) -> Vec<Span>
```

Returns `Vec<Span>` where each span has `kind: TokenKind`, `start: usize`, `end: usize`. The UI layer maps `TokenKind` to colors.

### `DatabaseType`

```rust
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite3,
}
```

### `DatabaseSchema`

```rust
let mut schema = DatabaseSchema::new();
schema.add_table(Table {
    name: "users".into(),
    columns: vec![
        Column { name: "id".into(), data_type: "INTEGER".into() },
    ],
});
schema.table_names();           // ["users"]
schema.get_table("users");     // Option<&Table>
```

### `Suggestion`

```rust
pub struct Suggestion {
    pub text: String,
    pub kind: SuggestionKind, // Keyword | Table | Column | Function | Alias
}
```

### `Span` & `TokenKind`

```rust
pub struct Span {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

pub enum TokenKind {
    Keyword,     // SELECT, FROM, WHERE
    Function,    // COUNT, SUM, MAX
    String,      // 'hello', "name", `table`
    Number,      // 42, 3.14
    Identifier,  // users, id, email
    Operator,    // =, <>, >=, ||, +, -, *, /, %
    Punctuation, // (, ), ,, ;, .
    Comment,     // -- line, /* block */, # hash
    Variable,    // @row_num
}
```

## Project Structure

```
src/
├── lib.rs          # Module exports
├── schema.rs       # DatabaseSchema / Table / Column
├── db_type.rs      # DatabaseType + dialect keywords / functions / types
├── suggestion.rs   # Suggestion / SuggestionKind
├── parser.rs       # CompletionRequest / context detection / alias extraction
├── completer.rs    # Completer / get_suggestions
└── tokenizer.rs    # tokenize_sql / Span / TokenKind
```

## License

MIT
