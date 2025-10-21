use crate::model::MonitorEvent;
use tokio::sync::mpsc::Receiver;

pub fn spawn_tui(mut rx: Receiver<MonitorEvent>, interval: u64) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Placeholder for TUI implementation
        loop {
            while let Some(event) = rx.recv().await {
                // Here you would update the TUI with the new event
                // todo!("Implement TUI update logic");
                println!("TUI display event: {:?}", event);
            }
            // todo!("Implement TUI refresh logic");
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    })
}
