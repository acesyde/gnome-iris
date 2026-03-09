//! Core game data model.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use crate::reshade::config::ShaderOverrides;

/// A game known to the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// SHA-512 hex of the canonical game path — used as a stable identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Directory containing the game's `.exe`.
    pub path: PathBuf,
    /// Where this game was discovered.
    pub source: GameSource,
    /// Current ReShade install status.
    pub status: InstallStatus,
    /// Per-game shader repo opt-outs.
    pub shader_overrides: ShaderOverrides,
    /// Preferred architecture — set at add time or auto-detected from the exe.
    ///
    /// `None` means "use the default for the detected exe at install time".
    #[serde(default)]
    pub preferred_arch: Option<ExeArch>,
}

impl Game {
    /// Creates a new uninstalled game entry.
    pub fn new(name: String, path: PathBuf, source: GameSource) -> Self {
        let id = Self::make_id(&path);
        Self {
            id,
            name,
            path,
            source,
            status: InstallStatus::NotInstalled,
            shader_overrides: ShaderOverrides::default(),
            preferred_arch: None,
        }
    }

    /// Derives a stable ID from the game path (SHA-512 hex).
    pub fn make_id(path: &PathBuf) -> String {
        let mut hasher = Sha512::new();
        hasher.update(path.to_string_lossy().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// How a game was discovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameSource {
    /// Discovered from the Steam library.
    Steam {
        /// Steam App ID.
        app_id: u32,
    },
    /// Added manually by the user.
    Manual,
}

/// ReShade installation status for a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallStatus {
    /// ReShade is not installed for this game.
    NotInstalled,
    /// ReShade is installed with these settings.
    Installed {
        /// The DLL that ReShade is masquerading as.
        dll: DllOverride,
        /// Detected executable architecture.
        arch: ExeArch,
    },
}

impl InstallStatus {
    /// Returns `true` if ReShade is currently installed.
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Installed { .. })
    }
}

/// The Windows DLL name that ReShade replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DllOverride {
    /// `d3d8.dll`
    D3d8,
    /// `d3d9.dll`
    D3d9,
    /// `d3d11.dll`
    D3d11,
    /// `ddraw.dll`
    Ddraw,
    /// `dinput8.dll`
    Dinput8,
    /// `dxgi.dll` (default for 64-bit DirectX games)
    Dxgi,
    /// `opengl32.dll`
    OpenGl32,
}

impl DllOverride {
    /// Returns the filename used for the symlink in the game directory.
    pub fn symlink_name(self) -> &'static str {
        match self {
            Self::D3d8 => "d3d8.dll",
            Self::D3d9 => "d3d9.dll",
            Self::D3d11 => "d3d11.dll",
            Self::Ddraw => "ddraw.dll",
            Self::Dinput8 => "dinput8.dll",
            Self::Dxgi => "dxgi.dll",
            Self::OpenGl32 => "opengl32.dll",
        }
    }

    /// All supported DLL overrides (for UI dropdown).
    pub fn all() -> &'static [Self] {
        &[
            Self::D3d8,
            Self::D3d9,
            Self::D3d11,
            Self::Ddraw,
            Self::Dinput8,
            Self::Dxgi,
            Self::OpenGl32,
        ]
    }
}

impl std::fmt::Display for DllOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.symlink_name())
    }
}

/// Executable architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExeArch {
    /// 32-bit (x86).
    X86,
    /// 64-bit (x86-64).
    X86_64,
}

impl ExeArch {
    /// Returns the ReShade DLL filename for this architecture.
    pub fn reshade_dll(self) -> &'static str {
        match self {
            Self::X86 => "ReShade32.dll",
            Self::X86_64 => "ReShade64.dll",
        }
    }

    /// Returns the d3dcompiler suffix for this architecture.
    pub fn d3dcompiler_suffix(self) -> &'static str {
        match self {
            Self::X86 => "32",
            Self::X86_64 => "64",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_override_symlink_name() {
        assert_eq!(DllOverride::Dxgi.symlink_name(), "dxgi.dll");
        assert_eq!(DllOverride::D3d9.symlink_name(), "d3d9.dll");
        assert_eq!(DllOverride::OpenGl32.symlink_name(), "opengl32.dll");
    }

    #[test]
    fn game_id_is_deterministic() {
        let path = PathBuf::from("/home/user/.steam/game");
        let id1 = Game::make_id(&path);
        let id2 = Game::make_id(&path);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 128); // SHA-512 hex = 128 chars
    }

    #[test]
    fn game_id_differs_for_different_paths() {
        let a = Game::make_id(&PathBuf::from("/game/a"));
        let b = Game::make_id(&PathBuf::from("/game/b"));
        assert_ne!(a, b);
    }

    #[test]
    fn install_status_is_installed() {
        let status = InstallStatus::Installed {
            dll: DllOverride::Dxgi,
            arch: ExeArch::X86_64,
        };
        assert!(status.is_installed());
        assert!(!InstallStatus::NotInstalled.is_installed());
    }
}
