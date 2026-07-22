use crate::dialect::Dialect;

/// Builder for user introspection queries across supported dialects
pub struct UsersIntrospection;

impl UsersIntrospection {
    /// Generate a query to list all database users
    pub fn list_users(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => {
                "SELECT User, Host, account_locked, password_expired \
                 FROM mysql.user ORDER BY User"
                    .to_string()
            }
            "PostgreSQL" => {
                "SELECT rolname AS user, rolsuper AS is_superuser, \
                 rolcanlogin AS can_login, rolcreaterole AS can_create_role, \
                 rolcreatedb AS can_create_db \
                 FROM pg_roles WHERE rolcanlogin = true ORDER BY rolname"
                    .to_string()
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }

    /// Generate a query to retrieve detailed info for a specific user
    pub fn user_info(dialect: &dyn Dialect, username: &str, host: Option<&str>) -> String {
        match dialect.name() {
            "MySQL" => {
                let user = dialect.quote_str(username);
                let host = dialect.quote_str(host.unwrap_or("%"));
                format!("SHOW CREATE USER {user}@{host}")
            }
            "PostgreSQL" => {
                let user = username.replace('\'', "''");
                format!(
                    "SELECT rolname AS user, rolsuper AS is_superuser, \
                     rolcanlogin AS can_login, rolcreaterole AS can_create_role, \
                     rolcreatedb AS can_create_db, rolvaliduntil AS valid_until \
                     FROM pg_roles WHERE rolname = '{user}'"
                )
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }
}
