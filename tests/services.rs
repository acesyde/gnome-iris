//! Integration tests for `DefaultReShadeProvider`, `DefaultGameRepository`, and
//! `DefaultShaderSyncService` — the adapters that bridge the domain layer to the UI workers.

use std::path::PathBuf;

use gnome_iris::reshade::app_state::AppState;
use gnome_iris::reshade::cache::UpdateCache;
use gnome_iris::reshade::config::{GlobalConfig, ShaderRepo};
use gnome_iris::reshade::game::{Game, GameSource};
use gnome_iris::reshade::services::{
    DefaultGameRepository, DefaultReShadeProvider, DefaultShaderSyncService, GameRepository,
    ReShadeProvider, ShaderSyncService,
};
use tempfile::tempdir;

// ── DefaultReShadeProvider ───────────────────────────────────────────────────

#[test]
fn list_installed_versions_returns_versions_from_temp_dir() {
    let dir = tempdir().unwrap();
    let reshade_dir = dir.path().join("reshade");
    std::fs::create_dir_all(reshade_dir.join("6.7.0")).unwrap();
    std::fs::create_dir_all(reshade_dir.join("6.7.3")).unwrap();
    std::fs::create_dir_all(reshade_dir.join("6.7.3-Addon")).unwrap();

    let provider = DefaultReShadeProvider::new(dir.path().to_path_buf());
    let versions = provider.list_installed_versions().unwrap();

    assert_eq!(versions, vec!["6.7.0", "6.7.3", "6.7.3-Addon"]);
}

#[test]
fn list_installed_versions_returns_empty_when_no_reshade_dir() {
    let dir = tempdir().unwrap();
    let provider = DefaultReShadeProvider::new(dir.path().to_path_buf());
    assert!(provider.list_installed_versions().unwrap().is_empty());
}

#[tokio::test]
async fn download_and_extract_skips_download_when_dll_already_present() {
    let dir = tempdir().unwrap();
    let version = "6.7.3";

    // Pre-populate the version directory so the network download is bypassed.
    // `download_and_extract` only downloads if `ReShade64.dll` is absent.
    let version_dir = dir.path().join("reshade").join(version);
    std::fs::create_dir_all(&version_dir).unwrap();
    std::fs::write(version_dir.join("ReShade64.dll"), b"fake dll").unwrap();

    let provider = DefaultReShadeProvider::new(dir.path().to_path_buf());
    provider.download_and_extract(version, false).await.unwrap();

    assert!(version_dir.join("ReShade64.dll").exists(), "ReShade64.dll should still be present");

    // The implementation records the version in the update cache after a successful call.
    let installed = UpdateCache::new(dir.path().to_path_buf()).read_installed().unwrap();
    assert!(
        installed.contains(&version.to_owned()),
        "version {version} should be recorded in the update cache"
    );
}

// ── DefaultGameRepository ────────────────────────────────────────────────────

#[test]
fn games_reflects_in_memory_list() {
    let dir = tempdir().unwrap();
    let state = AppState {
        games: vec![
            Game::new("Half-Life 2".into(), PathBuf::from("/games/hl2"), GameSource::Manual),
            Game::new("Portal".into(), PathBuf::from("/games/portal"), GameSource::Manual),
        ],
        reshade_version: None,
        config: GlobalConfig::default(),
        data_dir: dir.path().to_path_buf(),
    };

    let repo = DefaultGameRepository::new(state);

    assert_eq!(repo.games().len(), 2);
    assert_eq!(repo.games()[0].name, "Half-Life 2");
    assert_eq!(repo.games()[1].name, "Portal");
}

#[test]
fn save_games_persists_to_disk() {
    let dir = tempdir().unwrap();
    let state = AppState {
        games: vec![],
        reshade_version: None,
        config: GlobalConfig::default(),
        data_dir: dir.path().to_path_buf(),
    };

    let mut repo = DefaultGameRepository::new(state);
    let new_games =
        vec![Game::new("Cyberpunk 2077".into(), PathBuf::from("/games/cp2077"), GameSource::Manual)];
    repo.save_games(&new_games).unwrap();

    let reloaded = AppState::load_from(dir.path().to_path_buf());
    assert_eq!(reloaded.games.len(), 1);
    assert_eq!(reloaded.games[0].name, "Cyberpunk 2077");
}

// ── DefaultShaderSyncService ─────────────────────────────────────────────────

fn make_local_shader_repo(path: &std::path::Path) {
    let repo = git2::Repository::init(path).unwrap();
    std::fs::create_dir_all(path.join("Shaders")).unwrap();
    std::fs::write(path.join("Shaders/test.fx"), b"// test shader").unwrap();
    let mut index = repo.index().unwrap();
    index.add_all(std::iter::once(&"*"), git2::IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
}

#[test]
fn sync_repo_clones_local_fixture_via_service() {
    let source = tempdir().unwrap();
    let repos = tempdir().unwrap();
    make_local_shader_repo(source.path());

    let shader_repo = ShaderRepo {
        url: format!("file://{}", source.path().display()),
        local_name: "test-shaders".into(),
        branch: None,
        enabled_by_default: true,
    };

    let service = DefaultShaderSyncService;
    service.sync_repo(&shader_repo, repos.path()).unwrap();

    assert!(
        repos.path().join("test-shaders/Shaders/test.fx").exists(),
        "Shaders/test.fx should be cloned by the service"
    );
}

#[test]
fn rebuild_merged_creates_symlinks_via_service() {
    let repos = tempdir().unwrap();
    let shaders_dir = repos.path().join("my-shaders/Shaders");
    std::fs::create_dir_all(&shaders_dir).unwrap();
    std::fs::write(shaders_dir.join("effect.fx"), b"// effect").unwrap();

    let service = DefaultShaderSyncService;
    service.rebuild_merged(repos.path(), &[]).unwrap();

    assert!(
        repos.path().join("Merged/Shaders/effect.fx").is_symlink(),
        "effect.fx should be symlinked into Merged/Shaders"
    );
}
