mod common;

use common::setup_test_config;
use fleur_lib::app;
use fleur_lib::environment;

#[test]
fn test_full_app_lifecycle() {
    // Set up test mode first
    environment::set_test_mode(true);

    let (_config_path, temp_dir) = setup_test_config();

    // Mock home directory
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", temp_dir.path());

    // Test installation
    let install_result = app::install("Browser", None);
    if let Err(e) = &install_result {
        println!("Installation failed with error: {}", e);
    }
    assert!(install_result.is_ok());
    assert!(app::is_installed("Browser").unwrap());

    // Test uninstallation
    let uninstall_result = app::uninstall("Browser");
    if let Err(e) = &uninstall_result {
        println!("Uninstallation failed with error: {}", e);
    }
    assert!(uninstall_result.is_ok());
    assert!(!app::is_installed("Browser").unwrap());

    // Cleanup
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    }
}
