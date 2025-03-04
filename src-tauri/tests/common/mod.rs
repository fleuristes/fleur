use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

#[allow(dead_code)]
pub fn setup_test_config() -> (PathBuf, TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_dir = temp_dir.path().join("Library/Application Support/Claude");
    std::fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("claude_desktop_config.json");
    let initial_config = json!({
        "mcpServers": {}
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&initial_config).unwrap(),
    )
    .unwrap();

    (config_path, temp_dir)
}

pub fn setup_test_environment() -> TempDir {
    // Set up test mode
    fleur_lib::environment::set_test_mode(true);

    // Create a temporary directory for the test environment
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    // Mock environment variables
    std::env::set_var("HOME", temp_path);
    std::env::set_var("PATH", format!("{}/bin", temp_path));

    // Create necessary directories
    std::fs::create_dir_all(format!("{}/Library/Application Support/Claude", temp_path)).unwrap();
    std::fs::create_dir_all(format!("{}/bin", temp_path)).unwrap();

    temp_dir
}
