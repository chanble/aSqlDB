use crate::dialect::Dialect;

/// Builder for server-level introspection queries (processes, variables, status)
pub struct ServerIntrospection;

impl ServerIntrospection {
    /// Generate a query to list active server processes or connections
    pub fn process_list(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => "SHOW FULL PROCESSLIST".to_string(),
            "PostgreSQL" => {
                "SELECT pid, usename AS user, application_name, \
                 client_addr AS host, state, query, query_start, wait_event \
                 FROM pg_stat_activity ORDER BY pid"
                    .to_string()
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }

    /// Generate a query to show server configuration variables
    pub fn variables(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => "SHOW VARIABLES".to_string(),
            "PostgreSQL" => {
                "SELECT name, setting, unit, category, short_desc \
                 FROM pg_settings ORDER BY name"
                    .to_string()
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }

    /// Generate a query to retrieve server status metrics
    pub fn status(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => "SHOW GLOBAL STATUS".to_string(),
            "PostgreSQL" => {
                "SELECT datname, numbackends, xact_commit, xact_rollback, \
                 blks_read, blks_hit, tup_returned, tup_fetched, \
                 tup_inserted, tup_updated, tup_deleted, conflicts, deadlocks \
                 FROM pg_stat_database ORDER BY datname"
                    .to_string()
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }

    /// Generate a query to get the database server version.
    ///
    /// The result column is always named `version` so callers can read
    /// results with a fixed key regardless of the underlying engine.
    ///
    /// - MySQL:    `SELECT VERSION() as version`
    /// - PostgreSQL: `SELECT version() as version`
    /// - SQLite:  `SELECT sqlite_version() as version`
    pub fn version(dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => "SELECT VERSION() as version".to_string(),
            "PostgreSQL" => "SELECT version() as version".to_string(),
            "SQLite" => "SELECT sqlite_version() as version".to_string(),
            _ => unreachable!(),
        }
    }

    /// Generate a query to kill one or more server processes by PID
    pub fn kill_process(dialect: &dyn Dialect, pids: &[&str]) -> String {
        if pids.is_empty() {
            return String::new();
        }
        match dialect.name() {
            "MySQL" => {
                let stmts: Vec<String> = pids
                    .iter()
                    .map(|pid| format!("KILL CONNECTION {pid}"))
                    .collect();
                stmts.join(";\n")
            }
            "PostgreSQL" => {
                let pids = pids.join(", ");
                format!("SELECT count(pg_terminate_backend(pids.pid)) \
                         FROM unnest(ARRAY[{pids}]) AS pids(pid)")
            }
            "SQLite" => String::new(),
            _ => unreachable!(),
        }
    }
}
