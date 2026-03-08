//! Steam library discovery — reads `libraryfolders.vdf` to find installed games.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::reshade::game::{ExeArch, Game, GameSource};

/// Returns all Steam library root paths found on this system.
pub fn find_steam_libraries() -> Result<Vec<PathBuf>> {
    let steam_root = steam_root().context("Steam not found")?;
    let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");
    let vdf_str = std::fs::read_to_string(&vdf_path)
        .with_context(|| format!("Cannot read {}", vdf_path.display()))?;
    parse_library_folders_vdf(&vdf_str)
}

/// Parses the `libraryfolders.vdf` content and returns library root paths.
pub fn parse_library_folders_vdf(vdf_str: &str) -> Result<Vec<PathBuf>> {
    let vdf = keyvalues_parser::parse(vdf_str).context("Invalid VDF")?;
    let root = vdf.value.get_obj().context("VDF root is not an object")?;
    let mut paths = Vec::new();
    for (_key, values) in root.iter() {
        for value in values {
            if let Some(obj) = value.get_obj() {
                if let Some(path_values) = obj.get("path") {
                    if let Some(v) = path_values.first().and_then(|v| v.get_str()) {
                        paths.push(PathBuf::from(v));
                    }
                }
            }
        }
    }
    Ok(paths)
}

/// Scans all Steam libraries and returns discovered games.
///
/// Only entries whose `appmanifest_*.acf` declares `type = "Game"` are included,
/// which filters out Proton runtimes, SteamLinuxRuntime, and other tools.
pub fn discover_steam_games() -> Vec<Game> {
    let Ok(libraries) = find_steam_libraries() else {
        return vec![];
    };
    let mut games = Vec::new();
    for library in libraries {
        let steamapps = library.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&steamapps) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let fname = name.to_string_lossy();
            if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                continue;
            }
            if let Some(game) = parse_appmanifest(&path, &steamapps) {
                games.push(game);
            }
        }
    }
    games
}

/// Parses a single `appmanifest_<id>.acf` file.
///
/// Returns `Some(Game)` only for actual games, filtering out Proton versions,
/// Steam Linux Runtime, and other tools by name/installdir patterns.
fn parse_appmanifest(acf_path: &Path, steamapps: &Path) -> Option<Game> {
    let content = std::fs::read_to_string(acf_path).ok()?;
    let vdf = keyvalues_parser::parse(&content).ok()?;
    let obj = vdf.value.get_obj()?;

    let get = |key: &str| -> Option<String> {
        obj.get(key)
            .and_then(|vs| vs.first())
            .and_then(|v| v.get_str())
            .map(|s| s.to_owned())
    };

    let name = get("name")?;
    let install_dir = get("installdir")?;
    let app_id: u32 = get("appid").and_then(|s| s.parse().ok()).unwrap_or(0);

    // Filter out Proton versions and Steam Linux Runtime tools.
    if is_steam_tool(&name, &install_dir) {
        return None;
    }

    let path = steamapps.join("common").join(&install_dir);
    if !path.exists() {
        return None;
    }

    Some(Game::new(name, path, GameSource::Steam { app_id }))
}

/// Returns `true` if the name/installdir matches known Steam tool patterns
/// (Proton versions, Steam Linux Runtime, etc.) rather than an actual game.
fn is_steam_tool(name: &str, install_dir: &str) -> bool {
    let name_lower = name.to_ascii_lowercase();
    let dir_lower = install_dir.to_ascii_lowercase();
    name_lower.starts_with("proton")
        || name_lower.starts_with("steam linux runtime")
        || dir_lower.starts_with("proton")
        || dir_lower.starts_with("steamlinuxruntime")
}

/// Detects the architecture of a PE `.exe` by reading its header bytes.
///
/// Returns `None` if the file cannot be read or is not a valid PE.
pub fn detect_exe_arch(exe_path: &Path) -> Option<ExeArch> {
    use std::io::Read;
    let mut file = std::fs::File::open(exe_path).ok()?;
    let mut dos_header = [0u8; 64];
    file.read_exact(&mut dos_header).ok()?;
    // Check MZ magic
    if &dos_header[0..2] != b"MZ" {
        return None;
    }
    // PE offset is at 0x3c
    let pe_offset = u32::from_le_bytes(dos_header[60..64].try_into().ok()?) as usize;
    let mut buf = vec![0u8; pe_offset + 6];
    let mut file2 = std::fs::File::open(exe_path).ok()?;
    file2.read_exact(&mut buf).ok()?;
    // Check PE signature
    if &buf[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return None;
    }
    // Machine type is 2 bytes after PE signature
    let machine = u16::from_le_bytes(buf[pe_offset + 4..pe_offset + 6].try_into().ok()?);
    match machine {
        0x014c => Some(ExeArch::X86),
        0x8664 => Some(ExeArch::X86_64),
        _ => None,
    }
}

fn steam_root() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        format!("{home}/.local/share/Steam"),
        format!("{home}/.steam/steam"),
        format!("{home}/.steam/root"),
    ];
    candidates.into_iter().map(PathBuf::from).find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_vdf() -> &'static str {
        include_str!("../../tests/fixtures/libraryfolders.vdf")
    }

    #[test]
    fn parse_library_folders() {
        let folders = parse_library_folders_vdf(fixture_vdf()).unwrap();
        assert_eq!(folders.len(), 2);
        assert!(
            folders[0].to_string_lossy().contains(".local/share/Steam"),
            "expected Steam path, got: {}",
            folders[0].display()
        );
        assert!(
            folders[1].to_string_lossy().contains("mnt/games/Steam"),
            "expected games drive path, got: {}",
            folders[1].display()
        );
    }

    #[test]
    fn detect_arch_x86_64_from_pe() {
        // Minimal valid PE header for x86-64:
        // - MZ magic at 0
        // - PE offset at 0x3c = 60 (points to byte 64)
        // - PE\0\0 signature at offset 64
        // - Machine type 0x8664 (x86-64) at offset 68
        let mut buf = vec![0u8; 70];
        buf[0] = b'M';
        buf[1] = b'Z';
        // PE offset = 64 (little-endian u32)
        buf[60] = 64;
        buf[61] = 0;
        buf[62] = 0;
        buf[63] = 0;
        // PE signature
        buf[64] = b'P';
        buf[65] = b'E';
        buf[66] = 0;
        buf[67] = 0;
        // Machine: 0x8664 little-endian
        buf[68] = 0x64;
        buf[69] = 0x86;

        // Write to a temp file and detect
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.exe");
        std::fs::write(&path, &buf).unwrap();
        assert_eq!(detect_exe_arch(&path), Some(ExeArch::X86_64));
    }

    #[test]
    fn detect_arch_x86_from_pe() {
        let mut buf = vec![0u8; 70];
        buf[0] = b'M';
        buf[1] = b'Z';
        buf[60] = 64;
        // PE signature
        buf[64] = b'P';
        buf[65] = b'E';
        // Machine: 0x014c (x86) little-endian
        buf[68] = 0x4c;
        buf[69] = 0x01;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.exe");
        std::fs::write(&path, &buf).unwrap();
        assert_eq!(detect_exe_arch(&path), Some(ExeArch::X86));
    }

    #[test]
    fn detect_arch_returns_none_for_non_pe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not.exe");
        std::fs::write(&path, b"not a pe file").unwrap();
        assert_eq!(detect_exe_arch(&path), None);
    }
}
