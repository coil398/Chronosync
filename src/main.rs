use config::load_config;
use std::path::Path;
mod config;
mod watcher;
use tokio::sync::mpsc;
mod scheduler;
use scheduler::TaskScheduler;

#[tokio::main]
async fn main() {
    let config_path = Path::new("config.json");

    let (tx_reload, mut rx_reload) = mpsc::channel::<()>(1);

    let mut scheduler = TaskScheduler::new();

    let watcher_path = config_path.to_owned();
    let tx_clone = tx_reload.clone();

    tokio::spawn(async move {
        if let Err(e) = watcher::start_watcher(&watcher_path, tx_clone).await {
            eprintln!("Watcher failed: {:?}", e);
        }
    });

    println!("[Main] Chronosync Daemon started.");

    match load_config(config_path) {
        Ok(c) => {
            println!("[Main] Initial config loaded. {} tasks.", c.tasks.len());
            scheduler.reload_tasks(c);
        }
        Err(e) => {
            eprintln!("[Main] Failed to load initial config. Existing: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            Some(_) = rx_reload.recv() => {
                println!("\n>>> CONFIG CHANGE DETECTED! RELOADING... <<<");

                match load_config(config_path) {
                    Ok(new_config) => {
                        scheduler.reload_tasks(new_config);
                        println!("[Main] New configuration applied. Tasks reloaded.");
                    },
                    Err(e) => {
                        eprintln!("[Main] Error reloading configuration (Configuration rejected): {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n[Main] Ctrl+C received. Shutting down gracefully...");
                scheduler.reload_tasks(config::Config { tasks: vec![] });
                break;
            }
        }
    }
}
