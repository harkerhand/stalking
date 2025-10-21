use crate::Monitorable;
use crate::config::ServerConfig;
use anyhow::Result;
use async_ssh2_tokio::{ServerCheckMethod, ToSocketAddrsWithHostname};
use std::path::Path;

pub struct SSHClient {
    pub(crate) client: async_ssh2_tokio::Client,
}

impl SSHClient {
    pub async fn with_pswd(
        pswd: &str,
        user: impl AsRef<str>,
        addrs: impl ToSocketAddrsWithHostname,
    ) -> Result<Self> {
        let auth_method = async_ssh2_tokio::AuthMethod::with_password(pswd);
        let client = async_ssh2_tokio::Client::connect(
            addrs,
            user.as_ref(),
            auth_method,
            ServerCheckMethod::NoCheck,
        )
        .await?;
        Ok(Self { client })
    }

    pub async fn with_key(
        key_path: impl AsRef<Path>,
        user: impl AsRef<str>,
        passphrase: Option<&str>,
        addrs: impl ToSocketAddrsWithHostname,
    ) -> Result<Self> {
        let auth_method = async_ssh2_tokio::AuthMethod::with_key_file(key_path, passphrase);
        let client = async_ssh2_tokio::Client::connect(
            addrs,
            user.as_ref(),
            auth_method,
            ServerCheckMethod::NoCheck,
        )
        .await?;
        Ok(Self { client })
    }

    pub async fn connect_from_config(config: &ServerConfig) -> Result<Self> {
        if let Some(privkey_path) = &config.privkey_path {
            Self::with_key(
                privkey_path,
                &config.user,
                config.passphrase.as_deref(),
                (config.host.as_str(), config.port),
            )
            .await
        } else if let Some(pswd) = &config.password {
            Self::with_pswd(pswd, &config.user, (config.host.as_str(), config.port)).await
        } else {
            Err(anyhow::anyhow!(
                "no authentication method provided for server {}",
                config.name
            ))
        }
    }

    pub async fn exec<T: Monitorable>(&self, mut monitor: T) -> Result<T> {
        let result = self.client.execute(monitor.exec_cmd()).await?;
        match result.exit_status {
            0 => {
                monitor.parse_from_str(&result.stdout)?;
                Ok(monitor)
            }
            code => Err(anyhow::anyhow!(
                "command exited with non-zero status: {}",
                code
            )),
        }
    }
}
