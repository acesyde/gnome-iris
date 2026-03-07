//! Update tracking — stores the last known ReShade version and update timestamp.

use std::path::PathBuf;

use anyhow::Result;

/// Manages the version and timestamp files under the iris data directory.
pub struct UpdateCache {
    base: PathBuf,
}

impl UpdateCache {
    /// Creates a new cache pointing at the given directory.
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    /// Returns the last recorded ReShade version, or `None` if unknown.
    pub fn read_version(&self) -> Result<Option<String>> {
        let path = self.base.join("LVERS");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(path)?.trim().to_owned()))
    }

    /// Writes the current ReShade version to disk.
    pub fn write_version(&self, version: &str) -> Result<()> {
        std::fs::create_dir_all(&self.base)?;
        std::fs::write(self.base.join("LVERS"), version)?;
        Ok(())
    }

    /// Writes the current timestamp to the `LASTUPDATED` file.
    pub fn touch(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        std::fs::write(self.base.join("LASTUPDATED"), now.to_string())?;
        Ok(())
    }

    /// Returns `true` if more than `interval_hours` have passed since the last update.
    pub fn needs_update(&self, interval_hours: u64) -> bool {
        let path = self.base.join("LASTUPDATED");
        let Ok(content) = std::fs::read_to_string(path) else {
            return true;
        };
        let Ok(ts) = content.trim().parse::<u64>() else {
            return true;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(ts) >= interval_hours * 3600
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
}
