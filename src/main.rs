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
        ui::DisplayKind::Tui => ui::spawn_tui(rx, config.global.refresh, shutdown_tx.clone(), config.servers.iter().map(
            |s| s.name.clone()).collect::<Vec<_>>()),
        ui::DisplayKind::Plain => ui::spawn_plain(rx, config.global.refresh),
    };

    let mut agent_handles = Vec::new();
    for server in config.servers {
        let shutdown_rx = shutdown_tx.subscribe();
        let handle = agent::spawn_agent(server, tx.clone(), shutdown_rx, config.global.refresh);
        agent_handles.push(handle);
    }

    tokio::select! {
        // TUI 主动退出（内部发出 shutdown 信号）
        res = ui_handle => {
            if let Err(e) = res {
                eprintln!("UI 任务出错: {e}");
            }
        }

        // Ctrl + C
        _ = tokio::signal::ctrl_c() => {
            println!("收到退出信号 (Ctrl+C)，正在关闭...");
            let _ = shutdown_tx.send(());
        }
    }

    // 6️⃣ 等待所有 agent 收尾
    for handle in agent_handles {
        let _ = handle.await;
    }

    println!("✅ 所有 agent 已退出，程序结束。");
    Ok(())
}
