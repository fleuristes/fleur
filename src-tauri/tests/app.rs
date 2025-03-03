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
    assert!(result.is_ok());

    // Wait and verify config file
    thread::sleep(Duration::from_millis(100));

    // Read directly from the file to verify it was updated
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let config: Value = serde_json::from_str(&config_str).unwrap();

    // Check if puppeteer key exists and has expected values
    let puppeteer = &config["mcpServers"]["puppeteer"];
    assert!(
        puppeteer.is_object(),
        "Puppeteer config should be an object"
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

    // Uninstall the app
    let result = app::uninstall("Browser".to_string()).await;
    assert!(result.is_ok());

    // Wait and verify config file
    thread::sleep(Duration::from_millis(100));

    // Verify config was removed
    let config_str = std::fs::read_to_string(&config_path).unwrap();
    let config: Value = serde_json::from_str(&config_str).unwrap();

    // Check if puppeteer key was removed
    let puppeteer = &config["mcpServers"]["puppeteer"];
    assert!(
        puppeteer.is_null(),
        "Puppeteer config should be null after uninstall"
    );

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
    assert!(result["installed"].is_object());
    assert!(result["configured"].is_object());

    // Install and check status
    app::install("Browser".to_string()).await.unwrap();
    thread::sleep(Duration::from_millis(100));

    let result = app::get_app_statuses().unwrap();
    assert!(result["installed"]["Browser"].as_bool().unwrap());

    // Reset the test config path
    set_test_config_path(None);
}
