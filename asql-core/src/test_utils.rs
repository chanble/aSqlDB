use sqlx::Connection;

/// Base MySQL URL without a database path.
/// Respects the `ASQL_TEST_MYSQL_URL` environment variable.
/// Default: `mysql://root:123456@127.0.0.1:3306`
pub fn mysql_base_url() -> String {
    std::env::var("ASQL_TEST_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://root:123456@127.0.0.1:3306".to_string())
}

pub struct MySqlDbGuard {
    db_name: String,
}

impl Drop for MySqlDbGuard {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let _ = tokio::spawn(async move {
            let base_url = mysql_base_url();
            if let Ok(mut conn) = sqlx::MySqlConnection::connect(&base_url).await {
                let sql = format!("DROP DATABASE IF EXISTS `{}`", db_name);
                sqlx::query(&sql).execute(&mut conn).await.ok();
            }
        });
    }
}

/// Creates the test database and returns a connection with a cleanup guard.
/// The database is automatically dropped when the guard is dropped.
pub async fn mysql_conn(db_name: &str) -> Option<(sqlx::MySqlConnection, MySqlDbGuard)> {
    let base_url = mysql_base_url();
    if let Ok(mut conn) = sqlx::MySqlConnection::connect(&base_url).await {
        let sql = format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name);
        sqlx::query(&sql).execute(&mut conn).await.ok();
        drop(conn);
    }
    let url = if base_url.ends_with('/') {
        format!("{}{}", base_url, db_name)
    } else {
        format!("{}/{}", base_url, db_name)
    };
    let conn = sqlx::MySqlConnection::connect(&url).await.ok()?;
    Some((conn, MySqlDbGuard { db_name: db_name.to_string() }))
}

/// Base PostgreSQL URL without a database path.
/// Respects the `ASQL_TEST_PG_URL` environment variable.
/// Default: `postgres://postgres:password@127.0.0.1:5432`
pub fn pg_base_url() -> String {
    std::env::var("ASQL_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@127.0.0.1:5432".to_string())
}

pub struct PgDbGuard {
    db_name: String,
}

impl Drop for PgDbGuard {
    fn drop(&mut self) {
        let db_name = self.db_name.clone();
        let _ = tokio::spawn(async move {
            let base_url = pg_base_url();
            if let Ok(mut conn) = sqlx::PgConnection::connect(&base_url).await {
                let sql = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
                sqlx::query(&sql).execute(&mut conn).await.ok();
            }
        });
    }
}

/// Creates the test database and returns a connection with a cleanup guard.
/// The database is automatically dropped when the guard is dropped.
pub async fn pg_conn(db_name: &str) -> Option<(sqlx::PgConnection, PgDbGuard)> {
    let base_url = pg_base_url();
    if let Ok(mut conn) = sqlx::PgConnection::connect(&base_url).await {
        let sql = format!("CREATE DATABASE IF NOT EXISTS \"{}\"", db_name);
        sqlx::query(&sql).execute(&mut conn).await.ok();
        drop(conn);
    }
    let url = if base_url.ends_with('/') {
        format!("{}{}", base_url, db_name)
    } else {
        format!("{}/{}", base_url, db_name)
    };
    let conn = sqlx::PgConnection::connect(&url).await.ok()?;
    Some((conn, PgDbGuard { db_name: db_name.to_string() }))
}
