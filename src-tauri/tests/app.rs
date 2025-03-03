mod common;

use fleur_lib::app::{self, get_app_configs, set_test_config_path};
use serde_json::Value;
use serial_test::serial;
use std::{thread, time::Duration};
use tempfile;
use uuid::Uuid;

#[test]
fn test_get_app_configs() {
    let configs = get_app_configs();
    let browser = configs
        .iter()
        .find(|(name, _)| name == "Browser")
        .expect("Browser app not found");
    assert_eq!(browser.1.mcp_key, "puppeteer");
}

#[tokio::test]
async fn test_app_installation() {
    // Create a direct test with a unique ID
    let test_id = Uuid::new_v4().to_string();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir
        .path()
        .join(format!("test_config_{}.json", test_id));

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // Create initial config
    let initial_config = serde_json::json!({
        "mcpServers": {}
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&initial_config).unwrap(),
    )
    .unwrap();

    // Set the test config path
    set_test_config_path(Some(config_path.clone()));

    // Install the app
    let result = app::install("Browser".to_string()).await;
    assert!(result.is_ok(), "Installation should succeed");

    // Force a fresh read from disk
    set_test_config_path(Some(config_path.clone()));

    // Read directly from the file to verify it was updated
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let config: Value = serde_json::from_str(&config_str).unwrap();

    // Check if mcpServers exists and is an object
    assert!(config["mcpServers"].is_object(), "mcpServers should be an object");

    // Check if puppeteer key exists and has expected values
    let puppeteer = &config["mcpServers"]["puppeteer"];
    assert!(puppeteer.is_object(), "Puppeteer config should be an object");
    assert_eq!(puppeteer["command"].as_str().unwrap(), "npx", "Command should be 'npx'");
    assert!(puppeteer["args"].is_array(), "Args should be an array");
    assert_eq!(
        puppeteer["args"][0].as_str().unwrap(),
        "-y",
        "First arg should be '-y'"
    );

    // Reset the test config path
    set_test_config_path(None);
}

#[tokio::test]
async fn test_app_uninstallation() {
    // Create a direct test with a unique ID
    let test_id = Uuid::new_v4().to_string();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir
        .path()
        .join(format!("test_config_{}.json", test_id));

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // Create initial config with puppeteer already installed
    let initial_config = serde_json::json!({
        "mcpServers": {
            "puppeteer": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-puppeteer", "--debug"]
            }
        }
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&initial_config).unwrap(),
    )
    .unwrap();

    // Set the test config path
    set_test_config_path(Some(config_path.clone()));

    // Verify initial state
    let config = app::get_config().unwrap();
    assert!(config["mcpServers"]["puppeteer"].is_object());

    // Uninstall the app
    let result = app::uninstall("Browser".to_string()).await;
    assert!(result.is_ok());

    // Force a fresh read from disk
    set_test_config_path(Some(config_path.clone()));

    // Read directly from the file to verify it was updated
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let config: Value = serde_json::from_str(&config_str).unwrap();

    // Verify config was removed
    assert!(!config["mcpServers"].as_object().unwrap().contains_key("puppeteer"));

    // Reset the test config path
    set_test_config_path(None);
}

#[tokio::test]
async fn test_app_status() {
    // Create a direct test with a unique ID
    let test_id = Uuid::new_v4().to_string();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir
        .path()
        .join(format!("test_config_{}.json", test_id));

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // Create initial config
    let initial_config = serde_json::json!({
        "mcpServers": {}
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&initial_config).unwrap(),
    )
    .unwrap();

    // Set the test config path
    set_test_config_path(Some(config_path.clone()));

    // Test initial status
    let result = app::get_app_statuses().unwrap();
    assert!(result["installed"].is_object(), "installed should be an object");
    assert!(result["configured"].is_object(), "configured should be an object");
    assert!(
        !result["installed"]["Browser"].as_bool().unwrap_or(true),
        "Browser should not be installed initially"
    );

    // Install and check status
    app::install("Browser".to_string()).await.unwrap();

    // Force a fresh read
    set_test_config_path(Some(config_path.clone()));

    // Read directly from the file to verify it was updated
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let config: Value = serde_json::from_str(&config_str).unwrap();
    assert!(
        config["mcpServers"]["puppeteer"].is_object(),
        "Puppeteer config should exist and be an object"
    );

    let result = app::get_app_statuses().unwrap();
    assert!(
        result["installed"]["Browser"].as_bool().unwrap(),
        "Browser should be installed after installation"
    );

    // Reset the test config path
    set_test_config_path(None);
}
