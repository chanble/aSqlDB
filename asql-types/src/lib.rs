use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumIter,
)]
pub enum DatabaseType {
    MySql,
    #[strum(disabled)]
    Postgres,
    #[strum(disabled)]
    Sqlite,
}

impl DatabaseType {
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            DatabaseType::Postgres
        } else if url.starts_with("mysql://") {
            DatabaseType::MySql
        } else {
            DatabaseType::Sqlite
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DatabaseType::MySql => "MySQL",
            DatabaseType::Postgres => "PostgreSQL",
            DatabaseType::Sqlite => "SQLite",
        }
    }

    pub fn value(&self) -> &'static str {
        match self {
            DatabaseType::MySql => "mysql",
            DatabaseType::Postgres => "pgsql",
            DatabaseType::Sqlite => "sqlite",
        }
    }

    pub fn to_url(&self) -> &'static str {
        match self {
            DatabaseType::MySql => "mysql://",
            DatabaseType::Postgres => "postgres://",
            DatabaseType::Sqlite => "",
        }
    }

    pub fn keywords(&self) -> Vec<&'static str> {
        let mut kw: Vec<&'static str> = COMMON_KEYWORDS.to_vec();
        match self {
            DatabaseType::MySql => kw.extend_from_slice(MYSQL_KEYWORDS),
            DatabaseType::Postgres => kw.extend_from_slice(POSTGRESQL_KEYWORDS),
            DatabaseType::Sqlite => kw.extend_from_slice(SQLITE_KEYWORDS),
        }
        kw
    }

    pub fn functions(&self) -> &'static [&'static str] {
        match self {
            DatabaseType::MySql => &MYSQL_FUNCTIONS,
            DatabaseType::Postgres => &POSTGRESQL_FUNCTIONS,
            DatabaseType::Sqlite => &SQLITE_FUNCTIONS,
        }
    }

    pub fn column_types(&self) -> &'static [&'static str] {
        match self {
            DatabaseType::MySql => &MYSQL_TYPES,
            DatabaseType::Postgres => &POSTGRESQL_TYPES,
            DatabaseType::Sqlite => &SQLITE_TYPES,
        }
    }

    pub fn ddl_keywords(&self) -> &'static [&'static str] {
        match self {
            DatabaseType::MySql => &MYSQL_DDL,
            DatabaseType::Postgres => &POSTGRESQL_DDL,
            DatabaseType::Sqlite => &SQLITE_DDL,
        }
    }

    pub fn constraint_keywords(&self) -> &'static [&'static str] {
        &CONSTRAINT_KEYWORDS
    }

    pub fn highlight_keywords(&self) -> Vec<&'static str> {
        let mut kw: Vec<&'static str> = COMMON_KEYWORDS.to_vec();
        match self {
            DatabaseType::MySql => {
                kw.extend_from_slice(MYSQL_KEYWORDS);
                kw.extend_from_slice(MYSQL_DDL);
                kw.extend_from_slice(MYSQL_TYPES);
            }
            DatabaseType::Postgres => {
                kw.extend_from_slice(POSTGRESQL_KEYWORDS);
                kw.extend_from_slice(POSTGRESQL_DDL);
                kw.extend_from_slice(POSTGRESQL_TYPES);
            }
            DatabaseType::Sqlite => {
                kw.extend_from_slice(SQLITE_KEYWORDS);
                kw.extend_from_slice(SQLITE_DDL);
                kw.extend_from_slice(SQLITE_TYPES);
            }
        }
        kw.sort_unstable();
        kw.dedup();
        kw
    }

    pub fn highlight_functions(&self) -> Vec<&'static str> {
        let mut fns: Vec<&'static str> = COMMON_FUNCTIONS.to_vec();
        match self {
            DatabaseType::MySql => fns.extend_from_slice(MYSQL_FUNCTIONS),
            DatabaseType::Postgres => fns.extend_from_slice(POSTGRESQL_FUNCTIONS),
            DatabaseType::Sqlite => fns.extend_from_slice(SQLITE_FUNCTIONS),
        }
        fns.sort_unstable();
        fns.dedup();
        fns
    }

    // ─── Preset data accessors ────────────────────────────────────────

    pub fn data_types_info(&self) -> &'static [DataType] {
        match self {
            DatabaseType::MySql => MYSQL_TYPES_INFO,
            DatabaseType::Postgres => POSTGRES_TYPES_INFO,
            DatabaseType::Sqlite => SQLITE_TYPES_INFO,
        }
    }

    pub fn functions_info(&self) -> &'static [DbFunction] {
        match self {
            DatabaseType::MySql => MYSQL_FUNCTIONS_INFO,
            DatabaseType::Postgres => POSTGRES_FUNCTIONS_INFO,
            DatabaseType::Sqlite => SQLITE_FUNCTIONS_INFO,
        }
    }

    pub fn engines(&self) -> &'static [Engine] {
        match self {
            DatabaseType::MySql => MYSQL_ENGINES,
            _ => &[],
        }
    }

    pub fn charsets(&self) -> &'static [Charset] {
        match self {
            DatabaseType::MySql => MYSQL_CHARSETS,
            DatabaseType::Postgres => POSTGRES_CHARSETS,
            _ => &[],
        }
    }

    pub fn privileges(&self) -> &'static [Privilege] {
        match self {
            DatabaseType::MySql => MYSQL_PRIVILEGES,
            DatabaseType::Postgres => POSTGRES_PRIVILEGES,
            DatabaseType::Sqlite => SQLITE_PRIVILEGES,
        }
    }

    pub fn sql_modes(&self) -> &'static [SqlMode] {
        match self {
            DatabaseType::MySql => MYSQL_SQL_MODES,
            _ => &[],
        }
    }
}

impl fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseType::MySql => write!(f, "MySQL"),
            DatabaseType::Postgres => write!(f, "PostgreSQL"),
            DatabaseType::Sqlite => write!(f, "SQLite"),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Preset data models & per-database metadata
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTypeCategory {
    Numeric,
    String,
    DateTime,
    Json,
    Spatial,
}

#[derive(Debug, Clone, Copy)]
pub struct DataType {
    pub name: &'static str,
    pub category: DataTypeCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FunctionCategory {
    Aggregate,
    Numeric,
    String,
    DateTime,
    Conversion,
    Window,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DbFunction {
    pub name: &'static str,
    pub category: FunctionCategory,
}

#[derive(Debug, Clone, Copy)]
pub struct Engine {
    pub name: &'static str,
    pub desc: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Charset {
    pub name: &'static str,
    pub collations: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeScope {
    Global,
    Database,
    Table,
    Column,
    Routine,
    Replication,
}

#[derive(Debug, Clone, Copy)]
pub struct Privilege {
    pub name: &'static str,
    pub scopes: &'static [PrivilegeScope],
}

#[derive(Debug, Clone, Copy)]
pub struct SqlMode {
    pub name: &'static str,
    pub desc: &'static str,
}

// ─── ColumnType ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_width: Option<u32>,
    #[serde(default)]
    pub unsigned: bool,
    #[serde(default)]
    pub zerofill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    #[serde(default)]
    pub unsigned: bool,
    #[serde(default)]
    pub zerofill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumType {
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColumnExtra {
    #[serde(default)]
    pub auto_increment: bool,
    #[serde(default)]
    pub on_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColumnType {
    #[serde(rename = "INT")]
    Int(IntType),
    #[serde(rename = "TINYINT")]
    TinyInt(IntType),
    #[serde(rename = "SMALLINT")]
    SmallInt(IntType),
    #[serde(rename = "MEDIUMINT")]
    MediumInt(IntType),
    #[serde(rename = "BIGINT")]
    BigInt(IntType),
    #[serde(rename = "INTEGER")]
    Integer(IntType),
    #[serde(rename = "FLOAT")]
    Float(FloatType),
    #[serde(rename = "DOUBLE")]
    Double(FloatType),
    #[serde(rename = "DECIMAL")]
    Decimal(FloatType),
    #[serde(rename = "NUMERIC")]
    Numeric(FloatType),
    #[serde(rename = "BIT")]
    Bit(Option<u32>),
    #[serde(rename = "CHAR")]
    Char(StringType),
    #[serde(rename = "VARCHAR")]
    Varchar(StringType),
    #[serde(rename = "TINYTEXT")]
    TinyText,
    #[serde(rename = "TEXT")]
    Text,
    #[serde(rename = "MEDIUMTEXT")]
    MediumText,
    #[serde(rename = "LONGTEXT")]
    LongText,
    #[serde(rename = "BINARY")]
    Binary(StringType),
    #[serde(rename = "VARBINARY")]
    Varbinary(StringType),
    #[serde(rename = "TINYBLOB")]
    TinyBlob,
    #[serde(rename = "BLOB")]
    Blob,
    #[serde(rename = "MEDIUMBLOB")]
    MediumBlob,
    #[serde(rename = "LONGBLOB")]
    LongBlob,
    #[serde(rename = "ENUM")]
    Enum(EnumType),
    #[serde(rename = "SET")]
    Set(EnumType),
    #[serde(rename = "DATE")]
    Date,
    #[serde(rename = "DATETIME")]
    DateTime,
    #[serde(rename = "TIMESTAMP")]
    Timestamp,
    #[serde(rename = "TIME")]
    Time,
    #[serde(rename = "YEAR")]
    Year,
    #[serde(rename = "BOOLEAN")]
    Boolean,
    #[serde(rename = "BOOL")]
    Bool,
    #[serde(rename = "JSON")]
    Json,
}

impl ColumnType {
    pub fn to_sql(&self) -> String {
        let name = self.type_name();
        let param = self.param_string();
        let unsigned = self.is_unsigned();
        let zerofill = self.is_zerofill();

        let mut sql = name.to_string();
        if let Some(p) = &param {
            sql.push('(');
            sql.push_str(p);
            sql.push(')');
        }
        if zerofill {
            sql.push_str(" ZEROFILL");
        } else if unsigned {
            sql.push_str(" UNSIGNED");
        }
        sql
    }

    fn type_name(&self) -> &str {
        match self {
            ColumnType::Int(_) => "INT",
            ColumnType::TinyInt(_) => "TINYINT",
            ColumnType::SmallInt(_) => "SMALLINT",
            ColumnType::MediumInt(_) => "MEDIUMINT",
            ColumnType::BigInt(_) => "BIGINT",
            ColumnType::Integer(_) => "INTEGER",
            ColumnType::Float(_) => "FLOAT",
            ColumnType::Double(_) => "DOUBLE",
            ColumnType::Decimal(_) => "DECIMAL",
            ColumnType::Numeric(_) => "NUMERIC",
            ColumnType::Bit(_) => "BIT",
            ColumnType::Char(_) => "CHAR",
            ColumnType::Varchar(_) => "VARCHAR",
            ColumnType::TinyText => "TINYTEXT",
            ColumnType::Text => "TEXT",
            ColumnType::MediumText => "MEDIUMTEXT",
            ColumnType::LongText => "LONGTEXT",
            ColumnType::Binary(_) => "BINARY",
            ColumnType::Varbinary(_) => "VARBINARY",
            ColumnType::TinyBlob => "TINYBLOB",
            ColumnType::Blob => "BLOB",
            ColumnType::MediumBlob => "MEDIUMBLOB",
            ColumnType::LongBlob => "LONGBLOB",
            ColumnType::Enum(_) => "ENUM",
            ColumnType::Set(_) => "SET",
            ColumnType::Date => "DATE",
            ColumnType::DateTime => "DATETIME",
            ColumnType::Timestamp => "TIMESTAMP",
            ColumnType::Time => "TIME",
            ColumnType::Year => "YEAR",
            ColumnType::Boolean => "BOOLEAN",
            ColumnType::Bool => "BOOL",
            ColumnType::Json => "JSON",
        }
    }

    fn param_string(&self) -> Option<String> {
        match self {
            ColumnType::Int(p) | ColumnType::TinyInt(p) | ColumnType::SmallInt(p)
                | ColumnType::MediumInt(p) | ColumnType::BigInt(p) | ColumnType::Integer(p) => {
                p.display_width.map(|w| w.to_string())
            }
            ColumnType::Float(p) | ColumnType::Double(p)
                | ColumnType::Decimal(p) | ColumnType::Numeric(p) => {
                match (p.precision, p.scale) {
                    (Some(prec), Some(s)) => Some(format!("{prec},{s}")),
                    (Some(prec), None) => Some(prec.to_string()),
                    (None, _) => None,
                }
            }
            ColumnType::Bit(p) => p.as_ref().map(|v| v.to_string()),
            ColumnType::Char(p) | ColumnType::Varchar(p)
                | ColumnType::Binary(p) | ColumnType::Varbinary(p) => {
                p.length.map(|l| l.to_string())
            }
            ColumnType::Enum(p) | ColumnType::Set(p) => {
                if p.values.is_empty() {
                    None
                } else {
                    let q: Vec<_> = p.values.iter().map(|v| format!("'{v}'")).collect();
                    Some(q.join(","))
                }
            }
            _ => None,
        }
    }

    fn is_unsigned(&self) -> bool {
        match self {
            ColumnType::Int(p) | ColumnType::TinyInt(p) | ColumnType::SmallInt(p)
                | ColumnType::MediumInt(p) | ColumnType::BigInt(p) | ColumnType::Integer(p) => p.unsigned,
            ColumnType::Float(p) | ColumnType::Double(p)
                | ColumnType::Decimal(p) | ColumnType::Numeric(p) => p.unsigned,
            _ => false,
        }
    }

    fn is_zerofill(&self) -> bool {
        match self {
            ColumnType::Int(p) | ColumnType::TinyInt(p) | ColumnType::SmallInt(p)
                | ColumnType::MediumInt(p) | ColumnType::BigInt(p) | ColumnType::Integer(p) => p.zerofill,
            ColumnType::Float(p) | ColumnType::Double(p)
                | ColumnType::Decimal(p) | ColumnType::Numeric(p) => p.zerofill,
            _ => false,
        }
    }
}

pub fn parse_column_type(s: &str) -> ColumnType {
    let s = s.trim();
    if s.is_empty() {
        return ColumnType::Varchar(StringType { length: None });
    }

    let paren_pos = s.find('(');
    let space_pos = s.find(' ');
    let base_end = match (paren_pos, space_pos) {
        (Some(p), Some(sp)) => p.min(sp),
        (Some(p), None) => p,
        (None, Some(sp)) => sp,
        (None, None) => s.len(),
    };
    let base = &s[..base_end];

    let content = paren_pos.map(|p| {
        let start = p + 1;
        if let Some(end) = s[start..].rfind(')') {
            let inner = &s[start..start + end];
            // strip trailing spaces inside parens
            inner.trim()
        } else {
            ""
        }
    });

    let trailing = match paren_pos {
        Some(p) => {
            let after_paren = p + 1 + content.unwrap_or("").len() + 1; // +1 for ')'
            if after_paren < s.len() { s[after_paren..].trim() } else { "" }
        }
        None => {
            if let Some(sp) = space_pos { s[sp..].trim() } else { "" }
        }
    };

    let unsigned = trailing.contains("unsigned");
    let zerofill = trailing.contains("zerofill");

    let base_upper = base.to_uppercase();
    match base_upper.as_str() {
        "INT" | "INTEGER" => {
            let display_width = content.and_then(|c| c.parse().ok());
            let p = IntType { display_width, unsigned, zerofill };
            if base_upper == "INTEGER" { ColumnType::Integer(p) } else { ColumnType::Int(p) }
        }
        "TINYINT" => ColumnType::TinyInt(IntType {
            display_width: content.and_then(|c| c.parse().ok()),
            unsigned, zerofill,
        }),
        "SMALLINT" => ColumnType::SmallInt(IntType {
            display_width: content.and_then(|c| c.parse().ok()),
            unsigned, zerofill,
        }),
        "MEDIUMINT" => ColumnType::MediumInt(IntType {
            display_width: content.and_then(|c| c.parse().ok()),
            unsigned, zerofill,
        }),
        "BIGINT" => ColumnType::BigInt(IntType {
            display_width: content.and_then(|c| c.parse().ok()),
            unsigned, zerofill,
        }),
        "FLOAT" | "DOUBLE" | "DECIMAL" | "NUMERIC" => {
            let (precision, scale) = content.map(|c| {
                let parts: Vec<&str> = c.split(',').collect();
                (
                    parts.first().and_then(|p| p.trim().parse().ok()),
                    parts.get(1).and_then(|s| s.trim().parse().ok()),
                )
            }).unwrap_or((None, None));

            let p = FloatType { precision, scale, unsigned, zerofill };
            match base_upper.as_str() {
                "FLOAT" => ColumnType::Float(p),
                "DOUBLE" => ColumnType::Double(p),
                "NUMERIC" => ColumnType::Numeric(p),
                _ => ColumnType::Decimal(p),
            }
        }
        "BIT" => {
            ColumnType::Bit(content.and_then(|c| c.parse().ok()))
        }
        "CHAR" => ColumnType::Char(StringType {
            length: content.and_then(|c| c.parse().ok()),
        }),
        "VARCHAR" => ColumnType::Varchar(StringType {
            length: content.and_then(|c| c.parse().ok()),
        }),
        "BINARY" => ColumnType::Binary(StringType {
            length: content.and_then(|c| c.parse().ok()),
        }),
        "VARBINARY" => ColumnType::Varbinary(StringType {
            length: content.and_then(|c| c.parse().ok()),
        }),
        "TINYTEXT" => ColumnType::TinyText,
        "TEXT" => ColumnType::Text,
        "MEDIUMTEXT" => ColumnType::MediumText,
        "LONGTEXT" => ColumnType::LongText,
        "TINYBLOB" => ColumnType::TinyBlob,
        "BLOB" => ColumnType::Blob,
        "MEDIUMBLOB" => ColumnType::MediumBlob,
        "LONGBLOB" => ColumnType::LongBlob,
        "ENUM" | "SET" => {
            let values = content.map(parse_enum_values).unwrap_or_default();
            if base_upper == "SET" {
                ColumnType::Set(EnumType { values })
            } else {
                ColumnType::Enum(EnumType { values })
            }
        }
        "DATE" => ColumnType::Date,
        "DATETIME" => ColumnType::DateTime,
        "TIMESTAMP" => ColumnType::Timestamp,
        "TIME" => ColumnType::Time,
        "YEAR" => ColumnType::Year,
        "BOOLEAN" => ColumnType::Boolean,
        "BOOL" => ColumnType::Bool,
        "JSON" => ColumnType::Json,
        _ => ColumnType::Varchar(StringType {
            length: content.and_then(|c| c.parse().ok()),
        }),
    }
}

fn parse_enum_values(s: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '\'' {
            chars.next(); // consume opening quote
            let mut val = String::new();
            while let Some(&inner) = chars.peek() {
                if inner == '\'' {
                    chars.next(); // consume closing quote
                    break;
                } else {
                    val.push(inner);
                    chars.next();
                }
            }
            values.push(val);
        } else if c == ',' || c == ' ' {
            chars.next();
        } else {
            // unquoted value (non-standard but handle gracefully)
            let mut val = String::new();
            while let Some(&inner) = chars.peek() {
                if inner == ',' || inner == '\'' {
                    break;
                }
                val.push(inner);
                chars.next();
            }
            values.push(val.trim().to_string());
        }
    }
    values
}

// ═══════════════════════════════════════════════════════════
// Keyword / completion string constants
// ═══════════════════════════════════════════════════════════

const COMMON_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE",
    "CREATE", "DROP", "ALTER", "TABLE", "INDEX", "VIEW", "JOIN",
    "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "AND",
    "OR", "NOT", "IN", "IS", "NULL", "LIKE", "BETWEEN", "EXISTS",
    "GROUP", "BY", "ORDER", "ASC", "DESC", "HAVING", "LIMIT", "OFFSET",
    "UNION", "ALL", "AS", "DISTINCT", "SET", "VALUES", "CASE", "WHEN",
    "THEN", "ELSE", "END", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
    "UNIQUE", "CHECK", "DEFAULT", "BEGIN", "COMMIT", "ROLLBACK",
    "TRANSACTION", "ADD", "COLUMN", "MODIFY", "RENAME", "TO",
    "CONSTRAINT", "IF", "EXISTS", "EXCEPT", "INTERSECT", "WITH",
    "RECURSIVE", "OVER", "PARTITION", "FETCH", "NEXT", "ROW", "ROWS",
    "COMMENT",
];

const MYSQL_KEYWORDS: &[&str] = &[
    "SHOW", "DESCRIBE", "EXPLAIN", "USE", "REPLACE", "DUPLICATE",
    "ENGINE", "CHARSET", "AUTO_INCREMENT", "DATABASES", "TABLES",
];

const POSTGRESQL_KEYWORDS: &[&str] = &[
    "EXPLAIN", "ANALYZE", "RETURNING", "CONFLICT", "UPSERT", "LATERAL",
    "WINDOW", "MATERIALIZED", "SEQUENCE", "SERIAL",
];

const SQLITE_KEYWORDS: &[&str] = &[
    "REPLACE", "CONFLICT", "AUTOINCREMENT", "ROWID", "VACUUM", "PRAGMA",
];

const CONSTRAINT_KEYWORDS: &[&str] = &[
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK",
    "DEFAULT", "NOT", "NULL", "AUTO_INCREMENT", "AUTOINCREMENT",
    "CONSTRAINT", "ON", "DELETE", "CASCADE", "SET", "NO", "ACTION",
    "RESTRICT",
];

const MYSQL_DDL: &[&str] = &[
    "CREATE", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
    "TEMPORARY", "IF", "NOT", "EXISTS", "ALTER", "ADD", "COLUMN",
    "MODIFY", "CHANGE", "DROP", "RENAME", "TO", "AFTER", "FIRST",
    "ENGINE", "CHARSET", "COLLATE", "AUTO_INCREMENT", "PRIMARY", "KEY",
    "UNIQUE", "FOREIGN", "REFERENCES", "ON", "DELETE", "UPDATE",
    "CASCADE", "RESTRICT", "SET", "NULL", "NO", "ACTION", "DEFAULT",
    "CONSTRAINT", "UNSIGNED", "ZEROFILL", "ENUM",
];

const POSTGRESQL_DDL: &[&str] = &[
    "CREATE", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
    "TEMPORARY", "TEMP", "IF", "NOT", "EXISTS", "UNLOGGED", "ALTER",
    "ADD", "COLUMN", "DROP", "RENAME", "TO", "TYPE", "USING", "SET",
    "NOT", "NULL", "DEFAULT", "CHECK", "UNIQUE", "PRIMARY", "KEY",
    "FOREIGN", "REFERENCES", "ON", "DELETE", "UPDATE", "CASCADE",
    "RESTRICT", "SET", "NULL", "NO", "ACTION", "DEFERRABLE",
    "INITIALLY", "DEFERRED", "IMMEDIATE", "CONSTRAINT", "SERIAL",
    "BIGSERIAL", "MATERIALIZED", "REPLACE", "RETURNING", "EXCLUSION",
    "USING", "WITH",
];

const SQLITE_DDL: &[&str] = &[
    "CREATE", "TABLE", "INDEX", "VIEW", "TRIGGER", "TEMPORARY", "TEMP",
    "IF", "NOT", "EXISTS", "ALTER", "ADD", "COLUMN", "DROP", "RENAME",
    "TO", "PRIMARY", "KEY", "UNIQUE", "CHECK", "DEFAULT", "FOREIGN",
    "REFERENCES", "ON", "DELETE", "CASCADE", "RESTRICT", "SET", "NULL",
    "NO", "ACTION", "AUTOINCREMENT", "CONSTRAINT", "WITHOUT", "ROWID",
];

const MYSQL_TYPES: &[&str] = &[
    "INT", "TINYINT", "SMALLINT", "MEDIUMINT", "BIGINT", "INTEGER",
    "FLOAT", "DOUBLE", "DECIMAL", "NUMERIC", "CHAR", "VARCHAR", "TEXT",
    "TINYTEXT", "MEDIUMTEXT", "LONGTEXT", "BLOB", "TINYBLOB",
    "MEDIUMBLOB", "LONGBLOB", "DATE", "DATETIME", "TIMESTAMP", "TIME",
    "YEAR", "BOOLEAN", "BOOL", "ENUM", "SET", "JSON", "BINARY",
    "VARBINARY",
];

const POSTGRESQL_TYPES: &[&str] = &[
    "SMALLINT", "INTEGER", "BIGINT", "SERIAL", "BIGSERIAL",
    "SMALLSERIAL", "REAL", "DOUBLE", "PRECISION", "NUMERIC", "DECIMAL",
    "CHAR", "VARCHAR", "TEXT", "BYTEA", "DATE", "TIME", "TIMESTAMP",
    "TIMESTAMPTZ", "INTERVAL", "BOOLEAN", "BOOL", "JSON", "JSONB",
    "UUID", "INET", "CIDR", "MACADDR", "ARRAY", "POINT", "LINE",
    "LSEG", "BOX", "PATH", "POLYGON", "CIRCLE",
];

const SQLITE_TYPES: &[&str] = &[
    "INTEGER", "INT", "REAL", "TEXT", "BLOB", "NUMERIC", "BOOLEAN",
    "BOOL", "VARCHAR", "CHAR", "DATETIME", "DATE", "TIMESTAMP", "FLOAT",
    "DOUBLE", "DECIMAL",
];

const COMMON_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "COALESCE", "CAST", "NULLIF",
    "ABS", "ROUND",
];

const MYSQL_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "CONCAT", "LENGTH", "UPPER",
    "LOWER", "TRIM", "SUBSTRING", "REPLACE", "DATE", "NOW", "YEAR",
    "MONTH", "DAY", "IFNULL", "COALESCE", "CAST", "CONVERT",
    "GROUP_CONCAT", "ROUND", "FLOOR", "CEIL", "ABS",
    "CURRENT_TIMESTAMP", "FROM_UNIXTIME", "UNIX_TIMESTAMP",
];

const POSTGRESQL_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "CONCAT", "LENGTH", "UPPER",
    "LOWER", "TRIM", "SUBSTRING", "REPLACE", "DATE", "NOW", "YEAR",
    "MONTH", "DAY", "COALESCE", "CAST", "NULLIF", "STRING_AGG",
    "ROUND", "FLOOR", "CEIL", "ABS", "TO_CHAR", "TO_DATE",
    "TO_TIMESTAMP", "EXTRACT", "ARRAY_AGG", "ROW_NUMBER", "RANK",
    "DENSE_RANK",
];

const SQLITE_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "LENGTH", "UPPER", "LOWER",
    "TRIM", "SUBSTR", "REPLACE", "DATE", "TIME", "DATETIME", "NOW",
    "IFNULL", "COALESCE", "CAST", "NULLIF", "GROUP_CONCAT", "ROUND",
    "ABS", "TYPEOF", "INSTR",
];

// ═══════════════════════════════════════════════════════════
// Preset structured data
// ═══════════════════════════════════════════════════════════

// ─── DataTypes ─────────────────────────────────────────────

const MYSQL_TYPES_INFO: &[DataType] = &[
    DataType { name: "INT",         category: DataTypeCategory::Numeric },
    DataType { name: "TINYINT",     category: DataTypeCategory::Numeric },
    DataType { name: "SMALLINT",    category: DataTypeCategory::Numeric },
    DataType { name: "MEDIUMINT",   category: DataTypeCategory::Numeric },
    DataType { name: "BIGINT",      category: DataTypeCategory::Numeric },
    DataType { name: "INTEGER",     category: DataTypeCategory::Numeric },
    DataType { name: "FLOAT",       category: DataTypeCategory::Numeric },
    DataType { name: "DOUBLE",      category: DataTypeCategory::Numeric },
    DataType { name: "DECIMAL",     category: DataTypeCategory::Numeric },
    DataType { name: "NUMERIC",     category: DataTypeCategory::Numeric },
    DataType { name: "BIT",         category: DataTypeCategory::Numeric },
    DataType { name: "CHAR",        category: DataTypeCategory::String },
    DataType { name: "VARCHAR",     category: DataTypeCategory::String },
    DataType { name: "TEXT",        category: DataTypeCategory::String },
    DataType { name: "TINYTEXT",    category: DataTypeCategory::String },
    DataType { name: "MEDIUMTEXT",  category: DataTypeCategory::String },
    DataType { name: "LONGTEXT",    category: DataTypeCategory::String },
    DataType { name: "BLOB",        category: DataTypeCategory::String },
    DataType { name: "TINYBLOB",    category: DataTypeCategory::String },
    DataType { name: "MEDIUMBLOB",  category: DataTypeCategory::String },
    DataType { name: "LONGBLOB",    category: DataTypeCategory::String },
    DataType { name: "BINARY",      category: DataTypeCategory::String },
    DataType { name: "VARBINARY",   category: DataTypeCategory::String },
    DataType { name: "ENUM",        category: DataTypeCategory::String },
    DataType { name: "SET",         category: DataTypeCategory::String },
    DataType { name: "DATE",        category: DataTypeCategory::DateTime },
    DataType { name: "DATETIME",    category: DataTypeCategory::DateTime },
    DataType { name: "TIMESTAMP",   category: DataTypeCategory::DateTime },
    DataType { name: "TIME",        category: DataTypeCategory::DateTime },
    DataType { name: "YEAR",        category: DataTypeCategory::DateTime },
    DataType { name: "BOOLEAN",     category: DataTypeCategory::Numeric },
    DataType { name: "BOOL",        category: DataTypeCategory::Numeric },
    DataType { name: "JSON",        category: DataTypeCategory::Json },
];

const POSTGRES_TYPES_INFO: &[DataType] = &[
    DataType { name: "SMALLINT",           category: DataTypeCategory::Numeric },
    DataType { name: "INTEGER",            category: DataTypeCategory::Numeric },
    DataType { name: "BIGINT",             category: DataTypeCategory::Numeric },
    DataType { name: "SERIAL",             category: DataTypeCategory::Numeric },
    DataType { name: "BIGSERIAL",          category: DataTypeCategory::Numeric },
    DataType { name: "SMALLSERIAL",        category: DataTypeCategory::Numeric },
    DataType { name: "REAL",               category: DataTypeCategory::Numeric },
    DataType { name: "DOUBLE PRECISION",   category: DataTypeCategory::Numeric },
    DataType { name: "NUMERIC",            category: DataTypeCategory::Numeric },
    DataType { name: "DECIMAL",            category: DataTypeCategory::Numeric },
    DataType { name: "MONEY",              category: DataTypeCategory::Numeric },
    DataType { name: "CHAR",               category: DataTypeCategory::String },
    DataType { name: "VARCHAR",            category: DataTypeCategory::String },
    DataType { name: "TEXT",               category: DataTypeCategory::String },
    DataType { name: "BYTEA",              category: DataTypeCategory::String },
    DataType { name: "DATE",               category: DataTypeCategory::DateTime },
    DataType { name: "TIME",               category: DataTypeCategory::DateTime },
    DataType { name: "TIMESTAMP",          category: DataTypeCategory::DateTime },
    DataType { name: "TIMESTAMPTZ",        category: DataTypeCategory::DateTime },
    DataType { name: "INTERVAL",           category: DataTypeCategory::DateTime },
    DataType { name: "BOOLEAN",            category: DataTypeCategory::Numeric },
    DataType { name: "JSON",               category: DataTypeCategory::Json },
    DataType { name: "JSONB",              category: DataTypeCategory::Json },
    DataType { name: "UUID",               category: DataTypeCategory::String },
    DataType { name: "INET",               category: DataTypeCategory::String },
    DataType { name: "CIDR",               category: DataTypeCategory::String },
    DataType { name: "MACADDR",            category: DataTypeCategory::String },
    DataType { name: "MACADDR8",           category: DataTypeCategory::String },
    DataType { name: "ARRAY",              category: DataTypeCategory::String },
    DataType { name: "POINT",              category: DataTypeCategory::Spatial },
    DataType { name: "LINE",               category: DataTypeCategory::Spatial },
    DataType { name: "LSEG",               category: DataTypeCategory::Spatial },
    DataType { name: "BOX",                category: DataTypeCategory::Spatial },
    DataType { name: "PATH",               category: DataTypeCategory::Spatial },
    DataType { name: "POLYGON",            category: DataTypeCategory::Spatial },
    DataType { name: "CIRCLE",             category: DataTypeCategory::Spatial },
    DataType { name: "TSVECTOR",           category: DataTypeCategory::String },
    DataType { name: "TSQUERY",            category: DataTypeCategory::String },
];

const SQLITE_TYPES_INFO: &[DataType] = &[
    DataType { name: "INTEGER",  category: DataTypeCategory::Numeric },
    DataType { name: "INT",      category: DataTypeCategory::Numeric },
    DataType { name: "REAL",     category: DataTypeCategory::Numeric },
    DataType { name: "TEXT",     category: DataTypeCategory::String },
    DataType { name: "BLOB",     category: DataTypeCategory::String },
    DataType { name: "NUMERIC",  category: DataTypeCategory::Numeric },
    DataType { name: "BOOLEAN",  category: DataTypeCategory::Numeric },
    DataType { name: "VARCHAR",  category: DataTypeCategory::String },
    DataType { name: "CHAR",     category: DataTypeCategory::String },
    DataType { name: "DATE",     category: DataTypeCategory::DateTime },
    DataType { name: "DATETIME", category: DataTypeCategory::DateTime },
    DataType { name: "TIMESTAMP", category: DataTypeCategory::DateTime },
    DataType { name: "FLOAT",    category: DataTypeCategory::Numeric },
    DataType { name: "DOUBLE",   category: DataTypeCategory::Numeric },
    DataType { name: "DECIMAL",  category: DataTypeCategory::Numeric },
];

// ─── Functions ─────────────────────────────────────────────

const MYSQL_FUNCTIONS_INFO: &[DbFunction] = &[
    DbFunction { name: "COUNT",            category: FunctionCategory::Aggregate },
    DbFunction { name: "SUM",              category: FunctionCategory::Aggregate },
    DbFunction { name: "AVG",              category: FunctionCategory::Aggregate },
    DbFunction { name: "MIN",              category: FunctionCategory::Aggregate },
    DbFunction { name: "MAX",              category: FunctionCategory::Aggregate },
    DbFunction { name: "GROUP_CONCAT",     category: FunctionCategory::Aggregate },
    DbFunction { name: "CONCAT",           category: FunctionCategory::String },
    DbFunction { name: "LENGTH",           category: FunctionCategory::String },
    DbFunction { name: "UPPER",            category: FunctionCategory::String },
    DbFunction { name: "LOWER",            category: FunctionCategory::String },
    DbFunction { name: "TRIM",             category: FunctionCategory::String },
    DbFunction { name: "SUBSTRING",        category: FunctionCategory::String },
    DbFunction { name: "REPLACE",          category: FunctionCategory::String },
    DbFunction { name: "LEFT",             category: FunctionCategory::String },
    DbFunction { name: "RIGHT",            category: FunctionCategory::String },
    DbFunction { name: "LOCATE",           category: FunctionCategory::String },
    DbFunction { name: "REVERSE",          category: FunctionCategory::String },
    DbFunction { name: "REPEAT",           category: FunctionCategory::String },
    DbFunction { name: "SPACE",            category: FunctionCategory::String },
    DbFunction { name: "FORMAT",           category: FunctionCategory::String },
    DbFunction { name: "ROUND",            category: FunctionCategory::Numeric },
    DbFunction { name: "FLOOR",            category: FunctionCategory::Numeric },
    DbFunction { name: "CEIL",             category: FunctionCategory::Numeric },
    DbFunction { name: "ABS",              category: FunctionCategory::Numeric },
    DbFunction { name: "POW",              category: FunctionCategory::Numeric },
    DbFunction { name: "SQRT",             category: FunctionCategory::Numeric },
    DbFunction { name: "RAND",             category: FunctionCategory::Numeric },
    DbFunction { name: "MOD",              category: FunctionCategory::Numeric },
    DbFunction { name: "SIGN",             category: FunctionCategory::Numeric },
    DbFunction { name: "TRUNCATE",         category: FunctionCategory::Numeric },
    DbFunction { name: "NOW",              category: FunctionCategory::DateTime },
    DbFunction { name: "CURDATE",          category: FunctionCategory::DateTime },
    DbFunction { name: "CURTIME",          category: FunctionCategory::DateTime },
    DbFunction { name: "DATE",             category: FunctionCategory::DateTime },
    DbFunction { name: "YEAR",             category: FunctionCategory::DateTime },
    DbFunction { name: "MONTH",            category: FunctionCategory::DateTime },
    DbFunction { name: "DAY",              category: FunctionCategory::DateTime },
    DbFunction { name: "HOUR",             category: FunctionCategory::DateTime },
    DbFunction { name: "MINUTE",           category: FunctionCategory::DateTime },
    DbFunction { name: "SECOND",           category: FunctionCategory::DateTime },
    DbFunction { name: "DATE_ADD",         category: FunctionCategory::DateTime },
    DbFunction { name: "DATE_SUB",         category: FunctionCategory::DateTime },
    DbFunction { name: "DATEDIFF",         category: FunctionCategory::DateTime },
    DbFunction { name: "DATE_FORMAT",      category: FunctionCategory::DateTime },
    DbFunction { name: "UNIX_TIMESTAMP",   category: FunctionCategory::DateTime },
    DbFunction { name: "FROM_UNIXTIME",    category: FunctionCategory::DateTime },
    DbFunction { name: "CURRENT_TIMESTAMP", category: FunctionCategory::DateTime },
    DbFunction { name: "IFNULL",           category: FunctionCategory::Conversion },
    DbFunction { name: "NULLIF",           category: FunctionCategory::Conversion },
    DbFunction { name: "COALESCE",         category: FunctionCategory::Conversion },
    DbFunction { name: "IF",               category: FunctionCategory::Conversion },
    DbFunction { name: "CAST",             category: FunctionCategory::Conversion },
    DbFunction { name: "CONVERT",          category: FunctionCategory::Conversion },
];

const POSTGRES_FUNCTIONS_INFO: &[DbFunction] = &[
    DbFunction { name: "COUNT",            category: FunctionCategory::Aggregate },
    DbFunction { name: "SUM",              category: FunctionCategory::Aggregate },
    DbFunction { name: "AVG",              category: FunctionCategory::Aggregate },
    DbFunction { name: "MIN",              category: FunctionCategory::Aggregate },
    DbFunction { name: "MAX",              category: FunctionCategory::Aggregate },
    DbFunction { name: "STRING_AGG",       category: FunctionCategory::Aggregate },
    DbFunction { name: "ARRAY_AGG",        category: FunctionCategory::Aggregate },
    DbFunction { name: "ROW_NUMBER",       category: FunctionCategory::Window },
    DbFunction { name: "RANK",             category: FunctionCategory::Window },
    DbFunction { name: "DENSE_RANK",       category: FunctionCategory::Window },
    DbFunction { name: "NTILE",            category: FunctionCategory::Window },
    DbFunction { name: "LAG",              category: FunctionCategory::Window },
    DbFunction { name: "LEAD",             category: FunctionCategory::Window },
    DbFunction { name: "FIRST_VALUE",      category: FunctionCategory::Window },
    DbFunction { name: "LAST_VALUE",       category: FunctionCategory::Window },
    DbFunction { name: "CONCAT",           category: FunctionCategory::String },
    DbFunction { name: "LENGTH",           category: FunctionCategory::String },
    DbFunction { name: "UPPER",            category: FunctionCategory::String },
    DbFunction { name: "LOWER",            category: FunctionCategory::String },
    DbFunction { name: "TRIM",             category: FunctionCategory::String },
    DbFunction { name: "SUBSTRING",        category: FunctionCategory::String },
    DbFunction { name: "REPLACE",          category: FunctionCategory::String },
    DbFunction { name: "POSITION",         category: FunctionCategory::String },
    DbFunction { name: "SPLIT_PART",       category: FunctionCategory::String },
    DbFunction { name: "LEFT",             category: FunctionCategory::String },
    DbFunction { name: "RIGHT",            category: FunctionCategory::String },
    DbFunction { name: "REVERSE",          category: FunctionCategory::String },
    DbFunction { name: "REGEXP_MATCHES",   category: FunctionCategory::String },
    DbFunction { name: "REGEXP_REPLACE",   category: FunctionCategory::String },
    DbFunction { name: "ROUND",            category: FunctionCategory::Numeric },
    DbFunction { name: "FLOOR",            category: FunctionCategory::Numeric },
    DbFunction { name: "CEIL",             category: FunctionCategory::Numeric },
    DbFunction { name: "ABS",              category: FunctionCategory::Numeric },
    DbFunction { name: "POW",              category: FunctionCategory::Numeric },
    DbFunction { name: "SQRT",             category: FunctionCategory::Numeric },
    DbFunction { name: "RANDOM",           category: FunctionCategory::Numeric },
    DbFunction { name: "MOD",              category: FunctionCategory::Numeric },
    DbFunction { name: "TRUNC",            category: FunctionCategory::Numeric },
    DbFunction { name: "SIGN",             category: FunctionCategory::Numeric },
    DbFunction { name: "NOW",              category: FunctionCategory::DateTime },
    DbFunction { name: "CURRENT_DATE",     category: FunctionCategory::DateTime },
    DbFunction { name: "CURRENT_TIME",     category: FunctionCategory::DateTime },
    DbFunction { name: "DATE",             category: FunctionCategory::DateTime },
    DbFunction { name: "YEAR",             category: FunctionCategory::DateTime },
    DbFunction { name: "MONTH",            category: FunctionCategory::DateTime },
    DbFunction { name: "DAY",              category: FunctionCategory::DateTime },
    DbFunction { name: "EXTRACT",          category: FunctionCategory::DateTime },
    DbFunction { name: "DATE_PART",        category: FunctionCategory::DateTime },
    DbFunction { name: "DATE_TRUNC",       category: FunctionCategory::DateTime },
    DbFunction { name: "TO_CHAR",          category: FunctionCategory::Conversion },
    DbFunction { name: "TO_DATE",          category: FunctionCategory::Conversion },
    DbFunction { name: "TO_TIMESTAMP",     category: FunctionCategory::Conversion },
    DbFunction { name: "COALESCE",         category: FunctionCategory::Conversion },
    DbFunction { name: "NULLIF",           category: FunctionCategory::Conversion },
    DbFunction { name: "CAST",             category: FunctionCategory::Conversion },
];

const SQLITE_FUNCTIONS_INFO: &[DbFunction] = &[
    DbFunction { name: "COUNT",        category: FunctionCategory::Aggregate },
    DbFunction { name: "SUM",          category: FunctionCategory::Aggregate },
    DbFunction { name: "AVG",          category: FunctionCategory::Aggregate },
    DbFunction { name: "MIN",          category: FunctionCategory::Aggregate },
    DbFunction { name: "MAX",          category: FunctionCategory::Aggregate },
    DbFunction { name: "GROUP_CONCAT", category: FunctionCategory::Aggregate },
    DbFunction { name: "LENGTH",       category: FunctionCategory::String },
    DbFunction { name: "UPPER",        category: FunctionCategory::String },
    DbFunction { name: "LOWER",        category: FunctionCategory::String },
    DbFunction { name: "TRIM",         category: FunctionCategory::String },
    DbFunction { name: "SUBSTR",       category: FunctionCategory::String },
    DbFunction { name: "REPLACE",      category: FunctionCategory::String },
    DbFunction { name: "INSTR",        category: FunctionCategory::String },
    DbFunction { name: "TYPEOF",       category: FunctionCategory::String },
    DbFunction { name: "ROUND",        category: FunctionCategory::Numeric },
    DbFunction { name: "ABS",          category: FunctionCategory::Numeric },
    DbFunction { name: "RANDOM",       category: FunctionCategory::Numeric },
    DbFunction { name: "DATE",         category: FunctionCategory::DateTime },
    DbFunction { name: "TIME",         category: FunctionCategory::DateTime },
    DbFunction { name: "DATETIME",     category: FunctionCategory::DateTime },
    DbFunction { name: "NOW",          category: FunctionCategory::DateTime },
    DbFunction { name: "STRFTIME",     category: FunctionCategory::DateTime },
    DbFunction { name: "JULIANDAY",    category: FunctionCategory::DateTime },
    DbFunction { name: "IFNULL",       category: FunctionCategory::Conversion },
    DbFunction { name: "NULLIF",       category: FunctionCategory::Conversion },
    DbFunction { name: "COALESCE",     category: FunctionCategory::Conversion },
    DbFunction { name: "CAST",         category: FunctionCategory::Conversion },
    DbFunction { name: "TOTAL",        category: FunctionCategory::Aggregate },
];

// ─── Engines ───────────────────────────────────────────────

const MYSQL_ENGINES: &[Engine] = &[
    Engine { name: "InnoDB",     desc: "Supports transactions, row-level locking, and foreign keys" },
    Engine { name: "MyISAM",     desc: "High-speed storage engine, no transaction support" },
    Engine { name: "MEMORY",     desc: "In-memory tables, ideal for temporary data" },
    Engine { name: "CSV",        desc: "Stores data in comma-separated values format" },
    Engine { name: "ARCHIVE",    desc: "Optimized for high-volume batch inserts" },
    Engine { name: "BLACKHOLE",  desc: "Accepts data but discards it, no storage" },
    Engine { name: "MERGE",      desc: "Collection of identical MyISAM tables" },
    Engine { name: "FEDERATED",  desc: "Accesses tables on remote MySQL servers" },
    Engine { name: "NDB",        desc: "Clustered database engine for high availability" },
];

// ─── Charsets & Collations ─────────────────────────────────

const MYSQL_CHARSETS: &[Charset] = &[
    Charset { name: "utf8mb4",  collations: &["utf8mb4_general_ci", "utf8mb4_unicode_ci", "utf8mb4_bin", "utf8mb4_0900_ai_ci"] },
    Charset { name: "utf8",     collations: &["utf8_general_ci", "utf8_unicode_ci", "utf8_bin"] },
    Charset { name: "utf8mb3",  collations: &["utf8mb3_general_ci", "utf8mb3_bin"] },
    Charset { name: "latin1",   collations: &["latin1_swedish_ci", "latin1_general_ci", "latin1_bin"] },
    Charset { name: "ascii",    collations: &["ascii_general_ci", "ascii_bin"] },
    Charset { name: "big5",     collations: &["big5_chinese_ci", "big5_bin"] },
    Charset { name: "gbk",      collations: &["gbk_chinese_ci", "gbk_bin"] },
    Charset { name: "gb2312",   collations: &["gb2312_chinese_ci", "gb2312_bin"] },
    Charset { name: "ujis",     collations: &["ujis_japanese_ci", "ujis_bin"] },
    Charset { name: "sjis",     collations: &["sjis_japanese_ci", "sjis_bin"] },
    Charset { name: "euckr",    collations: &["euckr_korean_ci", "euckr_bin"] },
    Charset { name: "utf16",    collations: &["utf16_general_ci", "utf16_bin"] },
    Charset { name: "utf32",    collations: &["utf32_general_ci", "utf32_bin"] },
    Charset { name: "latin2",   collations: &["latin2_general_ci", "latin2_bin"] },
    Charset { name: "latin5",   collations: &["latin5_turkish_ci", "latin5_bin"] },
    Charset { name: "latin7",   collations: &["latin7_general_ci", "latin7_bin"] },
    Charset { name: "cp1250",   collations: &["cp1250_general_ci", "cp1250_bin"] },
    Charset { name: "cp1251",   collations: &["cp1251_general_ci", "cp1251_bin"] },
    Charset { name: "cp1257",   collations: &["cp1257_general_ci", "cp1257_bin"] },
    Charset { name: "cp850",    collations: &["cp850_general_ci", "cp850_bin"] },
    Charset { name: "armscii8", collations: &["armscii8_general_ci", "armscii8_bin"] },
    Charset { name: "geostd8",  collations: &["geostd8_general_ci", "geostd8_bin"] },
    Charset { name: "greek",    collations: &["greek_general_ci", "greek_bin"] },
    Charset { name: "hebrew",   collations: &["hebrew_general_ci", "hebrew_bin"] },
    Charset { name: "hp8",      collations: &["hp8_english_ci", "hp8_bin"] },
    Charset { name: "keybcs2",  collations: &["keybcs2_general_ci", "keybcs2_bin"] },
    Charset { name: "koi8r",    collations: &["koi8r_general_ci", "koi8r_bin"] },
    Charset { name: "koi8u",    collations: &["koi8u_general_ci", "koi8u_bin"] },
    Charset { name: "macce",    collations: &["macce_general_ci", "macce_bin"] },
    Charset { name: "macroman", collations: &["macroman_general_ci", "macroman_bin"] },
    Charset { name: "swe7",     collations: &["swe7_swedish_ci", "swe7_bin"] },
    Charset { name: "tis620",   collations: &["tis620_thai_ci", "tis620_bin"] },
    Charset { name: "dec8",     collations: &["dec8_swedish_ci", "dec8_bin"] },
    Charset { name: "dos",      collations: &["dos_general_ci", "dos_bin"] },
    Charset { name: "binary",   collations: &["binary"] },
];

const POSTGRES_CHARSETS: &[Charset] = &[
    Charset { name: "UTF8",      collations: &["en_US.UTF-8", "zh_CN.UTF-8", "ja_JP.UTF-8"] },
    Charset { name: "LATIN1",    collations: &["en_US.LATIN1"] },
    Charset { name: "LATIN9",    collations: &["en_US.LATIN9"] },
    Charset { name: "SQL_ASCII", collations: &["C", "POSIX"] },
    Charset { name: "EUC_JP",    collations: &["ja_JP.EUC_JP"] },
    Charset { name: "EUC_KR",    collations: &["ko_KR.EUC_KR"] },
    Charset { name: "EUC_CN",    collations: &["zh_CN.EUC_CN"] },
    Charset { name: "BIG5",      collations: &["zh_TW.BIG5"] },
    Charset { name: "WIN1250",   collations: &["win1250"] },
    Charset { name: "WIN1251",   collations: &["win1251"] },
    Charset { name: "WIN1252",   collations: &["win1252"] },
    Charset { name: "WIN1256",   collations: &["win1256"] },
    Charset { name: "WIN874",    collations: &["win874"] },
];

// ─── Privileges ────────────────────────────────────────────

const MYSQL_PRIVILEGES: &[Privilege] = &[
    Privilege { name: "ALL PRIVILEGES",      scopes: &[PrivilegeScope::Global] },
    Privilege { name: "ALTER",               scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "ALTER ROUTINE",       scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "CREATE",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CREATE ROUTINE",      scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "CREATE TABLESPACE",   scopes: &[PrivilegeScope::Global] },
    Privilege { name: "CREATE TEMPORARY TABLES", scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "CREATE USER",         scopes: &[PrivilegeScope::Global] },
    Privilege { name: "CREATE VIEW",         scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "DELETE",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "DROP",                scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "DROP ROLE",           scopes: &[PrivilegeScope::Global] },
    Privilege { name: "EVENT",               scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "EXECUTE",             scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "FILE",                scopes: &[PrivilegeScope::Global] },
    Privilege { name: "GRANT OPTION",        scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Routine] },
    Privilege { name: "INDEX",               scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "INSERT",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "LOCK TABLES",         scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "PROCESS",             scopes: &[PrivilegeScope::Global] },
    Privilege { name: "REFERENCES",          scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "RELOAD",              scopes: &[PrivilegeScope::Global] },
    Privilege { name: "REPLICATION CLIENT",  scopes: &[PrivilegeScope::Replication] },
    Privilege { name: "REPLICATION SLAVE",   scopes: &[PrivilegeScope::Replication] },
    Privilege { name: "SELECT",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "SHOW DATABASES",      scopes: &[PrivilegeScope::Global] },
    Privilege { name: "SHOW VIEW",           scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "SHUTDOWN",            scopes: &[PrivilegeScope::Global] },
    Privilege { name: "SUPER",               scopes: &[PrivilegeScope::Global] },
    Privilege { name: "TRIGGER",             scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "UPDATE",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "USAGE",               scopes: &[PrivilegeScope::Global] },
];

const POSTGRES_PRIVILEGES: &[Privilege] = &[
    Privilege { name: "ALL",               scopes: &[PrivilegeScope::Global] },
    Privilege { name: "SELECT",            scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "INSERT",            scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "UPDATE",            scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "DELETE",            scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "TRUNCATE",          scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "REFERENCES",        scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "TRIGGER",           scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CREATE",            scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CONNECT",           scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "TEMPORARY",         scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "EXECUTE",           scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "USAGE",             scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "CREATE SCHEMA",     scopes: &[PrivilegeScope::Global, PrivilegeScope::Database] },
    Privilege { name: "CREATE TABLE",      scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CREATE VIEW",       scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CREATE SEQUENCE",   scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "CREATE FUNCTION",   scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "CREATE PROCEDURE",  scopes: &[PrivilegeScope::Global, PrivilegeScope::Routine] },
    Privilege { name: "DROP",              scopes: &[PrivilegeScope::Global, PrivilegeScope::Database, PrivilegeScope::Table] },
];

const SQLITE_PRIVILEGES: &[Privilege] = &[
    Privilege { name: "SELECT",     scopes: &[PrivilegeScope::Table, PrivilegeScope::Database] },
    Privilege { name: "INSERT",     scopes: &[PrivilegeScope::Table, PrivilegeScope::Database] },
    Privilege { name: "UPDATE",     scopes: &[PrivilegeScope::Table, PrivilegeScope::Column] },
    Privilege { name: "DELETE",     scopes: &[PrivilegeScope::Table, PrivilegeScope::Database] },
    Privilege { name: "CREATE",     scopes: &[PrivilegeScope::Database, PrivilegeScope::Table] },
    Privilege { name: "DROP",       scopes: &[PrivilegeScope::Database, PrivilegeScope::Table] },
];

// ─── SQL Modes ─────────────────────────────────────────────

const MYSQL_SQL_MODES: &[SqlMode] = &[
    SqlMode { name: "STRICT_TRANS_TABLES",     desc: "Strict mode for transactional tables" },
    SqlMode { name: "STRICT_ALL_TABLES",       desc: "Strict mode for all tables" },
    SqlMode { name: "ONLY_FULL_GROUP_BY",       desc: "Reject non-aggregated columns not in GROUP BY" },
    SqlMode { name: "NO_ZERO_DATE",            desc: "Reject zero dates" },
    SqlMode { name: "NO_ZERO_IN_DATE",         desc: "Reject dates with zero month/day" },
    SqlMode { name: "ERROR_FOR_DIVISION_BY_ZERO", desc: "Error on division by zero" },
    SqlMode { name: "NO_AUTO_CREATE_USER",     desc: "Prevent automatic user creation" },
    SqlMode { name: "NO_ENGINE_SUBSTITUTION",  desc: "Raise error on missing engine" },
    SqlMode { name: "PIPES_AS_CONCAT",         desc: "Treat || as string concatenation" },
    SqlMode { name: "ANSI_QUOTES",             desc: "Treat \" as identifier quote" },
    SqlMode { name: "IGNORE_SPACE",            desc: "Allow spaces between function name and ()" },
    SqlMode { name: "NO_AUTO_VALUE_ON_ZERO",   desc: "Insert 0 into AUTO_INCREMENT column" },
    SqlMode { name: "NO_BACKSLASH_ESCAPES",    desc: "Disable backslash escape" },
    SqlMode { name: "NO_DIR_IN_CREATE",        desc: "Ignore INDEX DIRECTORY in CREATE TABLE" },
    SqlMode { name: "NO_KEY_OPTIONS",          desc: "Ignore engine-specific index options" },
    SqlMode { name: "NO_TABLE_OPTIONS",        desc: "Ignore table options in SHOW CREATE TABLE" },
    SqlMode { name: "NO_UNSIGNED_SUBTRACTION", desc: "Treat subtraction as signed" },
    SqlMode { name: "PAD_CHAR_TO_FULL_LENGTH", desc: "Pad CHAR columns to full length" },
    SqlMode { name: "REAL_AS_FLOAT",           desc: "Treat REAL as FLOAT, not DOUBLE" },
    SqlMode { name: "TRADITIONAL",             desc: "Combination of strict modes" },
    SqlMode { name: "ANSI",                    desc: "Combination of ANSI-compatible modes" },
];

// ─── DbTypeInfo ────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct DbTypeInfo {
    pub enum_name: &'static str,
    pub label: &'static str,
    pub default_port: Option<u16>,
}

pub const DATABASE_TYPES: &[DbTypeInfo] = &[
    DbTypeInfo { enum_name: "MySql",      label: "MySQL",      default_port: Some(3306) },
    DbTypeInfo { enum_name: "Postgres",   label: "PostgreSQL", default_port: Some(5432) },
    DbTypeInfo { enum_name: "Sqlite",     label: "SQLite",     default_port: None },
];
