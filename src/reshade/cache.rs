//! Update tracking — stores ReShade version state in a single JSON file.

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Persisted version and update state for ReShade.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionState {
    /// Latest known ReShade version string, e.g. `"6.1.0"`.
    pub latest: Option<String>,
    /// Unix timestamp (seconds) of the last GitHub API check.
    pub last_checked: Option<u64>,
    /// Locally installed versions, sorted in ascending semver order.
    #[serde(default)]
    pub installed: Vec<String>,
}

/// Parses a version key (e.g. `"6.7.3"` or `"6.7.3-Addon"`) into a sortable tuple.
///
/// The `-Addon` suffix sorts after the base version of the same number.
fn parse_version_key(s: &str) -> ((u64, u64, u64), bool) {
    let addon = s.ends_with("-Addon");
    let base = s.strip_suffix("-Addon").unwrap_or(s);
    let mut parts = base.splitn(3, '.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        ),
        addon,
    )
}

/// Manages the `reshade_state.json` file under the iris data directory.
pub struct UpdateCache {
    base: PathBuf,
}

impl UpdateCache {
    /// Creates a new cache pointing at the given directory.
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    fn state_path(&self) -> PathBuf {
        self.base.join("reshade_state.json")
    }

    fn read_state(&self) -> Result<VersionState> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(VersionState::default());
        }
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    fn write_state(&self, state: &VersionState) -> Result<()> {
        std::fs::create_dir_all(&self.base)?;
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(self.state_path(), json)?;
        Ok(())
    }

    /// Returns the last recorded ReShade version, or `None` if unknown.
    pub fn read_version(&self) -> Result<Option<String>> {
        Ok(self.read_state()?.latest)
    }

    /// Writes the current ReShade version to disk.
    pub fn write_version(&self, version: &str) -> Result<()> {
        let mut state = self.read_state()?;
        state.latest = Some(version.to_owned());
        self.write_state(&state)
    }

    /// Updates the last-checked timestamp to now.
    pub fn touch(&self) -> Result<()> {
        let mut state = self.read_state()?;
        state.last_checked = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        );
        self.write_state(&state)
    }

    /// Returns `true` if more than `interval_hours` have passed since the last update.
    pub fn needs_update(&self, interval_hours: u64) -> bool {
        let Ok(state) = self.read_state() else {
            return true;
        };
        let Some(ts) = state.last_checked else {
            return true;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(ts) >= interval_hours * 3600
    }

    /// Records a newly installed version, keeping the list sorted.
    pub fn add_installed(&self, version: &str) -> Result<()> {
        let mut state = self.read_state()?;
        if !state.installed.contains(&version.to_owned()) {
            state.installed.push(version.to_owned());
            state
                .installed
                .sort_by(|a, b| parse_version_key(a).cmp(&parse_version_key(b)));
        }
        self.write_state(&state)
    }

    /// Removes a version from the installed list.
    pub fn remove_installed(&self, version: &str) -> Result<()> {
        let mut state = self.read_state()?;
        state.installed.retain(|v| v != version);
        self.write_state(&state)
    }

    /// Returns the list of locally installed versions.
    pub fn read_installed(&self) -> Result<Vec<String>> {
        Ok(self.read_state()?.installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_and_read_version() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.write_version("6.1.0").unwrap();
        assert_eq!(cache.read_version().unwrap().as_deref(), Some("6.1.0"));
    }

    #[test]
    fn read_version_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        assert_eq!(cache.read_version().unwrap(), None);
    }

    #[test]
    fn needs_update_when_no_timestamp() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        assert!(cache.needs_update(4));
    }

    #[test]
    fn does_not_need_update_when_recent() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.touch().unwrap();
        assert!(!cache.needs_update(4));
    }

    #[test]
    fn add_and_read_installed() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.add_installed("6.1.0").unwrap();
        cache.add_installed("6.0.0").unwrap();
        cache.add_installed("6.1.0").unwrap(); // duplicate
        let installed = cache.read_installed().unwrap();
        assert_eq!(installed, vec!["6.0.0", "6.1.0"]);
    }

    #[test]
    fn add_installed_with_addon_key_sorts_correctly() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.add_installed("6.7.3-Addon").unwrap();
        cache.add_installed("6.7.3").unwrap();
        cache.add_installed("6.8.0").unwrap();
        assert_eq!(
            cache.read_installed().unwrap(),
            vec!["6.7.3", "6.7.3-Addon", "6.8.0"]
        );
    }

    #[test]
    fn remove_installed() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.add_installed("6.0.0").unwrap();
        cache.add_installed("6.1.0").unwrap();
        cache.remove_installed("6.0.0").unwrap();
        assert_eq!(cache.read_installed().unwrap(), vec!["6.1.0"]);
    }

    #[test]
    fn remove_installed_nonexistent_is_noop() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.add_installed("6.1.0").unwrap();
        cache.remove_installed("9.9.9").unwrap();
        assert_eq!(cache.read_installed().unwrap(), vec!["6.1.0"]);
    }

    #[test]
    fn state_persists_across_reads() {
        let dir = tempdir().unwrap();
        let cache = UpdateCache::new(dir.path().to_path_buf());
        cache.write_version("6.1.0").unwrap();
        cache.touch().unwrap();
        cache.add_installed("6.1.0").unwrap();

        let state_json =
            std::fs::read_to_string(dir.path().join("reshade_state.json")).unwrap();
        assert!(state_json.contains("\"latest\""));
        assert!(state_json.contains("\"last_checked\""));
        assert!(state_json.contains("\"installed\""));
    }
}
