use std::path::PathBuf;

use directories::ProjectDirs;

use crate::error::{Error, Result};

/// The OS-appropriate directory for Tunnelium's config and data.
///
/// - Linux:   `~/.config/Tunnelium`
/// - macOS:   `~/Library/Application Support/io.Tunnelium.Tunnelium`
/// - Windows: `%APPDATA%\Tunnelium\Tunnelium\config`
pub fn app_config_dir() -> Result<PathBuf> {
    ProjectDirs::from("io", "Tunnelium", "Tunnelium")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or(Error::NoConfigDir)
}
