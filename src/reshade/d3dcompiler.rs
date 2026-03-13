//! Embedded `d3dcompiler_47.dll` assets and installation helpers.
//!
//! The DLLs are compiled into the binary via [`rust_embed`] and written to the
//! application data directory on demand. This avoids any runtime download or
//! external-tool dependency.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rust_embed::RustEmbed;

use crate::reshade::game::ExeArch;

#[derive(RustEmbed)]
#[folder = "data/d3dcompiler"]
struct Assets;

/// Returns the path where the DLL for `arch` should live in `data_dir`.
///
/// The filename follows the convention used by the rest of the codebase:
/// `d3dcompiler_47.dll.32` or `d3dcompiler_47.dll.64`.
#[must_use]
pub fn dll_path(data_dir: &Path, arch: ExeArch) -> PathBuf {
    data_dir.join(format!(
        "d3dcompiler_47.dll.{}",
        arch.d3dcompiler_suffix()
    ))
}

/// Returns `true` if the DLL for `arch` already exists in `data_dir`.
#[must_use]
pub fn is_installed(data_dir: &Path, arch: ExeArch) -> bool {
    dll_path(data_dir, arch).is_file()
}

/// Ensures `d3dcompiler_47.dll.<arch>` exists in `data_dir`.
///
/// If the file is already present this is a no-op and returns `Ok(false)`.
/// Otherwise it writes the embedded asset to disk and returns `Ok(true)`.
///
/// # Errors
/// Returns an error if `data_dir` cannot be created or the file cannot be
/// written.
pub fn ensure(data_dir: &Path, arch: ExeArch) -> Result<bool> {
    if is_installed(data_dir, arch) {
        return Ok(false);
    }
    let asset_name = format!("d3dcompiler_47.dll.{}", arch.d3dcompiler_suffix());
    let asset = Assets::get(&asset_name)
        .ok_or_else(|| anyhow::anyhow!("Embedded asset '{asset_name}' not found"))?;
    std::fs::create_dir_all(data_dir).context("Cannot create data directory")?;
    let dest = dll_path(data_dir, arch);
    std::fs::write(&dest, asset.data)
        .with_context(|| format!("Cannot write {}", dest.display()))?;
    log::info!("Installed {} to {}", asset_name, dest.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dll_path_32bit() {
        let dir = tempdir().unwrap();
        let path = dll_path(dir.path(), ExeArch::X86);
        assert!(path.to_string_lossy().ends_with("d3dcompiler_47.dll.32"));
    }

    #[test]
    fn dll_path_64bit() {
        let dir = tempdir().unwrap();
        let path = dll_path(dir.path(), ExeArch::X86_64);
        assert!(path.to_string_lossy().ends_with("d3dcompiler_47.dll.64"));
    }

    #[test]
    fn is_installed_false_when_absent() {
        let dir = tempdir().unwrap();
        assert!(!is_installed(dir.path(), ExeArch::X86_64));
    }

    #[test]
    fn is_installed_true_when_present() {
        let dir = tempdir().unwrap();
        let path = dll_path(dir.path(), ExeArch::X86_64);
        std::fs::write(&path, b"fake dll").unwrap();
        assert!(is_installed(dir.path(), ExeArch::X86_64));
    }

    #[test]
    fn ensure_writes_file_and_returns_true() {
        let dir = tempdir().unwrap();
        let result = ensure(dir.path(), ExeArch::X86_64).unwrap();
        assert!(result, "should return true on first install");
        assert!(dll_path(dir.path(), ExeArch::X86_64).is_file());
    }

    #[test]
    fn ensure_is_idempotent() {
        let dir = tempdir().unwrap();
        ensure(dir.path(), ExeArch::X86_64).unwrap();
        let result = ensure(dir.path(), ExeArch::X86_64).unwrap();
        assert!(!result, "should return false when already installed");
    }

    #[test]
    fn ensure_both_archs() {
        let dir = tempdir().unwrap();
        ensure(dir.path(), ExeArch::X86).unwrap();
        ensure(dir.path(), ExeArch::X86_64).unwrap();
        assert!(dll_path(dir.path(), ExeArch::X86).is_file());
        assert!(dll_path(dir.path(), ExeArch::X86_64).is_file());
    }
}
