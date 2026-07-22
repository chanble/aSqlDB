use crate::db_manager::DatabaseType;
use crate::result::DbError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ConnectionParams {
    MySql {
        host: String,
        port: u16,
        user: String,
        password: Option<String>,
        database: Option<String>,
    },
    Postgres {
        host: String,
        port: u16,
        user: String,
        password: Option<String>,
        database: Option<String>,
    },
    Sqlite {
        path: String,
    },
}

impl ConnectionParams {
    pub fn to_url(&self) -> String {
        match self {
            ConnectionParams::MySql { host, port, user, password, database } => {
                let pass = password.as_deref().unwrap_or("");
                let base = if pass.is_empty() {
                    format!("mysql://{}@{}:{}", user, host, port)
                } else {
                    format!("mysql://{}:{}@{}:{}", user, pass, host, port)
                };
                match database {
                    Some(db) if !db.is_empty() => format!("{}/{}", base, db),
                    _ => base,
                }
            }
            ConnectionParams::Postgres { host, port, user, password, database } => {
                let pass = password.as_deref().unwrap_or("");
                let base = if pass.is_empty() {
                    format!("postgres://{}@{}:{}", user, host, port)
                } else {
                    format!("postgres://{}:{}@{}:{}", user, pass, host, port)
                };
                match database {
                    Some(db) if !db.is_empty() => format!("{}/{}", base, db),
                    _ => base,
                }
            }
            ConnectionParams::Sqlite { path } => {
                if path == ":memory:" {
                    "sqlite::memory:".to_string()
                } else {
                    format!("sqlite://{}", path)
                }
            }
        }
    }

    pub fn db_type(&self) -> DatabaseType {
        match self {
            ConnectionParams::MySql { .. } => DatabaseType::MySql,
            ConnectionParams::Postgres { .. } => DatabaseType::Postgres,
            ConnectionParams::Sqlite { .. } => DatabaseType::Sqlite,
        }
    }

    /// Parse a URL string into structured params (for backward compatibility).
    pub fn from_url(url: &str) -> Self {
        if let Some(rest) = url.strip_prefix("mysql://") {
            let (host, port, user, password, database) = parse_mysql_pg(rest);
            Self::MySql { host, port, user, password, database }
        } else if let Some(rest) = url.strip_prefix("postgres://").or_else(|| url.strip_prefix("postgresql://")) {
            let (host, port, user, password, database) = parse_mysql_pg(rest);
            Self::Postgres { host, port, user, password, database }
        } else {
            let path = if url == "sqlite::memory:" {
                ":memory:".to_string()
            } else if let Some(p) = url.strip_prefix("sqlite://") {
                p.to_string()
            } else {
                url.to_string()
            };
            Self::Sqlite { path }
        }
    }
}

fn parse_mysql_pg(rest: &str) -> (String, u16, String, Option<String>, Option<String>) {
    let (userinfo, hostpart) = rest.split_once('@').unwrap_or(("", rest));
    let (user, password) = userinfo.split_once(':')
        .map(|(u, p)| (u.to_string(), Some(p.to_string())))
        .unwrap_or_else(|| (userinfo.to_string(), None));
    let (hostport, database) = hostpart.split_once('/')
        .map(|(hp, db)| (hp, Some(db.to_string())))
        .unwrap_or((hostpart, None));
    let (host, port) = hostport.split_once(':')
        .map(|(h, p)| (h.to_string(), p.parse::<u16>().unwrap_or(3306)))
        .unwrap_or_else(|| (hostport.to_string(), 3306));
    (host, port, user, password, database)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectionConfig {
    pub name: String,
    pub params: ConnectionParams,
}

impl ConnectionConfig {
    pub fn from_url(name: String, url: &str) -> Self {
        Self {
            name,
            params: ConnectionParams::from_url(url),
        }
    }
}

pub(crate) async fn save(
    configs: &[ConnectionConfig],
    path: &Path,
) -> Result<(), DbError> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf());

    if let Some(dir) = &dir {
        fs::create_dir_all(dir).await.map_err(|e| {
            DbError::ConnectionError(format!("Failed to create directory: {}", e))
        })?;
    }

    let json = serde_json::to_string_pretty(configs).map_err(|e| {
        DbError::ConnectionError(format!("Serialization failed: {}", e))
    })?;

    fs::write(path, json).await.map_err(|e| {
        DbError::ConnectionError(format!("Failed to write file: {}", e))
    })?;

    Ok(())
}

pub(crate) async fn load(path: &Path) -> Result<Vec<ConnectionConfig>, DbError> {
    let content = fs::read_to_string(path).await.map_err(|e| {
        DbError::ConnectionError(format!("Failed to read file: {}", e))
    })?;

    // Try new format first
    if let Ok(configs) = serde_json::from_str::<Vec<ConnectionConfig>>(&content) {
        return Ok(configs);
    }

    // Fall back to old format with plain url field
    #[derive(Deserialize)]
    struct OldConfig {
        name: String,
        url: String,
    }

    let old: Vec<OldConfig> = serde_json::from_str(&content).map_err(|e| {
        DbError::ConnectionError(format!("Failed to parse config: {}", e))
    })?;

    Ok(old.into_iter().map(|c| ConnectionConfig::from_url(c.name, &c.url)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::fs;

    #[serial]
    #[tokio::test]
    async fn test_save_and_load() {
        let dir = std::env::temp_dir().join("asql_persistence_test");
        let path = dir.join("connections.json");

        let configs = vec![
            ConnectionConfig {
                name: "my_mysql".to_string(),
                params: ConnectionParams::MySql {
                    host: "127.0.0.1".to_string(),
                    port: 3306,
                    user: "root".to_string(),
                    password: Some("123456".to_string()),
                    database: None,
                },
            },
            ConnectionConfig {
                name: "my_pg".to_string(),
                params: ConnectionParams::Postgres {
                    host: "localhost".to_string(),
                    port: 5432,
                    user: "user".to_string(),
                    password: Some("pass".to_string()),
                    database: Some("mydb".to_string()),
                },
            },
            ConnectionConfig {
                name: "my_sqlite".to_string(),
                params: ConnectionParams::Sqlite {
                    path: ":memory:".to_string(),
                },
            },
        ];

        save(&configs, &path).await.unwrap();

        let loaded = load(&path).await.unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].name, "my_mysql");
        assert_eq!(loaded[1].name, "my_pg");
        assert_eq!(loaded[2].name, "my_sqlite");

        let json_str = fs::read_to_string(&path).await.unwrap();
        assert!(json_str.contains("\"name\": \"my_mysql\""));

        fs::remove_dir_all(&dir).await.ok();
    }

    #[serial]
    #[tokio::test]
    async fn test_load_old_format() {
        let dir = std::env::temp_dir().join("asql_persistence_old_test");
        let path = dir.join("connections.json");
        let old_json = serde_json::json!([
            { "name": "old_mysql", "url": "mysql://root:pass@localhost:3306/test" },
            { "name": "old_pg", "url": "postgres://u:p@pg.local:5432/db" },
            { "name": "old_sqlite", "url": "sqlite::memory:" },
        ]);
        fs::create_dir_all(&dir).await.unwrap();
        fs::write(&path, old_json.to_string()).await.unwrap();

        let loaded = load(&path).await.unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].name, "old_mysql");
        assert_eq!(loaded[1].name, "old_pg");
        assert_eq!(loaded[2].name, "old_sqlite");

        fs::remove_dir_all(&dir).await.ok();
    }

    #[serial]
    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let result = load(Path::new("/nonexistent/path/connections.json")).await;
        assert!(result.is_err());
    }

    #[serial]
    #[tokio::test]
    async fn test_load_invalid_json() {
        let dir = std::env::temp_dir().join("asql_persistence_invalid_test");
        let path = dir.join("bad.json");
        fs::create_dir_all(&dir).await.unwrap();
        fs::write(&path, "not valid json").await.unwrap();

        let result = load(&path).await;
        assert!(result.is_err());

        fs::remove_dir_all(&dir).await.ok();
    }

    #[serial]
    #[tokio::test]
    async fn test_save_empty_configs() {
        let dir = std::env::temp_dir().join("asql_persistence_empty_test");
        let path = dir.join("connections.json");

        save(&[], &path).await.unwrap();
        let loaded = load(&path).await.unwrap();
        assert!(loaded.is_empty());

        fs::remove_dir_all(&dir).await.ok();
    }

    #[serial]
    #[tokio::test]
    async fn test_from_url() {
        let c = ConnectionConfig::from_url("test".into(), "mysql://root:pass@127.0.0.1:3307/mydb");
        assert_eq!(c.name, "test");
        match &c.params {
            ConnectionParams::MySql { host, port, user, password, database } => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(*port, 3307);
                assert_eq!(user, "root");
                assert_eq!(password.as_deref(), Some("pass"));
                assert_eq!(database.as_deref(), Some("mydb"));
            }
            _ => panic!("expected MySql"),
        }
    }

    #[serial]
    #[tokio::test]
    async fn test_to_url() {
        let params = ConnectionParams::MySql {
            host: "localhost".to_string(),
            port: 3306,
            user: "u".to_string(),
            password: Some("p".to_string()),
            database: Some("db".to_string()),
        };
        assert_eq!(params.to_url(), "mysql://u:p@localhost:3306/db");

        let params = ConnectionParams::Sqlite {
            path: ":memory:".to_string(),
        };
        assert_eq!(params.to_url(), "sqlite::memory:");
    }
}
