//! Shared application state passed between UI components.

use std::path::PathBuf;
use std::sync::Arc;

use directories::ProjectDirs;
use tokio::sync::RwLock;

use crate::reshade::cache::UpdateCache;
use crate::reshade::config::GlobalConfig;
use crate::reshade::game::Game;

/// Shared mutable application state — wrap in `Arc<RwLock<_>>` to share across components.
pub type Shared<T> = Arc<RwLock<T>>;

/// Top-level application state shared across all Relm4 components.
#[derive(Debug)]
pub struct AppState {
    /// All games known to the application (Steam-discovered + manually added).
    pub games: Vec<Game>,
    /// Currently installed ReShade version, if any.
    pub reshade_version: Option<String>,
    /// Global configuration.
    pub config: GlobalConfig,
    /// Root data directory (`$XDG_DATA_HOME/iris/`).
    pub data_dir: PathBuf,
}

impl AppState {
    /// Initializes app state from disk (or defaults if first run).
    pub fn load() -> Self {
        let data_dir = iris_data_dir();
        let config = load_config(&data_dir);
        let games = load_games(&data_dir);
        let reshade_version = load_reshade_version(&data_dir);
        Self {
            games,
            reshade_version,
            config,
            data_dir,
        }
    }

    /// Persists the current state to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        let config_json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(self.data_dir.join("config.json"), config_json)?;
        let games_json = serde_json::to_string_pretty(&self.games)?;
        std::fs::write(self.data_dir.join("games.json"), games_json)?;
        Ok(())
    }
}

/// Returns the XDG data directory for gnome-iris (`$XDG_DATA_HOME/iris/`).
pub fn iris_data_dir() -> PathBuf {
    ProjectDirs::from("org", "gnome", "Iris")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned());
            PathBuf::from(home).join(".local/share/iris")
        })
}

fn load_config(data_dir: &PathBuf) -> GlobalConfig {
    let path = data_dir.join("config.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_games(data_dir: &PathBuf) -> Vec<Game> {
    let path = data_dir.join("games.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_reshade_version(data_dir: &PathBuf) -> Option<String> {
    UpdateCache::new(data_dir.clone())
        .read_version()
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_reload_config() {
        let dir = tempdir().unwrap();
        let mut state = AppState {
            games: vec![],
            reshade_version: None,
            config: GlobalConfig::default(),
            data_dir: dir.path().to_path_buf(),
        };
        state.config.update_interval_hours = 8;
        state.save().unwrap();

        let reloaded = load_config(&dir.path().to_path_buf());
        assert_eq!(reloaded.update_interval_hours, 8);
    }

    #[test]
    fn save_and_reload_games() {
        use crate::reshade::game::{Game, GameSource};
        let dir = tempdir().unwrap();
        let state = AppState {
            games: vec![Game::new(
                "Test Game".into(),
                PathBuf::from("/games/test"),
                GameSource::Manual,
            )],
            reshade_version: Some("6.1.0".into()),
            config: GlobalConfig::default(),
            data_dir: dir.path().to_path_buf(),
        };
        state.save().unwrap();

        let reloaded = load_games(&dir.path().to_path_buf());
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].name, "Test Game");
    }
}
