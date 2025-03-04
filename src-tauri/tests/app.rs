mod common;

use common::{setup_test_config, setup_test_environment};
use fleur_lib::app::{self, get_app_configs, set_test_config_path, APP_REGISTRY_CACHE};
use serde_json::{json, Value};
use serial_test::serial;
use std::{fs, thread, time::Duration};
use tempfile;

fn setup_mock_registry() -> Value {
    // Create the stubbed app registry for tests
    let stubbed_registry = json!([{
        "name": "Browser",
        "description": "This is a browser app that allows Claude to navigate to any website, take screenshots, and interact with the page.",
        "icon": {
          "type": "url",
          "url": {
            "light": "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/assets/browser.svg",
            "dark": "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/assets/browser.svg"
          }
        },
        "category": "Utilities",
        "price": "Free",
        "developer": "Google LLC",
        "sourceUrl": "https://github.com/modelcontextprotocol/servers/tree/main/src/puppeteer",
        "config": {
          "mcpKey": "puppeteer",
          "runtime": "npx",
          "args": [
            "-y",
            "@modelcontextprotocol/server-puppeteer",
            "--debug"
          ]
        },
        "features": [
          {
            "name": "Navigate to any website",
            "description": "Navigate to any URL in the browser",
            "prompt": "Navigate to the URL google.com and..."
          },
          {
            "name": "Interact with any website - search, click, scroll, screenshot, etc.",
            "description": "Click elements on the page",
            "prompt": "Go to google.com and search for..."
          }
        ],
        "setup": []
    },
    {
        "name": "Time",
        "description": "Get and convert time from different timezones",
        "icon": {
          "type": "url",
          "url": {
            "light": "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/assets/time.svg",
            "dark": "https://raw.githubusercontent.com/fleuristes/app-registry/refs/heads/main/assets/time.svg"
          }
        },
        "category": "Utilities",
        "price": "Free",
        "developer": "Anthropic",
        "sourceUrl": "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
        "config": {
          "mcpKey": "time",
          "runtime": "npx",
          "args": [
            "-y",
            "mcp-server-time",
            "--debug"
          ]
        },
        "features": [
          {
            "name": "Get current time in a timezone",
            "description": "Get the current time in a specific timezone",
            "prompt": "What time is it in Tokyo?"
          },
          {
            "name": "Convert time between timezones",
            "description": "Convert a time from one timezone to another",
            "prompt": "Convert 3PM PST to JST"
          }
        ],
        "setup": []
    }]);

    // Set the stubbed registry in the cache
    {
        let mut cache = APP_REGISTRY_CACHE.lock().unwrap();
        *cache = Some(stubbed_registry.clone());
    }

    stubbed_registry
}

fn teardown_mock_registry() {
    // Reset the cache after test
    let mut cache = APP_REGISTRY_CACHE.lock().unwrap();
    *cache = None;
}

#[test]
#[serial]
fn test_get_app_configs() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    let mock_registry = setup_mock_registry();

    // Test getting app configs
    let configs = get_app_configs().expect("Failed to get app configs");

    // Verify we got the expected number of apps
    assert_eq!(
        configs.len(),
        mock_registry.as_array().unwrap().len(),
        "Expected the same number of apps as in the mock registry"
    );

    // Check Browser app
    let browser = configs
        .iter()
        .find(|(name, _)| name == "Browser")
        .expect("Browser app not found");
    assert_eq!(browser.1.mcp_key, "puppeteer");
    assert!(
        browser.1.command.contains("npx"),
        "Expected command to contain 'npx'"
    );
    assert_eq!(
        browser.1.args.len(),
        3,
        "Expected 3 arguments for Browser app"
    );

    // Check Time app
    let time_app = configs
        .iter()
        .find(|(name, _)| name == "Time")
        .expect("Time app not found");
    assert_eq!(time_app.1.mcp_key, "time");
    assert_eq!(time_app.1.args[1], "mcp-server-time");

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_install() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Install the app
    let result = app::install("Browser", None);
    assert!(
        result.is_ok(),
        "Installation failed with error: {:?}",
        result.err()
    );

    // Wait a bit for file system operations to complete
    thread::sleep(Duration::from_millis(100));

    // Read the config file directly to verify it was updated correctly
    let config_str =
        fs::read_to_string(&config_path).expect("Failed to read config file after installation");
    let config: Value =
        serde_json::from_str(&config_str).expect("Failed to parse config JSON after installation");

    // Check if puppeteer key exists and has expected values
    let puppeteer = &config["mcpServers"]["puppeteer"];
    assert!(
        puppeteer.is_object(),
        "Puppeteer config should be an object"
    );

    // Verify command and args
    assert_eq!(
        puppeteer["command"].as_str().unwrap_or(""),
        config["mcpServers"]["puppeteer"]["command"]
            .as_str()
            .unwrap_or(""),
        "Command doesn't match expected value"
    );

    let args = puppeteer["args"]
        .as_array()
        .expect("Args should be an array");
    assert_eq!(args.len(), 3, "Expected 3 arguments");
    assert_eq!(args[0].as_str().unwrap_or(""), "-y");
    assert_eq!(
        args[1].as_str().unwrap_or(""),
        "@modelcontextprotocol/server-puppeteer"
    );
    assert_eq!(args[2].as_str().unwrap_or(""), "--debug");

    // Test installing with environment variables
    let env_vars = json!({
        "API_KEY": "test-key",
        "DEBUG": "true"
    });

    let result = app::install("Time", Some(env_vars.clone()));
    assert!(
        result.is_ok(),
        "Installation with env vars failed: {:?}",
        result.err()
    );

    thread::sleep(Duration::from_millis(100));

    // Read the config file again to verify env vars
    let config_str = fs::read_to_string(&config_path)
        .expect("Failed to read config file after second installation");
    let config: Value = serde_json::from_str(&config_str)
        .expect("Failed to parse config JSON after second installation");

    // Check if time app has environment variables
    let time_config = &config["mcpServers"]["time"];
    assert!(time_config.is_object(), "Time config should be an object");
    assert_eq!(
        time_config["env"]["API_KEY"].as_str().unwrap_or(""),
        "test-key"
    );
    assert_eq!(time_config["env"]["DEBUG"].as_str().unwrap_or(""), "true");

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_uninstall() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Create initial config with puppeteer already installed
    let initial_config = json!({
        "mcpServers": {
            "puppeteer": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-puppeteer", "--debug"]
            },
            "time": {
                "command": "npx",
                "args": ["-y", "mcp-server-time", "--debug"]
            }
        }
    });

    fs::write(
        &config_path,
        serde_json::to_string_pretty(&initial_config).unwrap(),
    )
    .expect("Failed to write initial config");

    // Test uninstalling the Browser app
    let result = app::uninstall("Browser");
    assert!(
        result.is_ok(),
        "Failed to uninstall Browser app: {:?}",
        result.err()
    );

    thread::sleep(Duration::from_millis(100));

    // Verify config was updated correctly
    let config_str =
        fs::read_to_string(&config_path).expect("Failed to read config after uninstall");
    let config: Value =
        serde_json::from_str(&config_str).expect("Failed to parse config after uninstall");

    // Check if puppeteer key was removed
    assert!(
        !config["mcpServers"]
            .as_object()
            .unwrap()
            .contains_key("puppeteer"),
        "Puppeteer config should be removed after uninstall"
    );

    // Check that time app is still there
    assert!(
        config["mcpServers"]
            .as_object()
            .unwrap()
            .contains_key("time"),
        "Time app should still be present"
    );

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_is_installed() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Check if app is installed before installation
    let is_installed_before = app::is_installed("Browser").expect("Failed to check installation");
    assert!(
        !is_installed_before,
        "App should not be installed initially"
    );

    // Install the app
    app::install("Browser", None).expect("Failed to install Browser app");

    thread::sleep(Duration::from_millis(100));

    // Check if app is installed after installation
    let is_installed_after = app::is_installed("Browser").expect("Failed to check installation");
    assert!(
        is_installed_after,
        "App should be installed after installation"
    );

    // Uninstall the app
    app::uninstall("Browser").expect("Failed to uninstall Browser app");

    thread::sleep(Duration::from_millis(100));

    // Check if app is uninstalled
    let is_installed_final = app::is_installed("Browser").expect("Failed to check installation");
    assert!(
        !is_installed_final,
        "App should not be installed after uninstallation"
    );

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_app_env() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Install app first
    app::install("Browser", None).expect("Failed to install Browser app");

    // Set environment variables
    let env_values = json!({
        "API_KEY": "test-key",
        "DEBUG": "true"
    });

    let result = app::save_app_env("Browser", env_values.clone());
    assert!(result.is_ok(), "Failed to save app env: {:?}", result.err());

    thread::sleep(Duration::from_millis(100));

    // Get and verify environment variables
    let app_env = app::get_app_env("Browser").expect("Failed to get app env");
    assert_eq!(app_env["API_KEY"].as_str().unwrap_or(""), "test-key");
    assert_eq!(app_env["DEBUG"].as_str().unwrap_or(""), "true");

    // Update environment variables
    let updated_env = json!({
        "API_KEY": "new-key",
        "LOG_LEVEL": "debug"
    });

    let result = app::save_app_env("Browser", updated_env.clone());
    assert!(
        result.is_ok(),
        "Failed to update app env: {:?}",
        result.err()
    );

    thread::sleep(Duration::from_millis(100));

    // Get and verify updated environment variables
    let updated_app_env = app::get_app_env("Browser").expect("Failed to get updated app env");
    assert_eq!(updated_app_env["API_KEY"].as_str().unwrap_or(""), "new-key");
    assert_eq!(updated_app_env["DEBUG"].as_str().unwrap_or(""), "true");
    assert_eq!(updated_app_env["LOG_LEVEL"].as_str().unwrap_or(""), "debug");

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_app_statuses() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Test initial statuses (no apps installed)
    let statuses = app::get_app_statuses().expect("Failed to get initial app statuses");

    // Verify initial statuses
    assert!(
        !statuses["installed"]["Browser"].as_bool().unwrap_or(true),
        "Browser should not be installed initially"
    );
    assert!(
        !statuses["installed"]["Time"].as_bool().unwrap_or(true),
        "Time should not be installed initially"
    );

    assert!(
        statuses["configured"]["Browser"].as_bool().unwrap_or(false),
        "Browser should be configured"
    );
    assert!(
        statuses["configured"]["Time"].as_bool().unwrap_or(false),
        "Time should be configured"
    );

    // Install an app and check status
    app::install("Browser", None).expect("Failed to install Browser app");

    thread::sleep(Duration::from_millis(100));

    let statuses_after =
        app::get_app_statuses().expect("Failed to get app statuses after installation");

    // Verify statuses after installation
    assert!(
        statuses_after["installed"]["Browser"]
            .as_bool()
            .unwrap_or(false),
        "Browser should be installed"
    );
    assert!(
        !statuses_after["installed"]["Time"]
            .as_bool()
            .unwrap_or(true),
        "Time should not be installed"
    );

    teardown_mock_registry();
    set_test_config_path(None);
}

#[test]
#[serial]
fn test_get_app_registry() {
    let _temp_dir = setup_test_environment();
    let (config_path, _temp_dir2) = setup_test_config();
    set_test_config_path(Some(config_path.clone()));

    setup_mock_registry();

    // Get the app registry
    let registry = app::get_app_registry().expect("Failed to get app registry");

    // Verify registry contents
    assert!(registry.is_array(), "Registry should be an array");
    assert_eq!(
        registry.as_array().unwrap().len(),
        2,
        "Registry should contain 2 apps"
    );

    // Verify Browser app
    let browser = registry
        .as_array()
        .unwrap()
        .iter()
        .find(|app| app["name"].as_str().unwrap_or("") == "Browser")
        .expect("Browser app not found in registry");

    assert_eq!(
        browser["config"]["mcpKey"].as_str().unwrap_or(""),
        "puppeteer"
    );
    assert_eq!(browser["category"].as_str().unwrap_or(""), "Utilities");

    // Verify Time app
    let time = registry
        .as_array()
        .unwrap()
        .iter()
        .find(|app| app["name"].as_str().unwrap_or("") == "Time")
        .expect("Time app not found in registry");

    assert_eq!(time["config"]["mcpKey"].as_str().unwrap_or(""), "time");
    assert_eq!(time["developer"].as_str().unwrap_or(""), "Anthropic");

    teardown_mock_registry();
    set_test_config_path(None);
}
