use crate::os::OSType;
use dirs;
use lazy_static::lazy_static;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use crate::environment::CREATE_NO_WINDOW;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClientType {
    Claude,
    Cursor,
    Windsurf,
}

impl ClientType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientType::Claude => "Claude",
            ClientType::Cursor => "Cursor",
            ClientType::Windsurf => "Windsurf",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Claude" => Some(ClientType::Claude),
            "Cursor" => Some(ClientType::Cursor),
            "Windsurf" => Some(ClientType::Windsurf),
            _ => None,
        }
    }

    pub fn all() -> Vec<ClientType> {
        vec![ClientType::Claude, ClientType::Cursor, ClientType::Windsurf]
    }

    pub fn all_as_str() -> Vec<&'static str> {
        vec!["Claude", "Cursor", "Windsurf"]
    }

    pub fn default() -> ClientType {
        ClientType::Claude
    }
}

#[derive(Clone, Debug)]
pub struct ClientPathConfig {
    pub base_dir: PathBuf,
    pub config_filename: String,
    pub os: OSType,
}

lazy_static! {
    static ref CLIENT_PATH_CONFIGS: Mutex<std::collections::HashMap<ClientType, ClientPathConfig>> =
        Mutex::new(std::collections::HashMap::new());
}

pub fn init_client_path_configs() {
    let mut configs = CLIENT_PATH_CONFIGS.lock().unwrap();

    if configs.is_empty() {
        if let Some(home_dir) = dirs::home_dir() {
            #[cfg(target_os = "macos")]
            {
                configs.insert(
                    ClientType::Claude,
                    ClientPathConfig {
                        base_dir: home_dir.join("Library/Application Support/Claude"),
                        config_filename: "claude_desktop_config.json".to_string(),
                        os: OSType::MacOS,
                    },
                );

                configs.insert(
                    ClientType::Cursor,
                    ClientPathConfig {
                        base_dir: home_dir.join(".cursor/"),
                        config_filename: "mcp.json".to_string(),
                        os: OSType::MacOS,
                    },
                );

                configs.insert(
                    ClientType::Windsurf,
                    ClientPathConfig {
                        base_dir: home_dir.join(".codeium/windsurf"),
                        config_filename: "mcp_config.json".to_string(),
                        os: OSType::MacOS,
                    },
                );
            }

            #[cfg(target_os = "windows")]
            {
                let appdata_roaming =
                    dirs::config_dir().unwrap_or_else(|| home_dir.join("AppData/Roaming"));
                configs.insert(
                    ClientType::Claude,
                    ClientPathConfig {
                        base_dir: appdata_roaming.join("Claude"),
                        config_filename: "claude_desktop_config.json".to_string(),
                        os: OSType::Windows,
                    },
                );

                configs.insert(
                    ClientType::Cursor,
                    ClientPathConfig {
                        base_dir: home_dir.join(".cursor"),
                        config_filename: "mcp.json".to_string(),
                        os: OSType::Windows,
                    },
                );

                configs.insert(
                    ClientType::Windsurf,
                    ClientPathConfig {
                        base_dir: home_dir.join(".codeium\\windsurf"),
                        config_filename: "mcp_config.json".to_string(),
                        os: OSType::Windows,
                    },
                );
            }
        }
    }
}

pub fn get_client_path_config(client: &ClientType) -> Result<ClientPathConfig, String> {
    init_client_path_configs();

    let configs = CLIENT_PATH_CONFIGS.lock().unwrap();
    if let Some(config) = configs.get(client) {
        Ok(config.clone())
    } else {
        Err(format!(
            "No path configuration for client: {}",
            client.as_str()
        ))
    }
}

pub fn set_client_path_config(client: &ClientType, config: ClientPathConfig) -> Result<(), String> {
    if ClientType::from_str(client.as_str()).is_none() {
        return Err(format!("Unsupported client: {}", client.as_str()));
    }

    let mut configs = CLIENT_PATH_CONFIGS.lock().unwrap();
    configs.insert(client.clone(), config);

    debug!("Updated path configuration for client: {}", client.as_str());
    Ok(())
}

pub fn validate_client(client: &ClientType) -> Result<(), String> {
    if ClientType::from_str(client.as_str()).is_none() {
        return Err(format!("Unsupported client: {}", client.as_str()));
    }
    Ok(())
}

pub fn get_default_client() -> ClientType {
    ClientType::default()
}

pub fn check_client_installed(client: &ClientType) -> Result<bool, String> {
    validate_client(client)?;

    #[cfg(target_os = "macos")]
    {
        let app_path = std::path::PathBuf::from(format!("/Applications/{}.app", client.as_str()));
        debug!(
            "Checking for {}.app at: {}",
            client.as_str(),
            app_path.display()
        );
        return Ok(app_path.exists());
    }

    #[cfg(target_os = "windows")]
    {
        if *client == ClientType::Claude {
            if let Some(local_app_data) = dirs::data_local_dir() {
                let app_dir = local_app_data.join("AnthropicClaude");
                let exe_path = app_dir.join("claude.exe");

                debug!("Checking for claude.exe at: {}", exe_path.display());

                let exists = exe_path.exists();
                info!(
                    "Claude {} at {}",
                    if exists { "found" } else { "not found" },
                    exe_path.display()
                );
                return Ok(exists);
            }
            info!("Claude not found - could not locate AppData directory");
            return Ok(false);
        }

        if *client == ClientType::Cursor {
            // Check common installation locations for Cursor
            if let Some(program_files) = dirs::data_local_dir() {
                let home_dir = dirs::home_dir().unwrap_or_default();
                let possible_paths = [
                    program_files.join("Programs\\Cursor\\Cursor.exe"),
                    home_dir.join("AppData\\Local\\Programs\\Cursor\\Cursor.exe"),
                ];
                
                for path in &possible_paths {
                    debug!("Checking for Cursor at: {}", path.display());
                    if path.exists() {
                        info!("Cursor found at {}", path.display());
                        return Ok(true);
                    }
                }
            }
            info!("Cursor not found in common locations");
            return Ok(false);
        } else if *client == ClientType::Windsurf {
            // Check common installation locations for Windsurf
            if let Some(program_files) = dirs::data_local_dir() {
                let home_dir = dirs::home_dir().unwrap_or_default();
                let possible_paths = [
                    program_files.join("Programs\\Windsurf\\Windsurf.exe"),
                    home_dir.join("AppData\\Local\\Programs\\Windsurf\\Windsurf.exe"),
                ];
                
                for path in &possible_paths {
                    debug!("Checking for Windsurf at: {}", path.display());
                    if path.exists() {
                        info!("Windsurf found at {}", path.display());
                        return Ok(true);
                    }
                }
            }
            info!("Windsurf not found in common locations");
            return Ok(false);
        }
        
        debug!("Unknown client type for Windows: {}", client.as_str());
        return Ok(false);
    }
}

pub fn restart_client_app(client: &ClientType) -> Result<String, String> {
    validate_client(client)?;

    info!("Restarting {} app...", client.as_str());

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("pkill")
            .arg("-x")
            .arg(client.as_str())
            .output()
            .map_err(|e| format!("Failed to kill {} app: {}", client.as_str(), e))?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        std::process::Command::new("open")
            .arg("-a")
            .arg(client.as_str())
            .output()
            .map_err(|e| format!("Failed to relaunch {} app: {}", client.as_str(), e))?;

        return Ok(format!("{} app restarted successfully", client.as_str()));
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", &format!("{}.exe", client.as_str())])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        std::thread::sleep(std::time::Duration::from_millis(1000));

        if *client == ClientType::Claude {
            if let Some(local_app_data) = dirs::data_local_dir() {
                let app_dir = local_app_data.join("AnthropicClaude");
                let exe_path = app_dir.join("claude.exe");

                if exe_path.exists() {
                    info!("Claude executable found at: {}", exe_path.display());
                    Command::new(&exe_path)
                        .creation_flags(CREATE_NO_WINDOW)
                        .spawn()
                        .map_err(|e| format!("Failed to restart Claude: {}", e))?;
                    return Ok("Claude app restarted successfully".to_string());
                }

                info!("Claude executable not found at: {}", exe_path.display());
                return Err("Could not find claude.exe to restart".to_string());
            }
        }

        if *client == ClientType::Cursor {
            if let Some(program_files) = dirs::data_local_dir() {
                let home_dir = dirs::home_dir().unwrap_or_default();
                let possible_paths = [
                    program_files.join("Programs\\Cursor\\Cursor.exe"),
                    home_dir.join("AppData\\Local\\Programs\\Cursor\\Cursor.exe"),
                ];
                
                for path in &possible_paths {
                    if path.exists() {
                        info!("Cursor executable found at: {}", path.display());
                        Command::new(&path)
                            .creation_flags(CREATE_NO_WINDOW)
                            .spawn()
                            .map_err(|e| format!("Failed to restart Cursor: {}", e))?;
                        return Ok("Cursor app restarted successfully".to_string());
                    }
                }
                
                return Err("Could not find Cursor.exe to restart".to_string());
            }
        } else if *client == ClientType::Windsurf {
            if let Some(program_files) = dirs::data_local_dir() {
                let home_dir = dirs::home_dir().unwrap_or_default();
                let possible_paths = [
                    program_files.join("Programs\\Windsurf\\Windsurf.exe"),
                    home_dir.join("AppData\\Local\\Programs\\Windsurf\\Windsurf.exe"),
                ];
                
                for path in &possible_paths {
                    if path.exists() {
                        info!("Windsurf executable found at: {}", path.display());
                        Command::new(&path)
                            .creation_flags(CREATE_NO_WINDOW)
                            .spawn()
                            .map_err(|e| format!("Failed to restart Windsurf: {}", e))?;
                        return Ok("Windsurf app restarted successfully".to_string());
                    }
                }
                
                return Err("Could not find Windsurf.exe to restart".to_string());
            }
        }
        
        return Err(format!(
            "Restart not implemented for client: {}",
            client.as_str()
        ));
    }
}
