//! Shared types used by background workers.

use std::fmt;

/// A typed progress event emitted by background workers.
///
/// Workers emit this instead of raw strings so that progress semantics are
/// compiler-checked and presentation is decoupled from business logic.
#[derive(Debug)]
pub enum ProgressEvent {
    /// Downloading a `ReShade` version archive.
    Downloading {
        /// The version key being downloaded, e.g. `"6.3.0-Addon"`.
        version: String,
    },
    /// Installing `d3dcompiler_47.dll` into the app data directory.
    InstallingD3dcompiler,
    /// Installing `ReShade` DLLs into the game directory.
    Installing,
    /// Syncing a shader repository (clone or pull).
    SyncingRepo {
        /// The repository's local name.
        name: String,
    },
}

impl fmt::Display for ProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Downloading { version } => write!(f, "Downloading ReShade {version}..."),
            Self::InstallingD3dcompiler => write!(f, "Installing d3dcompiler_47.dll..."),
            Self::Installing => write!(f, "Installing..."),
            Self::SyncingRepo { name } => write!(f, "Syncing {name}..."),
        }
    }
}
