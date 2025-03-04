use crate::environment::{
    ensure_node_environment, ensure_npx_shim, ensure_uv_environment, get_uvx_path,
};
use crate::file_utils::{ensure_config_file, ensure_mcp_servers};
use dirs;
use lazy_static::lazy_static;
use log::{error, info};
use reqwest::blocking::get;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

lazy_static! {
    static ref CONFIG_CACHE: Mutex<Option<Value>> = Mutex::new(None);
    static ref TEST_CONFIG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub static ref APP_REGISTRY_CACHE: Mutex<Option<Value>> = Mutex::new(None);
}

pub fn set_test_config_path(path: Option<PathBuf>) {
    let mut test_path = TEST_CONFIG_PATH.lock().unwrap();
    *test_path = path;

    let mut cache = CONFIG_CACHE.lock().unwrap();
    *cache = None;
}

fn get_config_path() -> Result<PathBuf, String> {
    let test_path = TEST_CONFIG_PATH.lock().unwrap();
    if let Some(path) = test_path.clone() {
        return Ok(path);
    }

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

fn ensure_runtime_paths() -> Result<(String, String), String> {
    ensure_uv_environment().map_err(|e| format!("Failed to set up UV environment: {}", e))?;

    ensure_node_environment().map_err(|e| format!("Failed to set up Node environment: {}", e))?;

    let npx_shim = ensure_npx_shim().map_err(|e| format!("Failed to ensure NPX shim: {}", e))?;

    let uvx_path = get_uvx_path().map_err(|e| format!("Failed to get UVX path: {}", e))?;

    Ok((npx_shim, uvx_path))
}

fn fetch_app_registry() -> Result<Value, String> {
    let mut cache = APP_REGISTRY_CACHE.lock().unwrap();
    if let Some(ref registry) = *cache {
        return Ok(registry.clone());
    }

    let registry_url =
        "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/apps.json";
    let response = get(registry_url).map_err(|e| format!("Failed to fetch app registry: {}", e))?;

    let registry_json: Value = response
        .json()
        .map_err(|e| format!("Failed to parse app registry JSON: {}", e))?;

    *cache = Some(registry_json.clone());
    Ok(registry_json)
}

pub fn get_app_configs() -> Result<Vec<(String, AppConfig)>, String> {
    let (npx_shim, uvx_path) = ensure_runtime_paths()?;

    let registry = fetch_app_registry()?;
    let apps = registry.as_array().ok_or("App registry is not an array")?;

    let mut configs = Vec::new();

    for app in apps {
        let name = app["name"]
            .as_str()
            .ok_or("App name is missing")?
            .to_string();
        let config = app["config"].as_object().ok_or("App config is missing")?;

        let mcp_key = config["mcpKey"]
            .as_str()
            .ok_or("mcpKey is missing")?
            .to_string();
        let runtime = config["runtime"].as_str().ok_or("runtime is missing")?;

        let command = match runtime {
            "npx" => npx_shim.clone(),
            "uvx" => uvx_path.clone(),
            _ => runtime.to_string(),
        };

        let args_value = config["args"].as_array().ok_or("args is missing")?;
        let args: Vec<String> = args_value
            .iter()
            .map(|arg| arg.as_str().unwrap_or("").to_string())
            .collect();

        configs.push((
            name,
            AppConfig {
                mcp_key,
                command,
                args,
            },
        ));
    }

    Ok(configs)
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
pub fn install(app_name: &str, env_vars: Option<serde_json::Value>) -> Result<String, String> {
    info!("Installing app: {}", app_name);

    ensure_runtime_paths()?;

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config()?;
        let mcp_key = config.mcp_key.clone();
        let command = config.command.clone();
        let args = config.args.clone();

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            let mut app_config = json!({
                "command": command,
                "args": args.clone(),
            });

            // Add environment variables if provided
            if let Some(env) = env_vars {
                app_config["env"] = env;
            }

            mcp_servers.insert(mcp_key.clone(), app_config);
            save_config(&config_json)?;

            std::thread::spawn(move || {
                if command.contains("npx") && args.len() > 1 {
                    let package = &args[1];
                    let _ = Command::new("npm").args(["cache", "add", package]).output();
                }
            });

            Ok(format!("Added {} configuration for {}", mcp_key, app_name))
        } else {
            Err("Failed to find mcpServers in config".to_string())
        }
    } else {
        Ok(format!("No configuration available for {}", app_name))
    }
}

#[tauri::command]
pub fn uninstall(app_name: &str) -> Result<String, String> {
    info!("Uninstalling app: {}", app_name);

    if let Some((_, config)) = get_app_configs()?.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config()?;

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            if mcp_servers.remove(&config.mcp_key).is_some() {
                save_config(&config_json)?;
                Ok(format!(
                    "Removed {} configuration for {}",
                    config.mcp_key, app_name
                ))
            } else {
                Ok(format!("Configuration for {} was not found", app_name))
            }
        } else {
            Err("Failed to find mcpServers in config".to_string())
        }
    } else {
        Ok(format!("No configuration available for {}", app_name))
    }
}

#[tauri::command]
pub fn is_installed(app_name: &str) -> Result<bool, String> {
    if let Some((_, config)) = get_app_configs()?.iter().find(|(name, _)| name == app_name) {
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
pub fn save_app_env(app_name: &str, env_values: serde_json::Value) -> Result<String, String> {
    info!("Saving ENV values for app: {}", app_name);

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config()?;
        let mcp_key = config.mcp_key.clone();

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            if let Some(server_config) = mcp_servers
                .get_mut(&mcp_key)
                .and_then(|v| v.as_object_mut())
            {
                if !server_config.contains_key("env") {
                    server_config.insert("env".to_string(), json!({}));
                }

                if let Some(env) = server_config.get_mut("env").and_then(|v| v.as_object_mut()) {
                    if let Some(values) = env_values.as_object() {
                        for (key, value) in values {
                            env.insert(key.clone(), value.clone());
                        }

                        save_config(&config_json)?;
                        return Ok(format!("Saved ENV values for app '{}'", app_name));
                    }
                    return Err("Invalid env_values format".to_string());
                }
            }
            return Err(format!("App '{}' is not installed", app_name));
        } else {
            return Err("Failed to find mcpServers in config".to_string());
        }
    } else {
        return Err(format!("No configuration available for '{}'", app_name));
    }
}

#[tauri::command]
pub fn get_app_env(app_name: &str) -> Result<Value, String> {
    info!("Getting ENV values for app: {}", app_name);

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let config_json = get_config()?;
        let mcp_key = config.mcp_key.clone();

        if let Some(mcp_servers) = config_json.get("mcpServers").and_then(|v| v.as_object()) {
            if let Some(server_config) = mcp_servers.get(&mcp_key).and_then(|v| v.as_object()) {
                if let Some(env) = server_config.get("env") {
                    return Ok(env.clone());
                }
                return Ok(json!({}));
            }
            return Err(format!("App '{}' is not installed", app_name));
        } else {
            return Err("Failed to find mcpServers in config".to_string());
        }
    } else {
        return Err(format!("No configuration available for '{}'", app_name));
    }
}

#[tauri::command]
pub fn get_app_statuses() -> Result<Value, String> {
    let config_json = get_config()?;

    let mut installed_apps = json!({});
    let mut configured_apps = json!({});

    let app_configs = get_app_configs()?;

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

#[tauri::command]
pub fn get_app_registry() -> Result<Value, String> {
    info!("Fetching app registry...");
    let result = fetch_app_registry();
    match &result {
        Ok(value) => info!("Successfully fetched app registry"),
        Err(e) => error!("Failed to fetch app registry: {}", e),
    }
    result
}
