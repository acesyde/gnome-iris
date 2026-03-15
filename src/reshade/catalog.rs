//! Curated list of known `ReShade` shader repositories.
//!
//! The catalog data lives in `data/catalog.json` and is embedded into the
//! binary at compile time. Editing that file is all that is needed to add,
//! remove, or update a repository — no Rust recompile of this module is
//! required beyond the normal incremental rebuild triggered by file changes.

use std::sync::LazyLock;

use serde::Deserialize;

use crate::reshade::config::ShaderRepo;

/// A known shader repository from the community catalog.
#[derive(Debug, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct CatalogEntry {
    /// Display name shown in the Shaders tab.
    pub name: String,
    /// Short description of the shaders included.
    pub description: String,
    /// Local directory name under `ReShade_shaders/`.
    pub local_name: String,
    /// Remote HTTPS URL.
    pub url: String,
    /// Optional branch; `None` clones the default branch.
    #[serde(default)]
    pub branch: Option<String>,
}

impl CatalogEntry {
    /// Converts this catalog entry into a [`ShaderRepo`] suitable for syncing.
    #[must_use]
    pub fn to_shader_repo(&self) -> ShaderRepo {
        ShaderRepo {
            url: self.url.clone(),
            local_name: self.local_name.clone(),
            branch: self.branch.clone(),
            enabled_by_default: false,
        }
    }
}

/// All known shader repositories, in display order.
///
/// Loaded from the embedded `data/catalog.json` asset on first access.
pub static KNOWN_REPOS: LazyLock<Vec<CatalogEntry>> = LazyLock::new(|| {
    let json = include_str!("../../data/catalog.json");
    serde_json::from_str(json).expect("data/catalog.json is invalid — this is a compile-time asset")
});
