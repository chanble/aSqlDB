use crate::dialect::Dialect;

/// Operation to perform on a database
pub enum DatabaseOp {
    /// Create a new database
    Create,
    /// Alter an existing database
    Alter,
}

/// Builder for constructing `CREATE DATABASE` or `ALTER DATABASE` statements
pub struct DatabaseBuilder {
    op: DatabaseOp,
    name: Option<String>,
    if_not_exists: bool,
    character_set: Option<String>,
    collation: Option<String>,
    default_encryption: Option<bool>,
}

impl DatabaseBuilder {
    /// Creates a new builder for a `CREATE DATABASE` statement
    pub fn create(name: &str) -> Self {
        Self {
            op: DatabaseOp::Create,
            name: Some(name.to_string()),
            if_not_exists: false,
            character_set: None,
            collation: None,
            default_encryption: None,
        }
    }

    /// Creates a new builder for an `ALTER DATABASE` statement
    pub fn alter(name: &str) -> Self {
        Self {
            op: DatabaseOp::Alter,
            name: Some(name.to_string()),
            if_not_exists: false,
            character_set: None,
            collation: None,
            default_encryption: None,
        }
    }

    /// Adds `IF NOT EXISTS` to the statement (only applies to `CREATE`)
    pub fn if_not_exists(mut self) -> Self {
        self.if_not_exists = true;
        self
    }

    /// Sets the default character set for the database
    pub fn character_set(mut self, cs: &str) -> Self {
        self.character_set = Some(cs.to_string());
        self
    }

    /// Sets the default collation for the database
    pub fn collation(mut self, collation: &str) -> Self {
        self.collation = Some(collation.to_string());
        self
    }

    /// Sets the default encryption setting for the database (MySQL only)
    pub fn default_encryption(mut self, yes: bool) -> Self {
        self.default_encryption = Some(yes);
        self
    }

    /// Builds the `CREATE DATABASE` or `ALTER DATABASE` SQL statement
    pub fn build(&self, dialect: &dyn Dialect) -> String {
        let name = self
            .name
            .as_deref()
            .expect("DATABASE requires a name");
        let quoted_name = dialect.quote_ident(name);

        let cmd = match self.op {
            DatabaseOp::Create => Some("CREATE"),
            DatabaseOp::Alter => Some("ALTER"),
        };

        let mut sql = format!("{} DATABASE", cmd.unwrap());

        if self.if_not_exists {
            sql.push_str(" IF NOT EXISTS");
        }

        sql.push(' ');
        sql.push_str(&quoted_name);

        if dialect.name() == "MySQL" {
            if let Some(cs) = &self.character_set {
                sql.push_str(&format!(" DEFAULT CHARACTER SET {cs}"));
            }
            if let Some(ref collation) = self.collation {
                sql.push_str(&format!(" DEFAULT COLLATE {collation}"));
            }
            if let Some(enc) = self.default_encryption {
                let val = if enc { "'Y'" } else { "'N'" };
                sql.push_str(&format!(" DEFAULT ENCRYPTION {val}"));
            }
        } else if dialect.name() == "PostgreSQL" {
            if let Some(cs) = &self.character_set {
                sql.push_str(&format!(" ENCODING '{cs}'"));
            }
            if let Some(ref collation) = self.collation {
                sql.push_str(&format!(" LC_COLLATE '{collation}'"));
            }
        }

        sql
    }
}
