pub mod app;
pub mod environment;
pub mod file_utils;

use core::panic::PanicInfo;
use log::{error, info};
use simplelog::{Config, ConfigBuilder, LevelFilter, WriteLogger};
use std::fs;
use std::path::PathBuf;
use tauri::Manager as _;
use tauri_plugin_updater::{Builder as UpdaterBuilder, UpdaterExt};
use time::macros::format_description;

fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let log_dir = home.join("Library/Logs/Fleur");
    fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join("fleur.log");

    let config = ConfigBuilder::new()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]Z"
        ))
        .build();

    WriteLogger::init(LevelFilter::Info, config, fs::File::create(log_file)?)?;
    Ok(())
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        info!("Update available: {}", update.version);
        let mut downloaded = 0;
        match update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    info!("Downloaded {downloaded} from {content_length:?}");
                },
                || {
                    info!("Download finished, preparing to install...");
                },
            )
            .await
        {
            Ok(_) => {
                info!("Update installed successfully, restarting...");
                app.restart();
            }
            Err(e) => {
                error!("Failed to install update: {}", e);
                if e.to_string().contains("InvalidSignature") {
                    error!("Update signature verification failed. This could mean the update package has been tampered with or the public key doesn't match.");
                }
            }
        }
    } else {
        info!("No update available");
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    std::thread::spawn(|| {
        let _ = app::preload_dependencies();
    });

    tauri::Builder::default()
        .plugin(UpdaterBuilder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app::install,
            app::uninstall,
            app::is_installed,
            app::get_app_statuses,
            app::preload_dependencies,
            app::save_app_env,
            app::get_app_env,
            app::get_app_registry,
            environment::ensure_environment,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            info!("Checking for updates...");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update(handle).await {
                    error!("Error checking for updates: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
