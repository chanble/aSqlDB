use crate::dialect::Dialect;

// ── UserBuilder ─────────────────────────────────────────────────────────

enum UserMode {
    Create,
    Alter,
    Drop,
    Rename,
}

/// Builder for constructing user management statements (`CREATE USER`, `ALTER USER`, `DROP USER`, `RENAME USER`)
pub struct UserBuilder {
    mode: Option<UserMode>,
    username: Option<String>,
    host: Option<String>,
    identified_by: Option<String>,
    new_username: Option<String>,
    new_host: Option<String>,
}

impl UserBuilder {
    /// Creates a new empty `UserBuilder`
    pub fn new() -> Self {
        Self {
            mode: None,
            username: None,
            host: None,
            identified_by: None,
            new_username: None,
            new_host: None,
        }
    }

    /// Sets the builder to create a new user with the given name
    pub fn create_user(mut self, name: &str) -> Self {
        self.mode = Some(UserMode::Create);
        self.username = Some(name.to_string());
        self
    }

    /// Sets the builder to alter an existing user
    pub fn alter_user(mut self, name: &str) -> Self {
        self.mode = Some(UserMode::Alter);
        self.username = Some(name.to_string());
        self
    }

    /// Sets the builder to drop an existing user
    pub fn drop_user(mut self, name: &str) -> Self {
        self.mode = Some(UserMode::Drop);
        self.username = Some(name.to_string());
        self
    }

    /// Sets the builder to rename a user from the old name to a new name (MySQL: also supports changing host)
    pub fn rename_user(mut self, old: &str, new: &str) -> Self {
        self.mode = Some(UserMode::Rename);
        self.username = Some(old.to_string());
        self.new_username = Some(new.to_string());
        self
    }

    /// Sets the host part of the user (MySQL: defaults to `%`)
    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    /// Sets the password for the user (used with `IDENTIFIED BY`)
    pub fn identified_by(mut self, password: &str) -> Self {
        self.identified_by = Some(password.to_string());
        self
    }

    /// Sets the new host when renaming a user (MySQL only)
    pub fn new_host(mut self, host: &str) -> Self {
        self.new_host = Some(host.to_string());
        self
    }

    /// Builds the user management SQL statement for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => self.build_mysql(dialect),
            "PostgreSQL" => self.build_pg(dialect),
            _ => panic!("User management is not supported by {}", dialect.name()),
        }
    }

    fn build_mysql(&self, dialect: &dyn Dialect) -> String {
        let user = self.username.as_ref().expect("username is required");
        let host = dialect.quote_str(self.host.as_deref().unwrap_or("%"));
        let user_part = dialect.quote_str(user);
        let user_host = format!("{user_part}@{host}");

        match self.mode.as_ref().expect("operation mode is required") {
            UserMode::Create => {
                let mut sql = format!("CREATE USER {user_host}");
                if let Some(pw) = &self.identified_by {
                    sql.push_str(&format!(" IDENTIFIED BY {}", dialect.quote_str(pw)));
                }
                sql
            }
            UserMode::Alter => {
                let pw = self
                    .identified_by
                    .as_ref()
                    .expect("ALTER USER requires IDENTIFIED BY");
                format!(
                    "ALTER USER {user_host} IDENTIFIED BY {}",
                    dialect.quote_str(pw)
                )
            }
            UserMode::Drop => format!("DROP USER {user_host}"),
            UserMode::Rename => {
                let new_user = self.new_username.as_ref().expect("new username is required");
                let new_host = dialect.quote_str(self.new_host.as_deref().unwrap_or("%"));
                format!(
                    "RENAME USER {user_host} TO {new_user_part}@{new_host}",
                    new_user_part = dialect.quote_str(new_user)
                )
            }
        }
    }

    fn build_pg(&self, dialect: &dyn Dialect) -> String {
        let user = dialect.quote_ident(self.username.as_ref().expect("username is required"));

        match self.mode.as_ref().expect("operation mode is required") {
            UserMode::Create => {
                let mut sql = format!("CREATE USER {user}");
                if let Some(pw) = &self.identified_by {
                    sql.push_str(&format!(" WITH PASSWORD {}", dialect.quote_str(pw)));
                }
                sql
            }
            UserMode::Alter => {
                let pw = self
                    .identified_by
                    .as_ref()
                    .expect("ALTER USER requires PASSWORD");
                format!(
                    "ALTER USER {user} WITH PASSWORD {}",
                    dialect.quote_str(pw)
                )
            }
            UserMode::Drop => format!("DROP USER {user}"),
            UserMode::Rename => {
                let new_user = dialect.quote_ident(
                    self.new_username.as_ref().expect("new username is required"),
                );
                format!("ALTER USER {user} RENAME TO {new_user}")
            }
        }
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── GrantBuilder ────────────────────────────────────────────────────────

enum GrantMode {
    Grant,
    Revoke,
}

/// Predefined role profiles that configure a set of privileges and options
pub enum GrantRole {
    /// Full superuser access with all privileges and grant option on all objects
    SuperAdmin,
    /// Database administrator with all privileges and grant option
    DBA,
    /// Read-write access with SELECT, INSERT, UPDATE, DELETE, and EXECUTE
    ReadWrite,
    /// Read-only access with SELECT and SHOW VIEW
    ReadOnly,
    /// DDL access with CREATE, ALTER, DROP, and INDEX
    DDL,
}

/// Builder for constructing `GRANT` or `REVOKE` privilege statements
pub struct GrantBuilder {
    mode: Option<GrantMode>,
    privileges: Vec<String>,
    on: Option<String>,
    user: Option<String>,
    host: Option<String>,
    with_grant_option: bool,
}

impl GrantBuilder {
    /// Creates a new empty `GrantBuilder`
    pub fn new() -> Self {
        Self {
            mode: None,
            privileges: Vec::new(),
            on: None,
            user: None,
            host: None,
            with_grant_option: false,
        }
    }

    /// Sets the builder to grant the specified privileges
    pub fn grant(mut self, privileges: Vec<&str>) -> Self {
        self.mode = Some(GrantMode::Grant);
        self.privileges = privileges.into_iter().map(String::from).collect();
        self
    }

    /// Sets the builder to revoke the specified privileges
    pub fn revoke(mut self, privileges: Vec<&str>) -> Self {
        self.mode = Some(GrantMode::Revoke);
        self.privileges = privileges.into_iter().map(String::from).collect();
        self
    }

    /// Sets the target object or wildcard (e.g. `*.*` or `db.table`) on which privileges apply
    pub fn on(mut self, target: &str) -> Self {
        self.on = Some(target.to_string());
        self
    }

    /// Sets the user to grant privileges to
    pub fn to(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    /// Sets the user to revoke privileges from
    pub fn from(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    /// Sets the host part of the user (MySQL: defaults to `%`)
    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    /// Adds `WITH GRANT OPTION` to the statement
    pub fn with_grant_option(mut self) -> Self {
        self.with_grant_option = true;
        self
    }

    /// Configures the builder using a predefined role profile
    pub fn role(mut self, role: GrantRole) -> Self {
        self.mode = Some(GrantMode::Grant);
        match role {
            GrantRole::SuperAdmin => {
                self.privileges = vec!["ALL PRIVILEGES".to_string()];
                self.with_grant_option = true;
                if self.on.is_none() {
                    self.on = Some("*.*".to_string());
                }
            }
            GrantRole::DBA => {
                self.privileges = vec!["ALL PRIVILEGES".to_string()];
                self.with_grant_option = true;
            }
            GrantRole::ReadWrite => {
                self.privileges = vec![
                    "SELECT".to_string(),
                    "INSERT".to_string(),
                    "UPDATE".to_string(),
                    "DELETE".to_string(),
                    "EXECUTE".to_string(),
                ];
            }
            GrantRole::ReadOnly => {
                self.privileges =
                    vec!["SELECT".to_string(), "SHOW VIEW".to_string()];
            }
            GrantRole::DDL => {
                self.privileges = vec![
                    "CREATE".to_string(),
                    "ALTER".to_string(),
                    "DROP".to_string(),
                    "INDEX".to_string(),
                ];
            }
        }
        self
    }

    /// Builds the `GRANT` or `REVOKE` SQL statement for the configured dialect
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        match dialect.name() {
            "MySQL" => self.build_mysql(dialect),
            "PostgreSQL" => self.build_pg(dialect),
            _ => panic!("GRANT/REVOKE is not supported by {}", dialect.name()),
        }
    }

    fn fmt_on(&self, target: &str, dialect: &dyn Dialect) -> String {
        if target.contains('.') {
            target
                .split('.')
                .map(|part| {
                    if part == "*" {
                        part.to_string()
                    } else {
                        dialect.quote_ident(part)
                    }
                })
                .collect::<Vec<_>>()
                .join(".")
        } else if target == "*" {
            target.to_string()
        } else {
            dialect.quote_ident(target)
        }
    }

    fn build_mysql(&self, dialect: &dyn Dialect) -> String {
        let mode = self.mode.as_ref().expect("grant or revoke mode is required");
        let privs = self.privileges.join(", ");
        let target = self
            .on
            .as_ref()
            .map(|t| self.fmt_on(t, dialect))
            .expect("ON target is required");
        let user = self.user.as_ref().expect("user is required");
        let host = dialect.quote_str(self.host.as_deref().unwrap_or("%"));
        let user_host = format!("{}@{host}", dialect.quote_str(user));

        match mode {
            GrantMode::Grant => {
                let mut sql = format!("GRANT {privs} ON {target} TO {user_host}");
                if self.with_grant_option {
                    sql.push_str(" WITH GRANT OPTION");
                }
                sql
            }
            GrantMode::Revoke => {
                format!("REVOKE {privs} ON {target} FROM {user_host}")
            }
        }
    }

    fn build_pg(&self, dialect: &dyn Dialect) -> String {
        let mode = self.mode.as_ref().expect("grant or revoke mode is required");
        let privs = self.privileges.join(", ");
        let target = self
            .on
            .as_ref()
            .map(|t| self.fmt_on(t, dialect))
            .expect("ON target is required");
        let user = dialect.quote_ident(self.user.as_ref().expect("user is required"));

        match mode {
            GrantMode::Grant => {
                let mut sql = format!("GRANT {privs} ON {target} TO {user}");
                if self.with_grant_option {
                    sql.push_str(" WITH GRANT OPTION");
                }
                sql
            }
            GrantMode::Revoke => {
                format!("REVOKE {privs} ON {target} FROM {user}")
            }
        }
    }
}

impl Default for GrantBuilder {
    fn default() -> Self {
        Self::new()
    }
}
