use crate::model::MonitorEvent;
use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, Duration};

pub fn spawn_plain(mut rx: Receiver<MonitorEvent>, interval: u64) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        async move {
            loop {
                while let Some(event) = rx.recv().await {
                    // Print the event in a plain format
                    // todo!("Enhance plain display formatting");
                    println!("Plain display event: {:?}", event);
                }
                // todo!("Implement plain display refresh logic");
                sleep(Duration::from_secs(interval)).await;
            }
        }
    )
}