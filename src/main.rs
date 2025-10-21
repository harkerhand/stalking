mod agent;
mod config;
mod model;
mod monitor;
pub mod ssh;
#[cfg(test)]
mod tests;
mod ui;

use crate::config::Config;
use anyhow::Result;
use clap::Parser;
use monitor::Monitorable;
use std::path::PathBuf;
use tokio::select;

#[derive(clap::Parser)]
struct Cli {
    #[clap(
        short,
        long,
        default_value = "example_config.toml",
        help = "Path to configuration file"
    )]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load_config(&cli.config_path)?;
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let ui_handle = match config.global.display {
        ui::DisplayKind::Tui => ui::spawn_tui(rx, config.global.refresh),
        ui::DisplayKind::Plain => ui::spawn_plain(rx, config.global.refresh),
    };

    for server in config.servers {
        agent::spawn_agent(server, tx.clone());
    }

    select! {
        _ = tokio::signal::ctrl_c() => {
            println!("收到 Ctrl+C，正在退出...");
        }
        _ = ui_handle => {
            println!("UI 线程已退出");
        }
    }

    Ok(())
}
