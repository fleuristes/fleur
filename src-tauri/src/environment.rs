use log::info;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

struct EnvironmentState {
    uv_installed: AtomicBool,
    nvm_installed: AtomicBool,
    node_installed: AtomicBool,
    setup_started: AtomicBool,
}

impl EnvironmentState {
    const fn new() -> Self {
        Self {
            uv_installed: AtomicBool::new(false),
            nvm_installed: AtomicBool::new(false),
            node_installed: AtomicBool::new(false),
            setup_started: AtomicBool::new(false),
        }
    }
}

static ENV_STATE: EnvironmentState = EnvironmentState::new();
static NODE_VERSION: &str = "v20.9.0";

static mut IS_TEST_MODE: bool = false;

#[cfg(feature = "test-utils")]
pub fn set_test_mode(enabled: bool) {
    unsafe {
        IS_TEST_MODE = enabled;
    }
}

fn is_test_mode() -> bool {
    unsafe { IS_TEST_MODE }
}

#[cfg(feature = "test-utils")]
pub fn reset_environment_state_for_tests() {
    ENV_STATE.setup_started.store(false, Ordering::SeqCst);
    ENV_STATE.uv_installed.store(false, Ordering::Relaxed);
    ENV_STATE.nvm_installed.store(false, Ordering::Relaxed);
    ENV_STATE.node_installed.store(false, Ordering::Relaxed);
}

pub fn get_npx_shim_path() -> PathBuf {
    if is_test_mode() {
        return PathBuf::from("/test/.local/share/fleur/bin/npx-fleur");
    }

    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/fleur/bin/npx-fleur")
}

pub fn get_uvx_path() -> Result<String, String> {
    if is_test_mode() {
        return Ok("/test/uvx".to_string());
    }

    let output = Command::new("which")
        .arg("uvx")
        .output()
        .map_err(|e| format!("Failed to get uvx path: {}", e))?;

    if !output.status.success() {
        return Err("uvx not found in PATH".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_nvm_node_paths() -> Result<(String, String), String> {
    if is_test_mode() {
        return Ok(("/test/node".to_string(), "/test/npx".to_string()));
    }

    let shell_command = format!(
        r#"
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        nvm use {} > /dev/null 2>&1
        which node
        which npx
    "#,
        NODE_VERSION
    );

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_err(|e| format!("Failed to get node paths: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get node and npx paths".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut lines = output_str.lines();

    let node_path = lines
        .next()
        .ok_or("Failed to get node path")?
        .trim()
        .to_string();

    let npx_path = lines
        .next()
        .ok_or("Failed to get npx path")?
        .trim()
        .to_string();

    if !node_path.contains(".nvm/versions/node") {
        return Err("Node path is not from nvm installation".to_string());
    }

    Ok((node_path, npx_path))
}

fn check_uv_installed() -> bool {
    if is_test_mode() {
        return true;
    }

    if ENV_STATE.uv_installed.load(Ordering::Relaxed) {
        return true;
    }

    let which_command = Command::new("which")
        .arg("uv")
        .output()
        .map_or(false, |output| output.status.success());

    if !which_command {
        return false;
    }

    let version_command = Command::new("uv")
        .arg("--version")
        .output()
        .map_or(false, |output| output.status.success());

    if version_command {
        ENV_STATE.uv_installed.store(true, Ordering::Relaxed);
        info!("uv is already installed");
    }

    version_command
}

fn install_uv() -> Result<(), String> {
    if check_uv_installed() {
        return Ok(());
    }

    info!("Installing uv...");

    let shell_command = r#"
        curl -LsSf https://astral.sh/uv/install.sh | sh
    "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_err(|e| format!("Failed to install uv: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "uv installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    ENV_STATE.uv_installed.store(true, Ordering::Relaxed);
    info!("uv installed successfully");
    Ok(())
}

pub fn ensure_uv_environment() -> Result<String, String> {
    if !check_uv_installed() {
        install_uv()?;
    }

    Ok("UV environment is ready".to_string())
}

fn check_nvm_installed() -> bool {
    if is_test_mode() {
        return true;
    }

    if ENV_STATE.nvm_installed.load(Ordering::Relaxed) {
        return true;
    }

    let nvm_dir = dirs::home_dir()
        .map(|path| path.join(".nvm"))
        .filter(|path| path.exists());

    if nvm_dir.is_none() {
        return false;
    }

    let shell_command = r#"
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        nvm --version
    "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_or(false, |output| output.status.success());

    if output {
        ENV_STATE.nvm_installed.store(true, Ordering::Relaxed);
        info!("nvm is already installed");
    }

    output
}

fn install_nvm() -> Result<(), String> {
    info!("Installing nvm...");

    let shell_command = r#"
        curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
    "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_err(|e| format!("Failed to install nvm: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "nvm installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    ENV_STATE.nvm_installed.store(true, Ordering::Relaxed);
    info!("nvm installed successfully");
    Ok(())
}

fn check_node_version() -> Result<String, String> {
    if is_test_mode() {
        return Ok(NODE_VERSION.to_string());
    }

    if ENV_STATE.node_installed.load(Ordering::Relaxed) {
        return Ok(NODE_VERSION.to_string());
    }

    let which_command = Command::new("which")
        .arg("node")
        .output()
        .map_err(|e| format!("Failed to check node existence: {}", e))?;

    if !which_command.status.success() {
        return Err("Node not found in PATH".to_string());
    }

    let version_command = Command::new("node")
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to check node version: {}", e))?;

    if version_command.status.success() {
        let version = String::from_utf8_lossy(&version_command.stdout)
            .trim()
            .to_string();

        if version == NODE_VERSION {
            ENV_STATE.node_installed.store(true, Ordering::Relaxed);
        }

        Ok(version)
    } else {
        Err("Failed to get Node version".to_string())
    }
}

fn install_node() -> Result<(), String> {
    info!("Installing Node.js {}...", NODE_VERSION);

    // First ensure nvm is sourced
    let nvm_source = format!(
        r#"
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        which nvm
    "#
    );

    let nvm_path_output = Command::new("bash")
        .arg("-c")
        .arg(&nvm_source)
        .output()
        .map_err(|e| format!("Failed to source nvm: {}", e))?;

    if !nvm_path_output.status.success() {
        return Err("Failed to source nvm".to_string());
    }

    let nvm_path = String::from_utf8_lossy(&nvm_path_output.stdout)
        .trim()
        .to_string();

    if nvm_path.is_empty() {
        return Err("nvm not found after sourcing".to_string());
    }

    let output = Command::new("bash")
        .arg("-c")
        .arg(format!("{} install {}", nvm_source, NODE_VERSION))
        .output()
        .map_err(|e| format!("Failed to run node installation: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Node installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    ENV_STATE.node_installed.store(true, Ordering::Relaxed);
    info!("Node.js {} installed successfully", NODE_VERSION);
    Ok(())
}

pub fn ensure_npx_shim() -> Result<String, String> {
    if is_test_mode() {
        return Ok("/test/.local/share/fleur/bin/npx-fleur".to_string());
    }

    let shim_path = get_npx_shim_path();

    if shim_path.exists() {
        return Ok(shim_path.to_string_lossy().to_string());
    }

    let (node_path, npx_path) = get_nvm_node_paths()?;

    if let Some(parent) = shim_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create shim directory: {}", e))?;
    }

    let shim_content = format!(
        r#"#!/bin/sh
# NPX shim for Fleur

NODE="{}"
NPX="{}"

export PATH="$(dirname "$NODE"):$PATH"

exec "$NPX" "$@"
"#,
        node_path, npx_path
    );

    std::fs::write(&shim_path, shim_content)
        .map_err(|e| format!("Failed to write shim script: {}", e))?;

    Command::new("chmod")
        .arg("+x")
        .arg(&shim_path)
        .output()
        .map_err(|e| format!("Failed to make shim executable: {}", e))?;

    Ok(shim_path.to_string_lossy().to_string())
}

pub fn ensure_node_environment() -> Result<String, String> {
    if !check_nvm_installed() {
        install_nvm()?;
    }

    match check_node_version() {
        Ok(version) => {
            if version != NODE_VERSION {
                install_node()?;
            }
        }
        Err(_) => {
            install_node()?;
        }
    }

    ensure_npx_shim()?;

    Ok("Node environment is ready".to_string())
}

#[tauri::command]
pub fn ensure_environment() -> Result<String, String> {
    if ENV_STATE.setup_started.swap(true, Ordering::SeqCst) {
        return Ok("Environment setup already in progress".to_string());
    }

    // Reset state flags
    ENV_STATE.uv_installed.store(false, Ordering::Relaxed);
    ENV_STATE.nvm_installed.store(false, Ordering::Relaxed);
    ENV_STATE.node_installed.store(false, Ordering::Relaxed);

    std::thread::spawn(|| {
        if let Err(err) = ensure_uv_environment() {
            info!("UV setup error: {}", err);
        }

        if let Err(err) = ensure_node_environment() {
            info!("Node environment setup error: {}", err);
        }
    });

    Ok("Environment setup started".to_string())
}
