// use notify::EventKind;
use log::{error, info, warn};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
// use std::io::{self, Write};
use std::path::Path;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub async fn start_watcher(path: &Path, tx_reload: mpsc::Sender<()>) -> notify::Result<()> {
    let (tx_watcher, mut rx_watcher) = mpsc::channel(100);

    let handler = move |res: notify::Result<notify::Event>| match res {
        Ok(event) => {
            if event.kind.is_modify() {
                if let Err(e) = tx_watcher.try_send(()) {
                    error!(
                        "[Watcher] Failed to send event to internal channel: {:?}",
                        e
                    );
                }
            }
        }
        Err(e) => error!("[Watcher] Watch error: {:?}", e),
    };

    let mut watcher = RecommendedWatcher::new(
        handler,
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;

    info!("[Watcher] File watcher started on: {:?}", path);

    loop {
        if rx_watcher.recv().await.is_none() {
            break;
        }

        sleep(Duration::from_millis(1000)).await;

        while rx_watcher.try_recv().is_ok() {}

        info!("[Watcher] Debounced and sent reload signal.");

        match tx_reload.try_send(()) {
            Ok(_) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("[Watcher] Main loop is busy. Dropping reload signal.");
            }
            Err(e) => {
                error!("[Watcher] Failed to send reload signal: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
