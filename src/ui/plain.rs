use crate::model::{MonitorEvent, MonitorKind};
use crate::ui::{main_text, AppState};
use crossterm::event::{self, Event, KeyEvent};
use std::io::{stdout, Write};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{sleep, Duration};

pub fn spawn_plain(
    mut rx: Receiver<MonitorEvent>,
    interval_ms: u64,
    shutdown_tx: broadcast::Sender<()>,
    servers: Vec<String>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut shutdown_rx = shutdown_tx.subscribe();
        let state = Arc::new(RwLock::new(AppState::new(servers)));
        let kinds = [
            MonitorKind::Mem,
            MonitorKind::Cpu,
            MonitorKind::Disk,
            MonitorKind::Net,
        ];

        // 启用原始模式，便于捕获按键
        let _raw = crossterm::terminal::enable_raw_mode();

        loop {
            // 检查是否收到退出信号
            if let Ok(_) = shutdown_rx.try_recv() {
                crossterm::terminal::disable_raw_mode().ok();
                return;
            }

            // 收集所有新事件
            while let Ok(event) = rx.try_recv() {
                match event {
                    MonitorEvent::Sample { server, kind, payload, .. } => {
                        let server_states = &mut state.write().await.data;
                        server_states.entry(server).or_default().insert(kind, payload);
                    }
                    MonitorEvent::Error { server, kind, error, .. } => {
                        eprintln!("[{}][{:?}]: {}", server, kind, error);
                    }
                }
            }

            // 2. 处理键盘事件（非阻塞）
            while event::poll(Duration::from_millis(1)).unwrap_or(false) {
                if let Event::Key(KeyEvent { code, kind, .. }) = event::read().unwrap() {
                    if kind == event::KeyEventKind::Press {
                        let mut state = state.write().await;
                        if state.handle_key(code) {
                            let _ = shutdown_tx.send(());
                            crossterm::terminal::disable_raw_mode().ok();
                            return;
                        }
                    }
                }
            }

            // 3. 清屏
            print!("\x1B[2J\x1B[1;1H");
            stdout().flush().ok();

            // 4. 显示当前 server 和监控项
            let state = state.read().await;
            let text = main_text(&state, &kinds);
            println!("{}", text);
            println!("\n[N/L] NEXT/LAST SERVER  [1-4] MEM/CPU/DISK/NET  [q] QUIT");


            sleep(Duration::from_millis(interval_ms)).await;
        }
    })
}
