//! Integration test: full extract → install → detect → uninstall flow.
//!
//! Verifies that `extract_zip_from_bytes`, `install_reshade`, `detect_install_status`,
//! `list_installed_versions`, and `uninstall_reshade` work together end-to-end.

use std::io::Write as _;

use gnome_iris::reshade::game::{DllOverride, ExeArch, InstallStatus};
use gnome_iris::reshade::install::{detect_install_status, install_reshade, uninstall_reshade};
use gnome_iris::reshade::reshade::{extract_zip_from_bytes, list_installed_versions};
use tempfile::tempdir;

/// Builds a minimal zip archive containing `ReShade32.dll` and `ReShade64.dll`.
fn make_reshade_zip() -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    let mut buf = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
    let opts = SimpleFileOptions::default();
    zip.start_file("ReShade32.dll", opts).unwrap();
    zip.write_all(b"fake reshade 32-bit").unwrap();
    zip.start_file("ReShade64.dll", opts).unwrap();
    zip.write_all(b"fake reshade 64-bit").unwrap();
    zip.finish().unwrap();
    buf
}

/// Populates a fake `ReShade` version directory under `base/reshade/<version>/`
/// by extracting a mock zip, and creates the d3dcompiler stub.
fn setup_version(base: &std::path::Path, version: &str, arch: ExeArch) {
    let version_dir = base.join("reshade").join(version);
    std::fs::create_dir_all(&version_dir).unwrap();
    extract_zip_from_bytes(&make_reshade_zip(), &version_dir).unwrap();
    let suffix = arch.d3dcompiler_suffix();
    std::fs::write(base.join(format!("d3dcompiler_47.dll.{suffix}")), b"fake d3dc").unwrap();
}

#[test]
fn extract_then_install_then_detect_then_uninstall() {
    let base = tempdir().unwrap();
    let game_dir = tempdir().unwrap();
    let version = "6.7.3";
    let arch = ExeArch::X86_64;
    let dll = DllOverride::Dxgi;

    // Step 1: Extract mock zip into the versioned directory.
    setup_version(base.path(), version, arch);

    let version_dir = base.path().join("reshade").join(version);
    assert!(version_dir.join("ReShade64.dll").exists(), "ReShade64.dll should be extracted");
    assert!(version_dir.join("ReShade32.dll").exists(), "ReShade32.dll should be extracted");

    // Step 2: Install ReShade into the game directory.
    install_reshade(base.path(), game_dir.path(), "testgame", &[], version, dll, arch).unwrap();

    assert!(game_dir.path().join("dxgi.dll").is_symlink(), "dxgi.dll symlink missing after install");
    assert!(
        game_dir.path().join("d3dcompiler_47.dll").is_symlink(),
        "d3dcompiler_47.dll symlink missing after install"
    );

    // Step 3: Detect status → must be Installed with the correct version.
    let status = detect_install_status(game_dir.path());
    assert!(
        matches!(
            &status,
            InstallStatus::Installed {
                dll: DllOverride::Dxgi,
                arch: ExeArch::X86_64,
                version: Some(v),
            } if v == version
        ),
        "Expected Installed{{Dxgi, X86_64, {version}}}, got {status:?}"
    );

    // Step 4: Uninstall.
    uninstall_reshade(game_dir.path(), dll, base.path(), "testgame").unwrap();

    assert!(!game_dir.path().join("dxgi.dll").exists(), "dxgi.dll should be gone after uninstall");
    assert!(!game_dir.path().join("d3dcompiler_47.dll").exists(), "d3dcompiler_47.dll should be gone after uninstall");

    // Step 5: Detect status → NotInstalled.
    let status_after = detect_install_status(game_dir.path());
    assert!(
        matches!(status_after, InstallStatus::NotInstalled),
        "Expected NotInstalled after uninstall, got {status_after:?}"
    );
}

#[test]
fn list_installed_versions_reflects_extracted_archives() {
    let base = tempdir().unwrap();

    for version in &["6.7.0", "6.7.3", "6.7.3-Addon"] {
        let version_dir = base.path().join("reshade").join(version);
        std::fs::create_dir_all(&version_dir).unwrap();
        extract_zip_from_bytes(&make_reshade_zip(), &version_dir).unwrap();
    }

    let versions = list_installed_versions(base.path()).unwrap();
    assert_eq!(versions, vec!["6.7.0", "6.7.3", "6.7.3-Addon"]);
}

#[test]
fn x86_install_uses_reshade32_dll() {
    let base = tempdir().unwrap();
    let game_dir = tempdir().unwrap();
    let version = "6.7.3";
    let arch = ExeArch::X86;
    let dll = DllOverride::D3d9;
    setup_version(base.path(), version, arch);

    install_reshade(base.path(), game_dir.path(), "testgame", &[], version, dll, arch).unwrap();

    let status = detect_install_status(game_dir.path());
    assert!(
        matches!(
            &status,
            InstallStatus::Installed {
                dll: DllOverride::D3d9,
                arch: ExeArch::X86,
                ..
            }
        ),
        "Expected Installed{{D3d9, X86, ..}}, got {status:?}"
    );
}
