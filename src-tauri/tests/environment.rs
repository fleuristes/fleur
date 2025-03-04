#[cfg(test)]
mod common;

use fleur_lib::environment;
use std::thread;
use std::time::Duration;

// Environment module tests
#[test]
fn test_environment_setup() {
    // Reset the static variable state before the test
    environment::reset_environment_state_for_tests();
    environment::set_test_mode(true);

    let result = environment::ensure_environment();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Environment setup started");

    // Give the async thread some time to complete
    thread::sleep(Duration::from_millis(100));

    environment::set_test_mode(false);
}

#[test]
fn test_ensure_uv_environment() {
    environment::set_test_mode(true);
    let result = environment::ensure_uv_environment();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "UV environment is ready");
    environment::set_test_mode(false);
}

#[test]
fn test_ensure_node_environment() {
    environment::set_test_mode(true);
    let result = environment::ensure_node_environment();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Node environment is ready");
    environment::set_test_mode(false);
}

#[test]
fn test_npx_shim_path() {
    environment::set_test_mode(true);
    let path = environment::get_npx_shim_path();
    assert_eq!(
        path.to_str().unwrap(),
        "/test/.local/share/fleur/bin/npx-fleur"
    );
    environment::set_test_mode(false);
}

#[test]
fn test_npx_shim_creation() {
    environment::set_test_mode(true);
    let result = environment::ensure_npx_shim();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "/test/.local/share/fleur/bin/npx-fleur");
    environment::set_test_mode(false);
}

#[test]
fn test_uvx_path() {
    environment::set_test_mode(true);
    let result = environment::get_uvx_path();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "/test/uvx");
    environment::set_test_mode(false);
}

#[test]
fn test_nvm_node_paths() {
    environment::set_test_mode(true);
    let result = environment::get_nvm_node_paths();
    assert!(result.is_ok());
    let (node_path, npx_path) = result.unwrap();
    assert_eq!(node_path, "/test/node");
    assert_eq!(npx_path, "/test/npx");
    environment::set_test_mode(false);
}
