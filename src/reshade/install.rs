//! Install and uninstall `ReShade` into a game directory via symlinks.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::reshade::game::{DllOverride, ExeArch, InstallStatus};

/// Installs `ReShade` into `game_dir` by creating symlinks.
///
/// Links:
/// - `ReShade{32,64}.dll` → `<dll>.dll`
/// - `d3dcompiler_47.dll.<arch>` → `d3dcompiler_47.dll`
/// - `ReShade_shaders/Merged` → `reshade-shaders` (only if Merged exists)
///
/// # Errors
/// Returns an error if any symlink creation fails.
#[allow(clippy::module_name_repetitions)]
pub fn install_reshade(base: &Path, game_dir: &Path, version: &str, dll: DllOverride, arch: ExeArch) -> Result<()> {
    // ReShade DLL → <dll>.dll
    let reshade_src = base.join("reshade").join(version).join(arch.reshade_dll());
    let dll_dest = game_dir.join(dll.symlink_name());
    symlink_force(&reshade_src, &dll_dest)?;

    // d3dcompiler
    let d3dc_src = base.join(format!("d3dcompiler_47.dll.{}", arch.d3dcompiler_suffix()));
    let d3dc_dest = game_dir.join("d3dcompiler_47.dll");
    symlink_force(&d3dc_src, &d3dc_dest)?;

    // Shaders dir (optional — skip if not yet populated)
    let shaders_src = base.join("ReShade_shaders/Merged");
    let shaders_dest = game_dir.join("reshade-shaders");
    if shaders_src.exists() && !shaders_dest.exists() && !shaders_dest.is_symlink() {
        symlink_force(&shaders_src, &shaders_dest)?;
    }

    Ok(())
}

/// Removes all `ReShade` symlinks from `game_dir`.
///
/// # Errors
/// Returns an error if any symlink removal fails.
pub fn uninstall_reshade(game_dir: &Path, dll: DllOverride) -> Result<()> {
    let files = [
        dll.symlink_name().to_owned(),
        "d3dcompiler_47.dll".into(),
        "reshade-shaders".into(),
        "ReShade.ini".into(),
        "ReShade32.json".into(),
        "ReShade64.json".into(),
    ];
    for name in &files {
        let path = game_dir.join(name);
        if path.is_symlink() {
            std::fs::remove_file(&path).with_context(|| format!("Cannot remove {}", path.display()))?;
        }
    }
    Ok(())
}

/// Creates a symlink at `dest` pointing to `src`, removing any existing entry first.
fn symlink_force(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() || dest.is_symlink() {
        std::fs::remove_file(dest)?;
    }
    std::os::unix::fs::symlink(src, dest)
        .with_context(|| format!("Cannot create symlink {} -> {}", dest.display(), src.display()))
}

/// Returns the default DLL override for a given architecture.
#[must_use]
pub const fn default_dll_for_arch(arch: ExeArch) -> DllOverride {
    match arch {
        ExeArch::X86 => DllOverride::D3d9,
        ExeArch::X86_64 => DllOverride::Dxgi,
    }
}

/// Detects the current `ReShade` install status by inspecting symlinks in `game_dir`.
///
/// Scans for any known DLL override symlink. When found, reads the symlink
/// target to determine architecture from the DLL name (`ReShade64` vs `ReShade32`)
/// and the version from the parent directory name of the target path.
#[must_use]
pub fn detect_install_status(game_dir: &Path) -> InstallStatus {
    for &dll in DllOverride::all() {
        let symlink = game_dir.join(dll.symlink_name());
        if symlink.is_symlink() {
            let target = std::fs::read_link(&symlink).ok();
            let arch = target
                .as_ref()
                .and_then(|p| {
                    let s = p.to_string_lossy().into_owned();
                    if s.contains("ReShade64") {
                        Some(ExeArch::X86_64)
                    } else if s.contains("ReShade32") {
                        Some(ExeArch::X86)
                    } else {
                        None
                    }
                })
                .unwrap_or(ExeArch::X86_64);
            // The target path is <base>/reshade/<version>/ReShade{32,64}.dll,
            // so the version is the name of the parent directory.
            let version = target
                .and_then(|p| p.parent()?.file_name()?.to_str().map(String::from));
            return InstallStatus::Installed { dll, arch, version };
        }
    }
    InstallStatus::NotInstalled
}

/// Returns all `.exe` files in `game_dir`.
#[must_use]
pub fn find_exes(game_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(game_dir) else {
        return vec![];
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "exe"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_fake_reshade(base: &Path, version: &str, arch: ExeArch) {
        let dll_name = arch.reshade_dll();
        let versioned = base.join("reshade").join(version);
        std::fs::create_dir_all(&versioned).unwrap();
        std::fs::write(versioned.join(dll_name), "fake dll").unwrap();
        let suffix = arch.d3dcompiler_suffix();
        std::fs::write(base.join(format!("d3dcompiler_47.dll.{suffix}")), "fake d3dc").unwrap();
    }

    #[test]
    fn install_creates_symlinks() {
        let base = tempdir().unwrap();
        let game_dir = tempdir().unwrap();
        let version = "6.1.0";
        let arch = ExeArch::X86_64;
        let dll = DllOverride::Dxgi;
        setup_fake_reshade(base.path(), version, arch);

        install_reshade(base.path(), game_dir.path(), version, dll, arch).unwrap();

        assert!(game_dir.path().join("dxgi.dll").is_symlink(), "dxgi.dll symlink missing");
        assert!(game_dir.path().join("d3dcompiler_47.dll").is_symlink(), "d3dcompiler_47.dll symlink missing");
    }

    #[test]
    fn uninstall_removes_symlinks() {
        let base = tempdir().unwrap();
        let game_dir = tempdir().unwrap();
        let arch = ExeArch::X86_64;
        let dll = DllOverride::Dxgi;
        setup_fake_reshade(base.path(), "6.1.0", arch);

        install_reshade(base.path(), game_dir.path(), "6.1.0", dll, arch).unwrap();
        uninstall_reshade(game_dir.path(), dll).unwrap();

        assert!(!game_dir.path().join("dxgi.dll").exists());
        assert!(!game_dir.path().join("d3dcompiler_47.dll").exists());
    }

    #[test]
    fn default_dll_for_arch_x86() {
        assert_eq!(default_dll_for_arch(ExeArch::X86), DllOverride::D3d9);
    }

    #[test]
    fn default_dll_for_arch_x86_64() {
        assert_eq!(default_dll_for_arch(ExeArch::X86_64), DllOverride::Dxgi);
    }

    #[test]
    fn detect_install_status_finds_installed() {
        let base = tempdir().unwrap();
        let game_dir = tempdir().unwrap();
        let arch = ExeArch::X86_64;
        setup_fake_reshade(base.path(), "6.1.0", arch);
        install_reshade(base.path(), game_dir.path(), "6.1.0", DllOverride::Dxgi, arch).unwrap();

        let status = detect_install_status(game_dir.path());
        assert!(matches!(
            status,
            InstallStatus::Installed {
                dll: DllOverride::Dxgi,
                arch: ExeArch::X86_64,
                ..
            }
        ));
    }

    #[test]
    fn detect_install_status_not_installed() {
        let game_dir = tempdir().unwrap();
        let status = detect_install_status(game_dir.path());
        assert!(matches!(status, InstallStatus::NotInstalled));
    }

    #[test]
    fn find_exes_returns_only_exe_files() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), "").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "").unwrap();
        std::fs::write(dir.path().join("engine.exe"), "").unwrap();

        let exes = find_exes(dir.path());
        assert_eq!(exes.len(), 2);
        assert!(exes.iter().all(|p| p.extension().unwrap() == "exe"));
    }
}
