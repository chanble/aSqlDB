use std::path::PathBuf;

/// 统一配置管理：加载 TUI/CLI 共享的配置项
pub struct AppConfig {
    pub config_dir: PathBuf,
}

impl AppConfig {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    /// 默认配置目录 ~/.aSqlDB
    pub fn default_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".aSqlDB")
    }

    /// connections.json 路径
    pub fn connections_path(&self) -> PathBuf {
        self.config_dir.join("connections.json")
    }

    /// config.toml 路径
    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// LanceDB 存储路径
    pub fn lancedb_path(&self) -> PathBuf {
        self.config_dir.join("lancedb")
    }
}
