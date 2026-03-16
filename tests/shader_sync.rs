//! Integration test: `sync_repo` clones a local git repo, then `rebuild_merged`
//! creates the merged shader symlinks.

use gnome_iris::reshade::config::ShaderRepo;
use gnome_iris::reshade::shaders::{rebuild_merged, sync_repo};
use tempfile::tempdir;

/// Creates a local git repository at `path` with two `.fx` shader files committed.
fn make_test_shader_repo(path: &std::path::Path) {
    let repo = git2::Repository::init(path).unwrap();
    std::fs::create_dir_all(path.join("Shaders")).unwrap();
    std::fs::write(path.join("Shaders/test.fx"), b"// test shader").unwrap();
    std::fs::write(path.join("Shaders/another.fx"), b"// another shader").unwrap();

    let mut index = repo.index().unwrap();
    index.add_all(std::iter::once(&"*"), git2::IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
}

#[test]
fn sync_then_rebuild_creates_merged_symlinks() {
    let source_dir = tempdir().unwrap();
    let repos_dir = tempdir().unwrap();

    make_test_shader_repo(source_dir.path());

    let repo = ShaderRepo {
        url: format!("file://{}", source_dir.path().display()),
        local_name: "test-shaders".into(),
        branch: None,
        enabled_by_default: true,
    };

    // Clone the local repo into repos_dir.
    sync_repo(&repo, repos_dir.path()).unwrap();

    let cloned = repos_dir.path().join("test-shaders");
    assert!(cloned.exists(), "cloned directory should exist");
    assert!(cloned.join("Shaders/test.fx").exists(), "Shaders/test.fx should be cloned");

    // Build the Merged/ directory.
    rebuild_merged(repos_dir.path(), &[]).unwrap();

    let merged = repos_dir.path().join("Merged/Shaders");
    assert!(merged.join("test.fx").exists(), "test.fx should be in Merged/Shaders");
    assert!(merged.join("another.fx").exists(), "another.fx should be in Merged/Shaders");
    assert!(merged.join("test.fx").is_symlink(), "test.fx should be a symlink");
    assert!(merged.join("another.fx").is_symlink(), "another.fx should be a symlink");
}

#[test]
fn rebuild_merged_with_existing_dirs_skips_sync() {
    let repos_dir = tempdir().unwrap();

    // Simulate a pre-existing cloned repo without actually running sync_repo.
    let shader_files = repos_dir.path().join("my-shaders/Shaders");
    std::fs::create_dir_all(&shader_files).unwrap();
    std::fs::write(shader_files.join("effect.fx"), b"// effect").unwrap();
    std::fs::write(shader_files.join("lut.fx"), b"// lut").unwrap();

    rebuild_merged(repos_dir.path(), &[]).unwrap();

    let merged = repos_dir.path().join("Merged/Shaders");
    assert!(merged.join("effect.fx").is_symlink(), "effect.fx should be a symlink");
    assert!(merged.join("lut.fx").is_symlink(), "lut.fx should be a symlink");

    // The symlink targets should point back into my-shaders.
    let target = std::fs::read_link(merged.join("effect.fx")).unwrap();
    assert!(
        target.to_string_lossy().contains("my-shaders"),
        "symlink should point into my-shaders, got: {}",
        target.display()
    );
}

#[test]
fn rebuild_merged_disabled_repo_is_excluded() {
    let repos_dir = tempdir().unwrap();

    let enabled = repos_dir.path().join("enabled-repo/Shaders");
    let disabled = repos_dir.path().join("disabled-repo/Shaders");
    std::fs::create_dir_all(&enabled).unwrap();
    std::fs::create_dir_all(&disabled).unwrap();
    std::fs::write(enabled.join("good.fx"), b"// good").unwrap();
    std::fs::write(disabled.join("bad.fx"), b"// bad").unwrap();

    rebuild_merged(repos_dir.path(), &["disabled-repo".to_owned()]).unwrap();

    let merged = repos_dir.path().join("Merged/Shaders");
    assert!(merged.join("good.fx").exists(), "good.fx from enabled repo should be present");
    assert!(!merged.join("bad.fx").exists(), "bad.fx from disabled repo should be absent");
}
