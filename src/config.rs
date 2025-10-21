use crate::model::MonitorKind;
use crate::ui::DisplayKind;
use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// 全局配置根结构体，从 config.toml 反序列化。
#[derive(Debug, Deserialize)]
pub struct Config {
    pub global: GlobalConfig,
    pub servers: Vec<ServerConfig>,
}

impl Config {
    /// 从指定路径加载配置文件并反序列化为 Config 结构体。
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// 检查合法性
    pub fn validate(&self) -> Result<()> {
        self.global.validate()?;
        for server in &self.servers {
            server.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    /// UI 刷新间隔，单位毫秒
    #[serde(default = "default_refresh")]
    pub refresh: u64,
    /// 显示模式
    #[serde(default = "Default::default")]
    pub display: DisplayKind,
}
impl GlobalConfig {
    pub fn validate(&self) -> Result<()> {
        if self.refresh < 200 {
            anyhow::bail!("Global refresh interval must be greater than 200");
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub privkey_path: Option<PathBuf>,
    pub passphrase: Option<String>,
    pub monitors: Vec<MonitorKind>,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            anyhow::bail!("Server name cannot be empty");
        }
        if self.host.trim().is_empty() {
            anyhow::bail!("Server host cannot be empty");
        }
        if self.port == 0 {
            anyhow::bail!("Server port must be between 1 and 65535");
        }
        if self.user.trim().is_empty() {
            anyhow::bail!("Server user cannot be empty");
        }
        if self.monitors.is_empty() {
            anyhow::bail!(
                "At least one monitor must be specified for server {}",
                self.name
            );
        }
        Ok(())
    }
}

fn default_refresh() -> u64 {
    500
}
fn default_port() -> u16 {
    22
}
