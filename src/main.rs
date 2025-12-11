use config::load_config;
use std::process;
mod config;
mod watcher;
use tokio::sync::mpsc;
mod scheduler;
use clap::{Parser, Subcommand};
use directories::UserDirs;
use log::{debug, error, info, LevelFilter};
use scheduler::TaskScheduler;
use simple_logger::SimpleLogger;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run(RunArgs),
    List(ListArgs),
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
struct ListArgs {
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

fn get_config_path() -> Result<PathBuf, String> {
    if let Some(user_dirs) = UserDirs::new() {
        let home_dir = user_dirs.home_dir();
        let config_path = home_dir
            .join(".config")
            .join("chronsync")
            .join("config.json");

        return Ok(config_path);
    }

    Err("Could not determine user home directory.".to_string())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let log_level = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    SimpleLogger::new()
        .with_level(log_level)
        .init()
        .expect("Failed to initialize logger");

    debug!("Parsed CLI: {:?}", cli);

    match cli.command {
        Commands::Run(args) => {
            handle_run_command(args).await;
        }
        Commands::List(args) => {
            debug!("Dispatching to handle_list_command");
            handle_list_command(args).await;
        }
    }
}

async fn handle_run_command(args: RunArgs) {
    debug!("Entered handle_run_command with args: {:?}", args);
    let config_path = match args.config_path {
        Some(p) => p,
        None => match get_config_path() {
            Ok(p) => p,
            Err(e) => {
                error!("Initialization Error: {}", e);
                process::exit(1);
            }
        },
    };

    debug!("Resolved config path: {}", config_path.display());

    if !config_path.exists() {
        error!("Initialization Error: Configuration file not found at path:");
        error!("-> Path: {}", config_path.display());
        process::exit(1);
    }

    let (tx_reload, mut rx_reload) = mpsc::channel::<()>(1);

    let mut scheduler = TaskScheduler::new();

    let watcher_path = config_path.clone();
    let tx_clone = tx_reload.clone();

    tokio::spawn(async move {
        if let Err(e) = watcher::start_watcher(&watcher_path, tx_clone).await {
            error!("Watcher failed: {:?}", e);
        }
    });

    info!("[Main] chronsync Daemon started.");

    match load_config(&config_path) {
        Ok(c) => {
            info!("[Main] Initial config loaded. {} tasks.", c.tasks.len());
            scheduler.reload_tasks(c);
        }
        Err(e) => {
            error!("[Main] Failed to load initial config. Existing: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            Some(_) = rx_reload.recv() => {
                info!("\n>>> CONFIG CHANGE DETECTED! RELOADING... <<<");

                match load_config(&config_path) {
                    Ok(new_config) => {
                        scheduler.reload_tasks(new_config);
                        info!("[Main] New configuration applied. Tasks reloaded.");
                    },
                    Err(e) => {
                        error!("[Main] Error reloading configuration (Configuration rejected): {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("\n[Main] Ctrl+C received. Shutting down gracefully...");
                scheduler.reload_tasks(config::Config { tasks: vec![] });
                break;
            }
        }
    }
}

async fn handle_list_command(args: ListArgs) {
    debug!("Entered handle_list_command with args: {:?}", args);
    let config_path = match args.config_path {
        Some(p) => p,
        None => match get_config_path() {
            Ok(p) => p,
            Err(e) => {
                error!("Error: Failed to determine configuration path.");
                error!("Reason: {}", e);
                process::exit(1);
            }
        },
    };

    debug!("Resolved config path: {}", config_path.display());

    if !config_path.exists() {
        error!("Error: Configuration file not found at path:");
        error!("-> Path: {}", config_path.display());
        process::exit(1);
    }

    match load_config(&config_path) {
        Ok(config) => {
            println!("Configuration loaded from: {}", config_path.display());
            println!(
                "\n--- chronsync Task List ({} Tasks) ---",
                config.tasks.len()
            );
            for task in config.tasks {
                println!("- [{}]: {}", task.name, task.cron_schedule.to_string());
                println!(
                    "  Command: {} {:?}",
                    task.command,
                    task.args.unwrap_or_default()
                );
                println!("-----------------------------");
            }
        }
        Err(e) => {
            error!("Error loading configuration: {}", e);
            error!("The configuration file contains invalid JSON or an invalid cron schedule.");
            process::exit(1);
        }
    }
}
