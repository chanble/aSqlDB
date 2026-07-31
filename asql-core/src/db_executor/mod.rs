use std::time::Instant;

use crate::result::{DbError, DbSuccessResult, ExecutionResult};
use sqlx::Row;

#[allow(unused_imports)]
use sqlx::types::chrono;

mod mysql;
mod pg;
mod sqlite;

pub(crate) use mysql::*;
pub(crate) use pg::*;
pub(crate) use sqlite::*;

#[macro_export]
macro_rules! make_error {
    ($sql:expr, $e:expr) => {
        DbError::SqlExecutionError {
            sql: $sql.to_string(),
            source: $e,
        }
    };
}

pub(crate) enum SqlKind {
    Query,
    Modify,
    Schema,
}


#[derive(Clone, Copy, PartialEq)]
enum SplitState {
    Normal,
    SingleQuote,
    DoubleQuote,
    Backtick,
    LineComment,
    BlockComment,
}

pub fn split_sql_statements(sql: &str) -> Vec<&str> {
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut state = SplitState::Normal;
    let mut statements = Vec::new();
    let mut stmt_start = 0usize;
    let mut byte_pos = 0usize;

    let mut i = 0;
    while i < len {
        let c = chars[i];
        match state {
            SplitState::Normal => match c {
                '\'' => state = SplitState::SingleQuote,
                '"' => state = SplitState::DoubleQuote,
                '`' => state = SplitState::Backtick,
                '-' if i + 1 < len && chars[i + 1] == '-' => {
                    state = SplitState::LineComment;
                    advance2(&chars, &mut i, &mut byte_pos);
                    continue;
                }
                '/' if i + 1 < len && chars[i + 1] == '*' => {
                    state = SplitState::BlockComment;
                    advance2(&chars, &mut i, &mut byte_pos);
                    continue;
                }
                '#' => state = SplitState::LineComment,
                ';' => {
                    let stmt = sql[stmt_start..byte_pos].trim();
                    if !stmt.is_empty() {
                        statements.push(stmt);
                    }
                    stmt_start = byte_pos + c.len_utf8();
                }
                _ => {}
            },
            SplitState::SingleQuote => {
                if c == '\'' {
                    if i + 1 < len && chars[i + 1] == '\'' {
                        advance2(&chars, &mut i, &mut byte_pos);
                        continue;
                    }
                    state = SplitState::Normal;
                }
            }
            SplitState::DoubleQuote => {
                if c == '"' {
                    state = SplitState::Normal;
                }
            }
            SplitState::Backtick => {
                if c == '`' {
                    state = SplitState::Normal;
                }
            }
            SplitState::LineComment => {
                if c == '\n' {
                    state = SplitState::Normal;
                }
            }
            SplitState::BlockComment => {
                if c == '*' && i + 1 < len && chars[i + 1] == '/' {
                    state = SplitState::Normal;
                    advance2(&chars, &mut i, &mut byte_pos);
                    continue;
                }
            }
        }
        byte_pos += c.len_utf8();
        i += 1;
    }

    if stmt_start < sql.len() {
        let remaining = sql[stmt_start..].trim();
        if !remaining.is_empty() {
            statements.push(remaining);
        }
    }

    statements
}

fn advance2(chars: &[char], i: &mut usize, byte_pos: &mut usize) {
    *byte_pos += chars[*i].len_utf8();
    *i += 1;
    *byte_pos += chars[*i].len_utf8();
    *i += 1;
}

fn strip_leading_comments(sql: &str) -> &str {
    let mut s = sql.trim_start();
    loop {
        if s.starts_with("--") {
            if let Some(pos) = s.find('\n') {
                s = s[pos + 1..].trim_start();
            } else {
                return "";
            }
        } else if s.starts_with("/*") {
            if let Some(pos) = s.find("*/") {
                s = s[pos + 2..].trim_start();
            } else {
                return "";
            }
        } else {
            return s;
        }
    }
}

pub(crate) fn classify_sql(sql: &str) -> SqlKind {
    let stripped = strip_leading_comments(sql);
    let upper = stripped.to_uppercase();

    if upper.starts_with("SELECT")
        || upper.starts_with("SHOW")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("DESCRIBE")
        || upper.starts_with("WITH")
    {
        SqlKind::Query
    } else if upper.starts_with("INSERT")
        || upper.starts_with("UPDATE")
        || upper.starts_with("DELETE")
    {
        SqlKind::Modify
    } else {
        SqlKind::Schema
    }
}

pub(crate) async fn execute_sql_on_mysql_conn(
    sql: &str,
    conn: &mut sqlx::MySqlConnection,
) -> Result<DbSuccessResult, DbError> {
    let trimmed_sql = sql.trim();
    let kind = classify_sql(trimmed_sql);
    let start = Instant::now();
    let duration_ms = || start.elapsed().as_millis() as u64;
    let sql = sql.to_string();
    match kind {
        SqlKind::Query => execute_select_mysql(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Select(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Modify => execute_modify_mysql(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Modify(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Schema => execute_generic_mysql(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Schema(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
    }
}

pub(crate) async fn execute_sql_on_pg_conn(
    sql: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<DbSuccessResult, DbError> {
    let trimmed_sql = sql.trim();
    let kind = classify_sql(trimmed_sql);
    let start = Instant::now();
    let duration_ms = || start.elapsed().as_millis() as u64;
    let sql = sql.to_string();
    match kind {
        SqlKind::Query => execute_select_pg(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Select(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Modify => execute_modify_pg(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Modify(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Schema => execute_generic_pg(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Schema(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
    }
}

pub(crate) async fn execute_sql_on_sqlite_conn(
    sql: &str,
    conn: &mut sqlx::SqliteConnection,
) -> Result<DbSuccessResult, DbError> {
    let trimmed_sql = sql.trim();
    let kind = classify_sql(trimmed_sql);
    let start = Instant::now();
    let duration_ms = || start.elapsed().as_millis() as u64;
    let sql = sql.to_string();
    match kind {
        SqlKind::Query => execute_select_sqlite(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Select(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Modify => execute_modify_sqlite(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Modify(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
        SqlKind::Schema => execute_generic_sqlite(trimmed_sql, conn).await.map(|data| {
            DbSuccessResult::Schema(ExecutionResult { sql, duration_ms: duration_ms(), data })
        }),
    }
}


fn decode_col<'r, T, R>(row: &'r R, idx: usize) -> Option<serde_json::Value>
where
    R: Row,
    T: serde::Serialize + sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database> + 'static,
    usize: sqlx::ColumnIndex<R>,
{
    match row.try_get::<Option<T>, _>(idx) {
        Ok(Some(v)) => serde_json::to_value(v).ok(),
        Ok(None) => Some(serde_json::Value::Null),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use sqlx::Connection;
    use sqlx::AssertSqlSafe;

    #[serial]
    #[test]
    fn test_classify_sql() {
        assert!(matches!(classify_sql("SELECT 1"), SqlKind::Query));
        assert!(matches!(classify_sql("  SELECT 1"), SqlKind::Query));
        assert!(matches!(
            classify_sql("INSERT INTO t VALUES (1)"),
            SqlKind::Modify
        ));
        assert!(matches!(classify_sql("UPDATE t SET x=1"), SqlKind::Modify));
        assert!(matches!(classify_sql("DELETE FROM t"), SqlKind::Modify));
        assert!(matches!(
            classify_sql("CREATE TABLE t (id INT)"),
            SqlKind::Schema
        ));
        assert!(matches!(classify_sql("DROP TABLE t"), SqlKind::Schema));
        assert!(matches!(classify_sql("SHOW TABLES"), SqlKind::Query));
        assert!(matches!(classify_sql("EXPLAIN SELECT 1"), SqlKind::Query));
        assert!(matches!(classify_sql("DESCRIBE t"), SqlKind::Query));
        assert!(matches!(
            classify_sql("WITH cte AS (SELECT 1) SELECT * FROM cte"),
            SqlKind::Query
        ));
        assert!(matches!(
            classify_sql("-- comment\nSELECT 1"),
            SqlKind::Query
        ));
        assert!(matches!(
            classify_sql("/* block */ SELECT 1"),
            SqlKind::Query
        ));
        assert!(matches!(
            classify_sql("/* a */ -- b\nWITH cte AS (SELECT 1) SELECT * FROM cte"),
            SqlKind::Query
        ));
    }

    // ─── split_sql_statements ───────────────────────────────────────────

    #[test]
    fn test_split_sql_statements_basic() {
        let stmts = split_sql_statements("SELECT 1; SELECT 2;");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "SELECT 1");
        assert_eq!(stmts[1], "SELECT 2");
    }

    #[test]
    fn test_split_sql_statements_no_trailing_semicolon() {
        let stmts = split_sql_statements("INSERT INTO t VALUES (1); UPDATE t SET x = 2");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[1], "UPDATE t SET x = 2");
    }

    #[test]
    fn test_split_sql_statements_semicolon_in_quotes() {
        let stmts = split_sql_statements("INSERT INTO t VALUES ('a;b'); SELECT 1");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "INSERT INTO t VALUES ('a;b')");
    }

    #[test]
    fn test_split_sql_statements_escaped_quote() {
        let stmts = split_sql_statements("INSERT INTO t VALUES ('it''s; ok'); SELECT 1");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "INSERT INTO t VALUES ('it''s; ok')");
    }

    #[test]
    fn test_split_sql_statements_backtick_and_double_quote() {
        let stmts = split_sql_statements("SELECT `a;b` FROM t; SELECT \"x;y\"");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "SELECT `a;b` FROM t");
        assert_eq!(stmts[1], "SELECT \"x;y\"");
    }

    #[test]
    fn test_split_sql_statements_comments() {
        let sql = "-- line; comment\nSELECT 1; /* block; comment */ SELECT 2; # hash; comment\nSELECT 3";
        let stmts = split_sql_statements(sql);
        assert_eq!(stmts.len(), 3);
        assert_eq!(stmts[0], "-- line; comment\nSELECT 1");
        assert_eq!(stmts[1], "/* block; comment */ SELECT 2");
        assert_eq!(stmts[2], "# hash; comment\nSELECT 3");
    }

    #[test]
    fn test_split_sql_statements_empty_and_whitespace() {
        let stmts = split_sql_statements("  ;  \n; ;  ");
        assert_eq!(stmts.len(), 0);
    }

    #[test]
    fn test_split_sql_statements_unicode() {
        let stmts = split_sql_statements("SELECT '中文;测试'; SELECT '😀;emoji'");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "SELECT '中文;测试'");
        assert_eq!(stmts[1], "SELECT '😀;emoji'");
    }

    #[test]
    fn test_split_sql_statements_large_linear() {
        let mut sql = String::new();
        for _ in 0..100_000 {
            sql.push_str("INSERT INTO t VALUES (1);");
        }
        let stmts = split_sql_statements(&sql);
        assert_eq!(stmts.len(), 100_000);
    }

    // ─── SQLite in-memory helpers ────────────────────────────────────────

    async fn setup_sqlite_table(create: &str, insert: &str) -> sqlx::SqliteConnection {
        let mut conn = sqlx::sqlite::SqliteConnection::connect(":memory:")
            .await
            .expect("cannot create in-memory SQLite database");
        sqlx::query(AssertSqlSafe(create)).execute(&mut conn).await.unwrap();
        sqlx::query(AssertSqlSafe(insert)).execute(&mut conn).await.unwrap();
        conn
    }

    // ─── get_column_value_sqlite ─────────────────────────────────────────

    #[serial]
    #[tokio::test]
    async fn test_sqlite_integer() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v INTEGER)", "INSERT INTO t VALUES (42)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(42i64)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_integer_negative() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v INTEGER)", "INSERT INTO t VALUES (-7)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(-7i64)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_integer_zero() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v INTEGER)", "INSERT INTO t VALUES (0)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(0i64)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_real() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v REAL)", "INSERT INTO t VALUES (3.14)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let got = get_column_value_sqlite(&rows[0], 0);
        let expected = serde_json::json!(3.14f64);
        assert!(
            (got.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-10,
            "expected {:?}, got {:?}",
            expected,
            got
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_real_zero() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v REAL)", "INSERT INTO t VALUES (0.0)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(0.0f64)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_text() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v TEXT)", "INSERT INTO t VALUES ('hello')").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!("hello")
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_text_empty() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v TEXT)", "INSERT INTO t VALUES ('')").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(get_column_value_sqlite(&rows[0], 0), serde_json::json!(""));
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_boolean_true() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v BOOLEAN)", "INSERT INTO t VALUES (1)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(true)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_boolean_false() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v BOOLEAN)", "INSERT INTO t VALUES (0)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(false)
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_null() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v INTEGER)", "INSERT INTO t VALUES (NULL)").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::Value::Null
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_blob() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (v BLOB)",
            "INSERT INTO t VALUES (x'48656c6c6f')",
        )
        .await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!("<BLOB>")
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_blob_non_utf8() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v BLOB)", "INSERT INTO t VALUES (x'ff')").await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let val = get_column_value_sqlite(&rows[0], 0);
        assert_eq!(val, serde_json::json!("<BLOB>"));
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_date() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (v DATE)",
            "INSERT INTO t VALUES ('2024-01-15')",
        )
        .await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let val = get_column_value_sqlite(&rows[0], 0);
        assert_eq!(val, serde_json::json!("2024-01-15"));
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_time() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (v TIME)",
            "INSERT INTO t VALUES ('12:30:00')",
        )
        .await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let val = get_column_value_sqlite(&rows[0], 0);
        assert_eq!(val, serde_json::json!("12:30:00"));
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_datetime() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (v DATETIME)",
            "INSERT INTO t VALUES ('2024-01-15 12:30:00')",
        )
        .await;
        let rows = sqlx::query("SELECT v FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let val = get_column_value_sqlite(&rows[0], 0);
        assert_eq!(val, serde_json::json!("2024-01-15T12:30:00"));
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_extract_rows_multiple_rows() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (a INTEGER, b TEXT)",
            "INSERT INTO t VALUES (1, 'one')",
        )
        .await;
        sqlx::query("INSERT INTO t VALUES (2, 'two')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT a, b FROM t ORDER BY a")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(1i64)
        );
        assert_eq!(
            get_column_value_sqlite(&rows[0], 1),
            serde_json::json!("one")
        );
        assert_eq!(
            get_column_value_sqlite(&rows[1], 0),
            serde_json::json!(2i64)
        );
        assert_eq!(
            get_column_value_sqlite(&rows[1], 1),
            serde_json::json!("two")
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_multiple_columns() {
        let mut conn = setup_sqlite_table(
            "CREATE TABLE t (a INTEGER, b TEXT, c REAL)",
            "INSERT INTO t VALUES (10, 'abc', 2.5)",
        )
        .await;
        let rows = sqlx::query("SELECT a, b, c FROM t")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_sqlite(&rows[0], 0),
            serde_json::json!(10i64)
        );
        assert_eq!(
            get_column_value_sqlite(&rows[0], 1),
            serde_json::json!("abc")
        );
        let got = get_column_value_sqlite(&rows[0], 2);
        let expected = serde_json::json!(2.5f64);
        assert!(
            (got.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-10,
            "expected {:?}, got {:?}",
            expected,
            got
        );
    }

    #[serial]
    #[tokio::test]
    async fn test_sqlite_empty_table() {
        let mut conn =
            setup_sqlite_table("CREATE TABLE t (v INTEGER)", "INSERT INTO t VALUES (1)").await;
        let rows = sqlx::query("SELECT v FROM t WHERE 1=0")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    // ─── get_column_value_mysql (requires ASQL_TEST_MYSQL_URL) ────────────

    #[serial]
    #[tokio::test]
    async fn test_mysql_integer() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_int (v INT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_int")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_int VALUES (42)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_int")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_mysql(&rows[0], 0),
            serde_json::json!(42i32)
        );
        sqlx::query("DROP TABLE IF EXISTS _test_int")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_varchar() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_vc (v VARCHAR(64))")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_vc")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_vc VALUES ('hello')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_vc")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_mysql(&rows[0], 0),
            serde_json::json!("hello")
        );
        sqlx::query("DROP TABLE IF EXISTS _test_vc")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_null() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_null (v INT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_null")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_null VALUES (NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_null")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(get_column_value_mysql(&rows[0], 0), serde_json::Value::Null);
        sqlx::query("DROP TABLE IF EXISTS _test_null")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_bigint() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_bi (v BIGINT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_bi")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_bi VALUES (9223372036854775807)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_bi")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_mysql(&rows[0], 0),
            serde_json::json!(9223372036854775807i64)
        );
        sqlx::query("DROP TABLE IF EXISTS _test_bi")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_double() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_db (v DOUBLE)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_db")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_db VALUES (2.71828)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_db")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let got = get_column_value_mysql(&rows[0], 0);
        let expected = serde_json::json!(2.71828f64);
        assert!(
            (got.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-10,
            "expected {:?}, got {:?}",
            expected,
            got
        );
        sqlx::query("DROP TABLE IF EXISTS _test_db")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_decimal_integer_value() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_dec2 (v DECIMAL(10,0))")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_dec2")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_dec2 VALUES (42)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_dec2")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let got = get_column_value_mysql(&rows[0], 0);
        assert!(got.is_string(), "expected a number, got {:?}", got);
        assert_eq!(got, "42");
        sqlx::query("DROP TABLE IF EXISTS _test_dec2")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_decimal() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_dec (v DECIMAL(10,2))")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_dec")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_dec VALUES (123.45)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_dec")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let got: serde_json::Value = get_column_value_mysql(&rows[0], 0);
        assert!(got.is_string(), "expected a string, got {:?}", got);
        assert_eq!(got, "123.45");
        sqlx::query("DROP TABLE IF EXISTS _test_dec")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_datetime() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_dt (v DATETIME(0))")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_dt")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_dt VALUES ('2024-01-15 12:30:00')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_dt")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_mysql(&rows[0], 0),
            serde_json::json!("2024-01-15T12:30:00")
        );
        sqlx::query("DROP TABLE IF EXISTS _test_dt")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_mysql_timestamp() {
        let (mut conn, _guard) = match crate::test_utils::mysql_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_ts (v TIMESTAMP(0))")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_ts")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_ts VALUES ('2024-01-15 12:30:00')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_ts")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_mysql(&rows[0], 0),
            serde_json::json!("2024-01-15T12:30:00Z")
        );
        sqlx::query("DROP TABLE IF EXISTS _test_ts")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    // ─── get_column_value_pg (requires ASQL_TEST_PG_URL) ──────────────────

    #[serial]
    #[tokio::test]
    async fn test_pg_integer() {
        let (mut conn, _guard) = match crate::test_utils::pg_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_int (v INT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_int")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_int VALUES (42)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_int")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(get_column_value_pg(&rows[0], 0), serde_json::json!(42i32));
        sqlx::query("DROP TABLE IF EXISTS _test_int")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_pg_text() {
        let (mut conn, _guard) = match crate::test_utils::pg_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_txt (v TEXT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_txt")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_txt VALUES ('world')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_txt")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(get_column_value_pg(&rows[0], 0), serde_json::json!("world"));
        sqlx::query("DROP TABLE IF EXISTS _test_txt")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_pg_null() {
        let (mut conn, _guard) = match crate::test_utils::pg_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_null (v INT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_null")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_null VALUES (NULL)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_null")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(get_column_value_pg(&rows[0], 0), serde_json::Value::Null);
        sqlx::query("DROP TABLE IF EXISTS _test_null")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_pg_bigint() {
        let (mut conn, _guard) = match crate::test_utils::pg_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_bi (v BIGINT)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_bi")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_bi VALUES (9223372036854775807)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_bi")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            get_column_value_pg(&rows[0], 0),
            serde_json::json!(9223372036854775807i64)
        );
        sqlx::query("DROP TABLE IF EXISTS _test_bi")
            .execute(&mut conn)
            .await
            .unwrap();
    }

    #[serial]
    #[tokio::test]
    async fn test_pg_double() {
        let (mut conn, _guard) = match crate::test_utils::pg_conn("asql_test_db").await {
            Some(v) => v,
            None => return,
        };
        sqlx::query("CREATE TABLE IF NOT EXISTS _test_db (v DOUBLE PRECISION)")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _test_db")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO _test_db VALUES (1.618)")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT v FROM _test_db")
            .fetch_all(&mut conn)
            .await
            .unwrap();
        let got = get_column_value_pg(&rows[0], 0);
        let expected = serde_json::json!(1.618f64);
        assert!(
            (got.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-10,
            "expected {:?}, got {:?}",
            expected,
            got
        );
        sqlx::query("DROP TABLE IF EXISTS _test_db")
            .execute(&mut conn)
            .await
            .unwrap();
    }
}
