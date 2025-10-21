use crate::Monitorable;
use crate::config::ServerConfig;
use anyhow::Result;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;

pub struct SSHClient {
    sess: Session,
}

impl SSHClient {
    pub fn with_pswd(host: &str, user: &str, pswd: &str) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{host}:22"))?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(user, pswd)?;
        Ok(Self { sess })
    }

    pub fn with_pubkey(
        host: &str,
        user: &str,
        pubkey_path: &str,
        privkey_path: &str,
        passphrase: Option<&str>,
    ) -> Result<Self> {
        let tcp = TcpStream::connect(format!("{host}:22"))?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_pubkey_file(
            user,
            Some(std::path::Path::new(pubkey_path)),
            std::path::Path::new(privkey_path),
            passphrase,
        )?;
        Ok(Self { sess })
    }

    pub fn connect_from_config(config: &ServerConfig) -> Result<Self> {
        if let (Some(pubkey_path), Some(privkey_path)) = (&config.key_path, &config.key_path) {
            Self::with_pubkey(&config.host, &config.user, pubkey_path, privkey_path, None)
        } else if let Some(pswd) = &config.password {
            Self::with_pswd(&config.host, &config.user, pswd)
        } else {
            Err(anyhow::anyhow!(
                "no authentication method provided for server {}",
                config.name
            ))
        }
    }

    pub fn exec<T: Monitorable>(&self, mut monitor: T) -> Result<T> {
        let mut channel = self.sess.channel_session()?;
        channel.exec(monitor.exec_cmd())?;
        let mut out = String::new();
        channel.read_to_string(&mut out)?;
        monitor.parse_from_str(&out)?;
        Ok(monitor)
    }
}
