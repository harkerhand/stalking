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
    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
    let ui_handle = match config.global.display {
        ui::DisplayKind::Tui => ui::spawn_tui(rx, config.global.refresh),
        ui::DisplayKind::Plain => ui::spawn_plain(rx, config.global.refresh),
    };

    let mut agent_handles = Vec::new();
    for server in config.servers {
        let shutdown_rx = shutdown_tx.subscribe();
        let handle = agent::spawn_agent(server, tx.clone(), shutdown_rx);
        agent_handles.push(handle);
    }

    tokio::select! {
        res = ui_handle => {
            res?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("收到退出信号，正在关闭...");
            let _ = shutdown_tx.send(());
            for handle in agent_handles {
                let _ = handle.await;
            }
            println!("所有 agent 已退出，程序结束。");
        }
    }


    Ok(())
}
