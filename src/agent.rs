use crate::config::ServerConfig;
use crate::model::{MonitorEvent, MonitorPayload};
use crate::ssh::SSHClient;
use std::time::Duration;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};
use tokio::task::JoinHandle;

pub fn spawn_agent(
    server: ServerConfig,
    tx: Sender<MonitorEvent>,
    mut shutdown: Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    // 收到退出信号，优雅退出
                    break;
                }
                _ = async {
                    let client = SSHClient::connect_from_config(&server).expect("failed to connect to server");
                    for kind in &server.monitors {
                        let payload = MonitorPayload::from(kind);
                        match client.exec(payload) {
                            Ok(payload) => {
                                tx.send(MonitorEvent::Sample {
                                    server: server.name.clone(),
                                    kind: kind.clone(),
                                    payload,
                                    timestamp: chrono::Utc::now(),
                                }).await.expect("failed to send monitor event");
                            }
                            Err(e) => {
                                tx.send(MonitorEvent::Error {
                                    server: server.name.clone(),
                                    kind: Some(kind.clone()),
                                    error: e.to_string(),
                                    timestamp: chrono::Utc::now(),
                                }).await.expect("failed to send monitor error event");
                                continue;
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(server.interval)).await;
                } => {}
            }
        }
    })
}
