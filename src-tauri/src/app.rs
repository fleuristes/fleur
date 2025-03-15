use crate::environment::{ensure_environment_sync, ensure_npx_shim, get_uvx_path};
use crate::file_utils::{ensure_config_file, ensure_mcp_servers};
use crate::clients::{self, ClientType, ClientPathConfig};
use dirs;
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use reqwest::blocking::get;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use regex;

lazy_static! {
    static ref CONFIG_CACHE: Mutex<std::collections::HashMap<String, Value>> = Mutex::new(std::collections::HashMap::new());
    static ref TEST_CONFIG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub static ref APP_REGISTRY_CACHE: Mutex<Option<Value>> = Mutex::new(None);
    static ref ENV_SETUP_COMPLETE: Mutex<bool> = Mutex::new(false);
}

// Initialize default path configurations for supported clients
pub fn init_client_path_configs() {
    clients::init_client_path_configs();
}

pub fn set_test_config_path(path: Option<PathBuf>) {
    let mut test_path = TEST_CONFIG_PATH.lock().unwrap();
    *test_path = path;

    // Clear the cache when changing the config path
    let mut cache = CONFIG_CACHE.lock().unwrap();
    cache.clear();

    debug!("Test config path set and cache cleared");
}

pub fn get_default_client() -> String {
    clients::get_default_client()
}

pub fn validate_client(client_name: &str) -> Result<(), String> {
    clients::validate_client(client_name)
}

fn get_config_path(client: &str) -> Result<PathBuf, String> {
    debug!(
        "Getting config path for client {}, test_mode: {}",
        client, crate::environment::is_test_mode()
    );

    // Validate client
    clients::validate_client(client)?;

    // Check if we have a test config path set
    let test_path = TEST_CONFIG_PATH.lock().unwrap();
    if let Some(path) = test_path.clone() {
        debug!("Using test config path: {}", path.display());
        return Ok(path);
    }

    // Get the client-specific path configuration
    let path_config = clients::get_client_path_config(client)?;

    // Construct the full path
    let config_path = path_config.base_dir.join(&path_config.config_filename);

    debug!("Using config path for {}: {}", client, config_path.display());
    Ok(config_path)
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
        "https://raw.githubusercontent.com/pranav7/app-registry/p7/12.03.25/browser-mcp/apps.json";
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

fn replace_env_vars(input: &str, env: &serde_json::Value) -> String {
    let mut result = input.to_string();

    // Find all ${...} patterns
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

    // Collect all matches first to avoid modifying the string while iterating
    let matches: Vec<(String, String)> = re.captures_iter(&result)
        .filter_map(|captures| {
            let full_match = captures.get(0)?.as_str().to_string();
            let var_name = captures.get(1)?.as_str().to_string();
            Some((full_match, var_name))
        })
        .collect();

    // Process each match
    for (full_match, var_name) in matches {
        // Look up the variable in the env object
        if let Some(value) = env.get(&var_name) {
            // Convert the value to a string representation
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => full_match.clone(), // Keep original for other types
            };

            // Replace this occurrence
            result = result.replace(&full_match, &replacement);
        }
        // If variable not found, leave the original ${VAR_NAME} in place
    }

    result
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

pub fn get_config(client_opt: Option<&str>) -> Result<Value, String> {
    // Use the default client if none is provided
    let client = client_opt.unwrap_or(clients::ClientType::default().as_str());

    debug!(
        "Getting config for client {}, test_mode: {}",
        client, crate::environment::is_test_mode()
    );

    // Validate client
    validate_client(client)?;

    let mut cache = CONFIG_CACHE.lock().unwrap();
    if let Some(config) = cache.get(client) {
        debug!("Using cached config for client {}", client);
        return Ok(config.clone());
    }

    let config_path = get_config_path(client)?;
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

    cache.insert(client.to_string(), config_json.clone());
    debug!("Config for client {} loaded and cached successfully", client);
    Ok(config_json)
}

pub fn save_config(config: &Value, client_opt: Option<&str>) -> Result<(), String> {
    // Use the default client if none is provided
    let client = client_opt.unwrap_or(clients::ClientType::default().as_str());

    // Validate client
    validate_client(client)?;

    let config_path = get_config_path(client)?;
    debug!("Saving config for client {} to {}", client, config_path.display());

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
    cache.insert(client.to_string(), config.clone());
    info!("Config for client {} saved successfully", client);

    Ok(())
}

#[tauri::command]
pub fn restart_client_app(client_name: Option<&str>) -> Result<String, String> {
    clients::restart_client_app(client_name)
}

// Keep the old function for backward compatibility, but have it call the new function
#[tauri::command]
pub fn restart_claude_app() -> Result<String, String> {
    restart_client_app(Some("Claude"))
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
pub fn install(app_name: &str, env_vars: Option<serde_json::Value>, client: Option<&str>) -> Result<String, String> {
    info!("Installing app: {} for client: {:?}", app_name, client);
    debug!(
        "Install called in test mode: {}",
        crate::environment::is_test_mode()
    );

    // Ensure environment is set up first
    ensure_env_setup()?;

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config(client)?;
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
            // Get existing environment variables for this app if any
            let existing_env = if let Some(server_config) = mcp_servers.get(&mcp_key) {
                if let Some(env) = server_config.get("env") {
                    env.clone()
                } else {
                    json!({})
                }
            } else {
                json!({})
            };

            // Merge with provided env_vars if any
            let env = if let Some(new_env) = env_vars {
                let mut merged = existing_env.as_object().unwrap_or(&serde_json::Map::new()).clone();
                for (k, v) in new_env.as_object().unwrap_or(&serde_json::Map::new()) {
                    merged.insert(k.clone(), v.clone());
                }
                serde_json::Value::Object(merged)
            } else {
                existing_env
            };

            // Process args to replace environment variables
            let processed_args = args.iter()
                .map(|arg| replace_env_vars(arg, &env))
                .collect::<Vec<String>>();

            let app_config = json!({
                "command": command,
                "args": processed_args,
                "env": env
            });

            debug!("Adding config for {}: {:?}", mcp_key, app_config);
            mcp_servers.insert(mcp_key.clone(), app_config);
            save_config(&config_json, client)?;

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

            let client_str = client.unwrap_or(clients::ClientType::default().as_str());
            info!("Successfully installed app: {} for client: {}", app_name, client_str);

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
pub fn uninstall(app_name: &str, client: Option<&str>) -> Result<String, String> {
    info!("Uninstalling app: {} for client: {:?}", app_name, client);

    if let Some((_, config)) = get_app_configs()?.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config(client)?;

        if let Some(mcp_servers) = config_json
            .get_mut("mcpServers")
            .and_then(|v| v.as_object_mut())
        {
            if mcp_servers.remove(&config.mcp_key).is_some() {
                save_config(&config_json, client)?;
                let client_str = client.unwrap_or(clients::ClientType::default().as_str());
                info!("Successfully uninstalled app: {} for client: {}", app_name, client_str);

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
pub fn is_installed(app_name: &str, client: Option<&str>) -> Result<bool, String> {
    debug!("Checking if app is installed: {} for client: {:?}", app_name, client);
    if let Some((_, config)) = get_app_configs()?.iter().find(|(name, _)| name == app_name) {
        let config_json = get_config(client)?;

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
pub fn save_app_env(app_name: &str, env_values: serde_json::Value, client: Option<&str>) -> Result<String, String> {
    info!("Saving ENV values for app: {} for client: {:?}", app_name, client);

    // Ensure environment is set up first
    ensure_env_setup()?;

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let mut config_json = get_config(client)?;
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

                        save_config(&config_json, client)?;
                        let client_str = client.unwrap_or(clients::ClientType::default().as_str());
                        info!("Successfully saved ENV values for app: {} for client: {}", app_name, client_str);
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
pub fn get_app_env(app_name: &str, client: Option<&str>) -> Result<Value, String> {
    debug!("Getting ENV values for app: {} for client: {:?}", app_name, client);

    let configs = get_app_configs()?;
    if let Some((_, config)) = configs.iter().find(|(name, _)| name == app_name) {
        let config_json = get_config(client)?;
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
pub fn get_app_statuses(client: Option<&str>) -> Result<Value, String> {
    debug!(
        "Getting app statuses for client: {:?}, test_mode: {}",
        client, crate::environment::is_test_mode()
    );

    // Ensure environment is set up before getting statuses
    ensure_env_setup()?;

    let config_json = get_config(client)?;
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

    let client_str = client.unwrap_or(clients::ClientType::default().as_str());
    debug!(
        "Retrieved app statuses for client {}: installed={:?}, configured={:?}",
        client_str, installed_apps, configured_apps
    );
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
        Ok(value) => info!("Successfully fetched app registry: {}", value),
        Err(e) => error!("Failed to fetch app registry: {}", e),
    }
    result
}

#[tauri::command]
pub fn refresh_app_registry() -> Result<Value, String> {
    info!("Refreshing app registry...");

    // Clear the cache
    {
        let mut cache = APP_REGISTRY_CACHE.lock().unwrap();
        *cache = None;
        info!("App registry cache cleared");
    }

    // Fetch fresh registry
    let result = fetch_app_registry();
    match &result {
        Ok(value) => info!("Successfully refreshed app registry: {}", value),
        Err(e) => error!("Failed to refresh app registry: {}", e),
    }
    result
}

#[tauri::command]
pub fn install_fleur_mcp(client: Option<&str>) -> Result<String, String> {
    info!("Installing fleur-mcp for client: {:?}...", client);

    let mut config_json = get_config(client)?;
    let uvx_path = get_uvx_path()?;

    if let Some(mcp_servers) = config_json
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
    {
        let app_config = json!({
            "command": uvx_path,
            "args": ["--from", "git+https://github.com/fleuristes/fleur-mcp", "fleur-mcp"]
        });

        debug!("Adding config for fleur: {:?}", app_config);
        mcp_servers.insert("fleur".to_string(), app_config);
        save_config(&config_json, client)?;

        let client_str = client.unwrap_or(clients::ClientType::default().as_str());
        info!("Successfully installed fleur-mcp for client: {}", client_str);
        Ok("Added fleur-mcp configuration".to_string())
    } else {
        let err = "Failed to find mcpServers in config".to_string();
        error!("{}", err);
        Err(err)
    }
}

#[tauri::command]
pub fn uninstall_fleur_mcp(client: Option<&str>) -> Result<String, String> {
    info!("Uninstalling fleur-mcp for client: {:?}...", client);

    let mut config_json = get_config(client)?;

    if let Some(mcp_servers) = config_json
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
    {
        if mcp_servers.remove("fleur").is_some() {
            save_config(&config_json, client)?;
            let client_str = client.unwrap_or(clients::ClientType::default().as_str());
            info!("Successfully uninstalled fleur-mcp for client: {}", client_str);
            Ok("Removed fleur-mcp configuration".to_string())
        } else {
            warn!("fleur-mcp configuration was not found");
            Ok("fleur-mcp configuration was not found".to_string())
        }
    } else {
        let err = "Failed to find mcpServers in config".to_string();
        error!("{}", err);
        Err(err)
    }
}

#[tauri::command]
pub fn check_onboarding_completed() -> Result<bool, String> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None => return Err("Could not determine home directory".to_string()),
    };
    let onboarding_file = home.join(".fleur/onboarding_completed");

    debug!("Checking onboarding file at: {}", onboarding_file.display());
    Ok(onboarding_file.exists())
}

#[tauri::command]
pub fn reset_onboarding_completed() -> Result<bool, String> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None => return Err("Could not determine home directory".to_string()),
    };
    let onboarding_file = home.join(".fleur/onboarding_completed");

    debug!("Resetting onboarding file at: {}", onboarding_file.display());
    if onboarding_file.exists() {
        std::fs::remove_file(&onboarding_file).map_err(|e| format!("Failed to remove onboarding file: {}", e))?;
    }

    Ok(true)
}

#[tauri::command]
pub fn check_client_installed(client_name: Option<&str>) -> Result<bool, String> {
    clients::check_client_installed(client_name)
}

// Keep these functions for backward compatibility, but have them call the new function
#[tauri::command]
pub fn check_claude_installed() -> Result<bool, String> {
    check_client_installed(Some("Claude"))
}

#[tauri::command]
pub fn check_cursor_installed() -> Result<bool, String> {
    check_client_installed(Some("Cursor"))
}

#[tauri::command]
pub fn get_supported_clients() -> Vec<String> {
    ClientType::all_as_str().iter().map(|&s| s.to_string()).collect()
}

#[tauri::command]
pub fn get_default_client_command() -> String {
    get_default_client()
}

#[tauri::command]
pub fn set_client_config_path(client: &str, base_dir: &str, config_filename: &str) -> Result<String, String> {
    // Validate client
    clients::validate_client(client)?;

    // Create path from string
    let base_path = std::path::PathBuf::from(base_dir);

    // Create the configuration
    let config = ClientPathConfig {
        base_dir: base_path,
        config_filename: config_filename.to_string(),
    };

    // Set the configuration
    clients::set_client_path_config(client, config)?;

    // Clear the cache for this client
    let mut cache = CONFIG_CACHE.lock().unwrap();
    cache.remove(client);

    info!("Updated path configuration for client {}: base_dir={}, config_filename={}",
          client, base_dir, config_filename);

    Ok(format!("Successfully updated path configuration for {}", client))
}

#[tauri::command]
pub fn get_client_config_path(client: &str) -> Result<Value, String> {
    // Validate client
    clients::validate_client(client)?;

    // Get the client's path configuration
    let config = clients::get_client_path_config(client)?;

    // Convert to JSON
    let result = json!({
        "base_dir": config.base_dir.to_string_lossy(),
        "config_filename": config.config_filename
    });

    Ok(result)
}
