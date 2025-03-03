use crate::environment::{ensure_npx_shim, get_uvx_path};
use crate::file_utils::{ensure_config_file, ensure_mcp_servers};
use dirs;
use lazy_static::lazy_static;
use log::{error, info};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

lazy_static! {
    static ref CONFIG_CACHE: Mutex<Option<Value>> = Mutex::new(None);
    static ref TEST_CONFIG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

// Function to set a test config path - only used in tests
pub fn set_test_config_path(path: Option<PathBuf>) {
    let mut test_path = TEST_CONFIG_PATH.lock().unwrap();
    *test_path = path;

    // Clear the cache when changing the config path
    let mut cache = CONFIG_CACHE.lock().unwrap();
    *cache = None;
}

// Function to get the config path - uses test path if set
fn get_config_path() -> Result<PathBuf, String> {
    // Check if we have a test config path set
    let test_path = TEST_CONFIG_PATH.lock().unwrap();
    if let Some(path) = test_path.clone() {
        return Ok(path);
    }

    // Otherwise use the default path
    let default_path = dirs::home_dir()
        .ok_or("Could not find home directory".to_string())?
        .join("Library/Application Support/Claude/claude_desktop_config.json");

    Ok(default_path)
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub mcp_key: String,
    pub command: String,
    pub args: Vec<String>,
}

pub fn get_app_configs() -> Vec<(String, AppConfig)> {
    let npx_shim = ensure_npx_shim().unwrap_or_else(|_| "npx".to_string());
    let uvx_path = get_uvx_path().unwrap_or_else(|_| "uvx".to_string());

    vec![
        (
            "Browser".to_string(),
            AppConfig {
                mcp_key: "puppeteer".to_string(),
                command: npx_shim.clone(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-puppeteer".to_string(),
                    "--debug".to_string(),
                ],
            },
        ),
        (
            "Time".to_string(),
            AppConfig {
                mcp_key: "time".to_string(),
                command: uvx_path.clone(),
                args: vec![
                    "--from".to_string(),
                    "git+https://github.com/modelcontextprotocol/servers.git#subdirectory=src/time"
                        .to_string(),
                    "mcp-server-time".to_string(),
                ],
            },
        ),
        (
            "Hacker News".to_string(),
            AppConfig {
                mcp_key: "hn".to_string(),
                command: uvx_path.clone(),
                args: vec![
                    "--from".to_string(),
                    "git+https://github.com/erithwik/mcp-hn".to_string(),
                    "mcp-hn".to_string(),
                ],
            },
        ),
        (
            "Gmail".to_string(),
            AppConfig {
                mcp_key: "gmail".to_string(),
                command: String::new(),
                args: vec![],
            },
        ),
        (
            "Google Calendar".to_string(),
            AppConfig {
                mcp_key: "calendar".to_string(),
                command: String::new(),
                args: vec![],
            },
        ),
        (
            "Google Drive".to_string(),
            AppConfig {
                mcp_key: "drive".to_string(),
                command: String::new(),
                args: vec![],
            },
        ),
    ]
}

pub fn get_config() -> Result<Value, String> {
    let mut cache = CONFIG_CACHE.lock().unwrap();
    if let Some(ref config) = *cache {
        return Ok(config.clone());
    }

    let config_path = get_config_path()?;

    if !config_path.exists() {
        ensure_config_file(&config_path)?;
    }

    let config_str = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let mut config_json: Value = serde_json::from_str(&config_str)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    ensure_mcp_servers(&mut config_json)?;

    *cache = Some(config_json.clone());
    Ok(config_json)
}

pub fn save_config(config: &Value) -> Result<(), String> {
    let config_path = get_config_path()?;

    let updated_config = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, updated_config)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    // Update cache
    let mut cache = CONFIG_CACHE.lock().unwrap();
    *cache = Some(config.clone());

    Ok(())
}

#[tauri::command]
pub fn preload_dependencies() -> Result<(), String> {
    std::thread::spawn(|| {
        let _ = Command::new("npm")
            .args(["cache", "add", "@modelcontextprotocol/server-puppeteer"])
            .output();

        let _ = Command::new("npm")
            .args(["cache", "add", "mcp-server-time"])
            .output();
    });
    Ok(())
}

#[tauri::command]
pub async fn install(app_name: String) -> Result<(), String> {
    info!("Installing app: {}", app_name);

    let configs = get_app_configs();
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == &app_name) {
        let mut config_json = get_config()?;
        let mcp_key = config.mcp_key.clone();
        let command = config.command.clone();
        let args = config.args.clone();

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            mcp_servers.insert(
                mcp_key.clone(),
                json!({
                    "command": command,
                    "args": args.clone(),
                }),
            );

            save_config(&config_json)?;

            std::thread::spawn(move || {
                if command.contains("npx") && args.len() > 1 {
                    let package = &args[1];
                    let _ = Command::new("npm").args(["cache", "add", package]).output();
                }
            });

            Ok(())
        } else {
            Err("Failed to find mcpServers in config".to_string())
        }
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn uninstall(app_name: String) -> Result<(), String> {
    info!("Uninstalling app: {}", app_name);

    if let Some((_, config)) = get_app_configs().iter().find(|(name, _)| name == &app_name) {
        let mut config_json = get_config()?;

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            if mcp_servers.remove(&config.mcp_key).is_some() {
                save_config(&config_json)?;
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Err("Failed to find mcpServers in config".to_string())
        }
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn is_installed(app_name: &str) -> Result<bool, String> {
    if let Some((_, config)) = get_app_configs().iter().find(|(name, _)| name == app_name) {
        let config_json = get_config()?;

        if let Some(mcp_servers) = config_json.get("mcpServers") {
            if let Some(servers) = mcp_servers.as_object() {
                return Ok(servers.contains_key(&config.mcp_key));
            }
        }

        Ok(false)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn get_app_statuses() -> Result<Value, String> {
    let config_json = get_config()?;

    let mut installed_apps = json!({});
    let mut configured_apps = json!({});

    let app_configs = get_app_configs();

    if let Some(mcp_servers) = config_json.get("mcpServers").and_then(|v| v.as_object()) {
        for (app_name, config) in app_configs {
            installed_apps[&app_name] = json!(mcp_servers.contains_key(&config.mcp_key));
            configured_apps[&app_name] = json!(!config.command.is_empty());
        }
    }

    Ok(json!({
        "installed": installed_apps,
        "configured": configured_apps
    }))
}
