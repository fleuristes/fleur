use crate::environment::{ensure_environment_sync, ensure_npx_shim, get_uvx_path};
use crate::file_utils::{ensure_config_file, ensure_mcp_servers};
use dirs;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use reqwest::blocking::get;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

lazy_static! {
    static ref CONFIG_CACHE: Mutex<Option<Value>> = Mutex::new(None);
    static ref TEST_CONFIG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub static ref APP_REGISTRY_CACHE: Mutex<Option<Value>> = Mutex::new(None);
    static ref ENV_SETUP_COMPLETE: Mutex<bool> = Mutex::new(false);
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
    debug!(
        "Getting config path, test_mode: {}",
        crate::environment::is_test_mode()
    );

    // Check if we have a test config path set
    let test_path = TEST_CONFIG_PATH.lock().unwrap();
    if let Some(path) = test_path.clone() {
        debug!("Using test config path: {}", path.display());
        return Ok(path);
    }

    // Otherwise use the default path
    let default_path = dirs::home_dir()
        .ok_or("Could not find home directory".to_string())?
        .join("Library/Application Support/Claude/claude_desktop_config.json");

    debug!("Using default config path: {}", default_path.display());
    Ok(default_path)
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub mcp_key: String,
    pub command: String,
    pub args: Vec<String>,
}

fn fetch_app_registry() -> Result<Value, String> {
    // Check if we have a cached registry
    let mut cache = APP_REGISTRY_CACHE.lock().unwrap();
    if let Some(ref registry) = *cache {
        debug!("Using cached app registry");
        return Ok(registry.clone());
    }

    // Fetch the registry from GitHub
    let registry_url =
        "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/apps.json";
    info!("Fetching app registry from {}", registry_url);
    let response = get(registry_url).map_err(|e| {
        error!("Failed to fetch app registry: {}", e);
        format!("Failed to fetch app registry: {}", e)
    })?;

    let registry_json: Value = response.json().map_err(|e| {
        error!("Failed to parse app registry JSON: {}", e);
        format!("Failed to parse app registry JSON: {}", e)
    })?;

    // Cache the registry
    *cache = Some(registry_json.clone());
    info!("Successfully fetched and cached app registry");
    Ok(registry_json)
}

// Function to ensure environment is set up before getting app configs
fn ensure_env_setup() -> Result<(), String> {
    // Skip for test mode
    if crate::environment::is_test_mode() {
        return Ok(());
    }

    // Check if environment setup is already marked as complete
    let mut setup_complete = ENV_SETUP_COMPLETE.lock().unwrap();
    if *setup_complete {
        debug!("Environment already set up");
        return Ok(());
    }

    // Ensure environment is set up synchronously
    info!("Ensuring environment is set up before fetching app configs");
    ensure_environment_sync()?;

    // Mark setup as complete
    *setup_complete = true;
    Ok(())
}

pub fn get_app_configs() -> Result<Vec<(String, AppConfig)>, String> {
    debug!(
        "Getting app configurations, test_mode: {}",
        crate::environment::is_test_mode()
    );

    // Ensure environment is set up first (skip in test mode)
    ensure_env_setup()?;

    // In test mode, use test paths directly
    let (npx_shim, uvx_path) = if crate::environment::is_test_mode() {
        debug!("Using test paths for npx_shim and uvx_path");
        (
            "/test/.local/share/fleur/bin/npx-fleur".to_string(),
            "/test/.local/bin/uvx".to_string(),
        )
    } else {
        // Get absolute paths, and fail if they can't be obtained
        let npx_shim = ensure_npx_shim()?;
        let uvx_path = get_uvx_path()?;
        (npx_shim, uvx_path)
    };

    info!("Using npx_shim: {}", npx_shim);
    info!("Using uvx_path: {}", uvx_path);

    let registry = fetch_app_registry()?;
    let apps = registry.as_array().ok_or_else(|| {
        let err = "App registry is not an array".to_string();
        error!("{}", err);
        err
    })?;

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

        debug!(
            "Configured app '{}' with command: '{}', args: {:?}",
            name, command, args
        );
        configs.push((
            name,
            AppConfig {
                mcp_key,
                command,
                args,
            },
        ));
    }

    info!("Successfully configured {} apps", configs.len());
    Ok(configs)
}

pub fn get_config() -> Result<Value, String> {
    debug!(
        "Getting config, test_mode: {}",
        crate::environment::is_test_mode()
    );

    let mut cache = CONFIG_CACHE.lock().unwrap();
    if let Some(ref config) = *cache {
        debug!("Using cached config");
        return Ok(config.clone());
    }

    let config_path = get_config_path()?;
    debug!("Using config path: {}", config_path.display());

    if !config_path.exists() {
        info!("Config file does not exist, creating it");
        ensure_config_file(&config_path)?;
    }

    let config_str = fs::read_to_string(&config_path).map_err(|e| {
        error!("Failed to read config file: {}", e);
        format!("Failed to read config file: {}", e)
    })?;

    let mut config_json: Value = serde_json::from_str(&config_str).map_err(|e| {
        error!("Failed to parse config JSON: {}", e);
        format!("Failed to parse config JSON: {}", e)
    })?;

    ensure_mcp_servers(&mut config_json)?;

    *cache = Some(config_json.clone());
    debug!("Config loaded and cached successfully");
    Ok(config_json)
}

pub fn save_config(config: &Value) -> Result<(), String> {
    let config_path = get_config_path()?;
    debug!("Saving config to {}", config_path.display());

    let updated_config = serde_json::to_string_pretty(config).map_err(|e| {
        error!("Failed to serialize config: {}", e);
        format!("Failed to serialize config: {}", e)
    })?;

    fs::write(&config_path, updated_config).map_err(|e| {
        error!("Failed to write config file: {}", e);
        format!("Failed to write config file: {}", e)
    })?;

    // Update cache
    let mut cache = CONFIG_CACHE.lock().unwrap();
    *cache = Some(config.clone());
    info!("Config saved successfully");

    Ok(())
}

#[tauri::command]
pub fn restart_claude_app() -> Result<String, String> {
    info!("Restarting Claude app...");

    // Kill the Claude app
    Command::new("pkill")
        .arg("-x")
        .arg("Claude")
        .output()
        .map_err(|e| format!("Failed to kill Claude app: {}", e))?;

    // Wait a moment to ensure it's fully closed
    sleep(Duration::from_millis(500));

    // Relaunch the app
    Command::new("open")
        .arg("-a")
        .arg("Claude")
        .output()
        .map_err(|e| format!("Failed to relaunch Claude app: {}", e))?;

    Ok("Claude app restarted successfully".to_string())
}

#[tauri::command]
pub fn preload_dependencies() -> Result<(), String> {
    info!("Preloading dependencies");
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
    debug!(
        "Install called in test mode: {}",
        crate::environment::is_test_mode()
    );

    // Ensure environment is set up first
    ensure_env_setup()?;

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config()?;
        let mcp_key = config.mcp_key.clone();
        let command = config.command.clone();
        let args = config.args.clone();

        debug!(
            "Installing {} with command: {}, args: {:?}",
            app_name, command, args
        );

        // Skip path validation entirely in test mode
        if !crate::environment::is_test_mode() {
            if !std::path::Path::new(&command).exists() {
                error!(
                    "Command path '{}' for app '{}' does not exist",
                    command, app_name
                );
                return Err(format!(
                    "Command path '{}' for app '{}' does not exist",
                    command, app_name
                ));
            }
        } else {
            debug!("Test mode: skipping path validation for {}", command);
        }

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

            debug!("Adding config for {}: {:?}", mcp_key, app_config);
            mcp_servers.insert(mcp_key.clone(), app_config);
            save_config(&config_json)?;

            // Only attempt to pre-cache npm packages if not in test mode
            if !crate::environment::is_test_mode() {
                std::thread::spawn(move || {
                    if command.contains("npx") && args.len() > 1 {
                        let package = &args[1];
                        info!("Pre-caching npm package: {}", package);
                        let _ = Command::new("npm").args(["cache", "add", package]).output();
                    }
                });
            }

            info!("Successfully installed app: {}", app_name);

            Ok(format!("Added {} configuration for {}", mcp_key, app_name))
        } else {
            let err = "Failed to find mcpServers in config".to_string();
            error!("{}", err);
            Err(err)
        }
    } else {
        let err = format!("No configuration available for: {}", app_name);
        warn!("{}", err);
        Ok(err)
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
                info!("Successfully uninstalled app: {}", app_name);

                Ok(format!(
                    "Removed {} configuration for {}",
                    config.mcp_key, app_name
                ))
            } else {
                warn!("Configuration for {} was not found", app_name);
                Ok(format!("Configuration for {} was not found", app_name))
            }
        } else {
            let err = "Failed to find mcpServers in config".to_string();
            error!("{}", err);
            Err(err)
        }
    } else {
        warn!("No configuration available for: {}", app_name);
        Ok(format!("No configuration available for {}", app_name))
    }
}

#[tauri::command]
pub fn is_installed(app_name: &str) -> Result<bool, String> {
    debug!("Checking if app is installed: {}", app_name);
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

    // Ensure environment is set up first
    ensure_env_setup()?;

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
                // Create ENV object if it doesn't exist
                if !server_config.contains_key("env") {
                    server_config.insert("env".to_string(), json!({}));
                }

                // Add or update all key-value pairs in ENV
                if let Some(env) = server_config.get_mut("env").and_then(|v| v.as_object_mut()) {
                    if let Some(values) = env_values.as_object() {
                        for (key, value) in values {
                            env.insert(key.clone(), value.clone());
                        }

                        save_config(&config_json)?;
                        info!("Successfully saved ENV values for app: {}", app_name);
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
    debug!("Getting ENV values for app: {}", app_name);

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
    debug!(
        "Getting app statuses, test_mode: {}",
        crate::environment::is_test_mode()
    );

    // Ensure environment is set up before getting statuses
    ensure_env_setup()?;

    let config_json = get_config()?;
    let mut installed_apps = json!({});
    let mut configured_apps = json!({});

    let app_configs = match get_app_configs() {
        Ok(configs) => configs,
        Err(e) => {
            // Log the error but return an empty status rather than failing
            error!("Failed to get app configs: {}. Returning empty status.", e);
            return Ok(json!({
                "installed": {},
                "configured": {}
            }));
        }
    };

    if let Some(mcp_servers) = config_json.get("mcpServers").and_then(|v| v.as_object()) {
        for (app_name, config) in app_configs {
            installed_apps[&app_name] = json!(mcp_servers.contains_key(&config.mcp_key));
            configured_apps[&app_name] = json!(!config.command.is_empty());
        }
    }

    debug!(
        "Retrieved app statuses: installed={:?}, configured={:?}",
        installed_apps, configured_apps
    );
    Ok(json!({
        "installed": installed_apps,
        "configured": configured_apps
    }))
}

// New function to expose the app registry to the frontend
#[tauri::command]
pub fn get_app_registry() -> Result<Value, String> {
    info!("Fetching app registry...");
    let result = fetch_app_registry();
    match &result {
        Ok(value) => info!("Successfully fetched app registry: {}", value),
        Err(e) => error!("Failed to fetch app registry: {}", e),
    }
    result
}
