use log::{debug, error, info};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

static UV_INSTALLED: AtomicBool = AtomicBool::new(false);
static NVM_INSTALLED: AtomicBool = AtomicBool::new(false);
static NODE_INSTALLED: AtomicBool = AtomicBool::new(false);
static ENVIRONMENT_SETUP_STARTED: AtomicBool = AtomicBool::new(false);
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

pub fn get_npx_shim_path() -> std::path::PathBuf {
    if is_test_mode() {
        return std::path::PathBuf::from("/test/.local/share/fleur/bin/npx-fleur");
    }
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/fleur/bin/npx-fleur")
}

pub fn get_uvx_path() -> Result<String, String> {
    let output = Command::new("which")
        .arg("uvx")
        .output()
        .map_err(|e| format!("Failed to get uvx path: {}", e))?;

    if !output.status.success() {
        return Err("uvx not found in PATH".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_nvm_dir() -> PathBuf {
    if cfg!(test) {
        debug!("Using test mock path");
        PathBuf::from("/tmp/nvm")
    } else {
        debug!("Using real implementation path");
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".nvm")
    }
}

pub fn get_nvm_node_paths() -> Result<(String, String), String> {
    if is_test_mode() {
        debug!("Using test mock path");
        return Ok(("/test/node".to_string(), "/test/npx".to_string()));
    }

    debug!("Using real implementation path");
    let shell_command = r#"
        export NVM_DIR="$HOME/.nvm"
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
        nvm use v20.9.0 > /dev/null 2>&1
        which node
        which npx
    "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_err(|e| {
            error!("Failed to get node paths: {}", e);
            format!("Failed to get node paths: {}", e)
        })?;

    if !output.status.success() {
        error!("Failed to get node and npx paths");
        return Err("Failed to get node and npx paths".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut lines = output_str.lines();

    let node_path = lines
        .next()
        .ok_or_else(|| {
            error!("Failed to get node path");
            "Failed to get node path".to_string()
        })?
        .trim()
        .to_string();

    let npx_path = lines
        .next()
        .ok_or_else(|| {
            error!("Failed to get npx path");
            "Failed to get npx path".to_string()
        })?
        .trim()
        .to_string();

    if !node_path.contains(".nvm/versions/node") {
        error!("Node path is not from nvm installation");
        return Err("Node path is not from nvm installation".to_string());
    }

    debug!("Found node path: {}", node_path);
    debug!("Found npx path: {}", npx_path);
    Ok((node_path, npx_path))
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

fn check_node_version() -> Result<String, String> {
    if is_test_mode() {
        return Ok("v20.9.0".to_string());
    }

    if NODE_INSTALLED.load(Ordering::Relaxed) {
        return Ok("v20.9.0".to_string());
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

        if version == "v20.9.0" {
            NODE_INSTALLED.store(true, Ordering::Relaxed);
        }

        Ok(version)
    } else {
        Err("Failed to get Node version".to_string())
    }
}

pub async fn ensure_node() -> Result<(), String> {
    info!("Installing Node.js v20.9.0...");
    if !check_nvm_installed() {
        install_nvm()?;
    }

    match check_node_version() {
        Ok(version) => {
            if version != "v20.9.0" {
                install_node()?;
            }
            ensure_npx_shim()?;
            info!("Node.js v20.9.0 installed successfully");
            Ok(())
        }
        Err(_) => {
            install_node()?;
            ensure_npx_shim()?;
            info!("Node.js v20.9.0 installed successfully");
            Ok(())
        }
    }
}

fn check_nvm_installed() -> bool {
    if is_test_mode() {
        return true;
    }

    if NVM_INSTALLED.load(Ordering::Relaxed) {
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
        NVM_INSTALLED.store(true, Ordering::Relaxed);
        info!("nvm is already installed");
    }

    output
}

pub async fn ensure_nvm() -> Result<(), String> {
    if Path::new(&get_nvm_dir().join("nvm.sh")).exists() {
        info!("nvm is already installed");
        return Ok(());
    }

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

    NVM_INSTALLED.store(true, Ordering::Relaxed);
    info!("nvm installed successfully");
    Ok(())
}

fn check_uv_installed() -> bool {
    if is_test_mode() {
        return true;
    }

    if UV_INSTALLED.load(Ordering::Relaxed) {
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
        UV_INSTALLED.store(true, Ordering::Relaxed);
        info!("uv is installed");
    }

    version_command
}

pub async fn ensure_uv() -> Result<(), String> {
    if Command::new("uv").arg("--version").output().is_ok() {
        info!("uv is installed");
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

    UV_INSTALLED.store(true, Ordering::Relaxed);
    info!("uv installed successfully");
    Ok(())
}

#[tauri::command]
pub async fn ensure_environment() -> Result<String, String> {
    if ENVIRONMENT_SETUP_STARTED.swap(true, Ordering::SeqCst) {
        return Ok("Environment setup already in progress".to_string());
    }

    std::thread::spawn(|| {
        if !check_uv_installed() {
            let _ = install_uv();
        }
        let _ = ensure_node();
    });

    Ok("Environment setup started".to_string())
}

fn install_node() -> Result<(), String> {
    info!("Installing Node.js v20.9.0...");

    let nvm_path_output = Command::new("which").arg("nvm").output().map_err(|e| {
        error!("Failed to get nvm path: {}", e);
        format!("Failed to get nvm path: {}", e)
    })?;

    if !nvm_path_output.status.success() {
        error!("nvm not found in PATH");
        return Err("nvm not found in PATH".to_string());
    }

    let nvm_path = String::from_utf8_lossy(&nvm_path_output.stdout)
        .trim()
        .to_string();

    let output = Command::new(nvm_path)
        .arg("install")
        .arg("v20.9.0")
        .output()
        .map_err(|e| {
            error!("Failed to run node installation: {}", e);
            format!("Failed to run node installation: {}", e)
        })?;

    if !output.status.success() {
        let err_msg = format!(
            "Node installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        error!("{}", err_msg);
        return Err(err_msg);
    }

    NODE_INSTALLED.store(true, Ordering::Relaxed);
    info!("Node.js v20.9.0 installed successfully");
    Ok(())
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
        .map_err(|e| {
            error!("Failed to run nvm installation: {}", e);
            format!("Failed to run nvm installation: {}", e)
        })?;

    if !output.status.success() {
        let err_msg = format!(
            "nvm installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        error!("{}", err_msg);
        return Err(err_msg);
    }

    NVM_INSTALLED.store(true, Ordering::Relaxed);
    info!("nvm installed successfully");
    Ok(())
}

fn install_uv() -> Result<(), String> {
    info!("Installing uv...");

    let shell_command = r#"
        curl -LsSf https://astral.sh/uv/install.sh | sh
    "#;

    let output = Command::new("bash")
        .arg("-c")
        .arg(shell_command)
        .output()
        .map_err(|e| {
            error!("Failed to run uv installation: {}", e);
            format!("Failed to run uv installation: {}", e)
        })?;

    if !output.status.success() {
        let err_msg = format!(
            "uv installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        error!("{}", err_msg);
        return Err(err_msg);
    }

    UV_INSTALLED.store(true, Ordering::Relaxed);
    info!("uv installed successfully");
    Ok(())
}
