#[cfg(test)]
mod common;

use fleur_lib::environment;

#[tokio::test]
async fn test_environment_setup() {
    environment::set_test_mode(true);
    let result = environment::ensure_environment().await;
    assert!(result.is_ok());
    environment::set_test_mode(false);
}

#[test]
fn test_node_environment() {
    environment::set_test_mode(true);
    let result = environment::ensure_npx_shim();
    assert!(result.is_ok());
    assert!(result.unwrap().contains("npx-fleur"));
    environment::set_test_mode(false);
}
