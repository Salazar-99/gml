//! Shared SSH public key resolution for `gml connect` and providers (e.g. Google TPU metadata).

use crate::error::GmlError;
use std::fs;
use std::path::{Path, PathBuf};

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Resolves the path to an SSH **public** key file (`.pub`).
///
/// Resolution order:
/// 1. `config_ssh_public_key` if set (from `[gml] ssh-public-key` in `~/.gml/config.toml`)
/// 2. First existing file among `~/.ssh/id_ed25519.pub`, `id_rsa.pub`, `id_ecdsa.pub`
pub fn get_ssh_public_key(config_ssh_public_key: Option<&str>) -> Result<PathBuf, GmlError> {
    if let Some(p) = config_ssh_public_key.filter(|s| !s.trim().is_empty()) {
        let path = expand_user_path(p.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(GmlError::from(format!(
            "[gml] ssh-public-key file not found: {}",
            path.display()
        )));
    }
    let home = dirs::home_dir().ok_or_else(|| {
        GmlError::from(
            "SSH public key: set [gml] ssh-public-key in ~/.gml/config.toml, or ensure HOME is set to search ~/.ssh/",
        )
    })?;
    for name in ["id_ed25519.pub", "id_rsa.pub", "id_ecdsa.pub"] {
        let path = home.join(".ssh").join(name);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(GmlError::from(
        "No SSH public key found. Set [gml] ssh-public-key in ~/.gml/config.toml, or create ~/.ssh/id_ed25519.pub",
    ))
}

/// Reads and validates the first non-empty line of an OpenSSH public key file.
pub fn read_ssh_public_key_line(path: &Path) -> Result<String, GmlError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        GmlError::from(format!("Failed to read SSH public key {}: {}", path.display(), e))
    })?;
    let line = contents.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    if line.is_empty() {
        return Err(GmlError::from(format!("Empty SSH public key file: {}", path.display())));
    }
    if !(line.starts_with("ssh-rsa")
        || line.starts_with("ssh-ed25519")
        || line.starts_with("ecdsa-")
        || line.starts_with("ssh-dss"))
    {
        return Err(GmlError::from(format!(
            "File {} does not look like an SSH public key (expected ssh-rsa, ssh-ed25519, ...)",
            path.display()
        )));
    }
    Ok(line.to_string())
}
