//! Integration test: `AppState` full save / reload roundtrip via the public API.

use std::path::PathBuf;

use gnome_iris::reshade::app_state::AppState;
use gnome_iris::reshade::config::{GlobalConfig, ShaderRepo};
use gnome_iris::reshade::game::{Game, GameSource};
use tempfile::tempdir;

#[test]
fn save_creates_config_and_games_files() {
    let dir = tempdir().unwrap();
    let state = AppState {
        games: vec![],
        reshade_version: None,
        config: GlobalConfig::default(),
        data_dir: dir.path().to_path_buf(),
    };

    state.save().unwrap();

    assert!(dir.path().join("config.json").exists(), "config.json should be created on save");
    assert!(dir.path().join("games.json").exists(), "games.json should be created on save");
}

#[test]
fn full_roundtrip_preserves_games_and_config() {
    let dir = tempdir().unwrap();

    let original = AppState {
        games: vec![
            Game::new("Half-Life 2".into(), PathBuf::from("/games/hl2"), GameSource::Manual),
            Game::new("Portal".into(), PathBuf::from("/games/portal"), GameSource::Steam { app_id: 400 }),
        ],
        reshade_version: Some("6.7.3".into()),
        config: GlobalConfig {
            shader_repos: vec![ShaderRepo {
                url: "https://example.com/shaders.git".into(),
                local_name: "example-shaders".into(),
                branch: Some("main".into()),
                enabled_by_default: false,
            }],
            merge_shaders: false,
            update_interval_hours: 12,
        },
        data_dir: dir.path().to_path_buf(),
    };

    original.save().unwrap();

    let reloaded = AppState::load_from(dir.path().to_path_buf());

    // Games are preserved.
    assert_eq!(reloaded.games.len(), 2, "both games should reload");
    assert_eq!(reloaded.games[0].name, "Half-Life 2");
    assert_eq!(reloaded.games[1].name, "Portal");
    assert!(matches!(reloaded.games[1].source, GameSource::Steam { app_id: 400 }), "Steam source should be preserved");

    // Config is preserved.
    assert_eq!(reloaded.config.update_interval_hours, 12);
    assert!(!reloaded.config.merge_shaders);
    assert_eq!(reloaded.config.shader_repos.len(), 1);
    assert_eq!(reloaded.config.shader_repos[0].local_name, "example-shaders");
    assert_eq!(reloaded.config.shader_repos[0].branch, Some("main".into()));
}

#[test]
fn missing_files_reload_as_defaults() {
    let dir = tempdir().unwrap();

    // Load from an empty directory — no config.json or games.json present.
    let state = AppState::load_from(dir.path().to_path_buf());

    assert!(state.games.is_empty(), "games should default to empty");
    assert!(!state.config.shader_repos.is_empty(), "config should default to built-in repos");
    assert!(state.reshade_version.is_none());
}
