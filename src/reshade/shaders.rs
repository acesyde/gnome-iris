//! Shader repository management: clone, update, and merge into a unified directory.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::reshade::config::ShaderRepo;

/// Clones or updates a shader repository.
///
/// If the local directory does not exist, it is cloned from `repo.url`.
/// If it does exist, a fast-forward pull is attempted.
///
/// # Errors
/// Returns an error if cloning or fetching fails.
pub fn sync_repo(repo: &ShaderRepo, repos_dir: &Path) -> Result<()> {
    let dest = repos_dir.join(&repo.local_name);
    if dest.exists() {
        let git_repo =
            git2::Repository::open(&dest).with_context(|| format!("Cannot open repo at {}", dest.display()))?;
        fetch_and_merge(&git_repo)?;
    } else {
        let mut opts = git2::FetchOptions::new();
        opts.download_tags(git2::AutotagOption::None);
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(opts);
        if let Some(branch) = &repo.branch {
            builder.branch(branch);
        }
        builder.clone(&repo.url, &dest).with_context(|| format!("Failed to clone {}", repo.url))?;
    }
    Ok(())
}

fn fetch_and_merge(repo: &git2::Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[] as &[&str], None, None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;
    if analysis.is_fast_forward() {
        let refname = {
            let head = repo.head()?;
            head.name().unwrap_or("refs/heads/main").to_owned()
        };
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }
    Ok(())
}

/// Rebuilds the `Merged/` directory by symlinking all unique shader/texture files.
///
/// Priority is determined by order in `repos`: first repo wins on name collision.
///
/// # Errors
/// Returns an error if directory creation or symlinking fails.
pub fn rebuild_merged(repos_dir: &Path, disabled_repos: &[String]) -> Result<()> {
    let merged_shaders = repos_dir.join("Merged/Shaders");
    let merged_textures = repos_dir.join("Merged/Textures");
    std::fs::create_dir_all(&merged_shaders)?;
    std::fs::create_dir_all(&merged_textures)?;

    let entries = std::fs::read_dir(repos_dir)
        .context("Cannot read repos dir")?
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name != "Merged" && !disabled_repos.contains(&name)
        });

    for entry in entries {
        let shaders_src = entry.path().join("Shaders");
        let textures_src = entry.path().join("Textures");
        if shaders_src.exists() {
            link_shader_files(&shaders_src, &merged_shaders)?;
        }
        if textures_src.exists() {
            link_shader_files(&textures_src, &merged_textures)?;
        }
    }
    Ok(())
}

/// Creates symlinks in `dest_dir` for each file in `src_dir`.
///
/// Skips files that already have a symlink in `dest_dir` (first-wins semantics).
///
/// # Errors
/// Returns an error if reading the source directory or creating a symlink fails.
pub fn link_shader_files(src_dir: &Path, dest_dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src_dir).context("Cannot read shader dir")?.flatten() {
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let dest = dest_dir.join(&file_name);
        if dest.exists() || dest.is_symlink() {
            continue; // first repo wins
        }
        std::os::unix::fs::symlink(&src, &dest)
            .with_context(|| format!("Cannot link {} -> {}", src.display(), dest.display()))?;
    }
    Ok(())
}

/// Returns the path to the merged shaders directory.
#[must_use]
pub fn merged_shaders_dir(base: &Path) -> PathBuf {
    base.join("ReShade_shaders/Merged/Shaders")
}

/// Returns the path to the merged textures directory.
#[must_use]
pub fn merged_textures_dir(base: &Path) -> PathBuf {
    base.join("ReShade_shaders/Merged/Textures")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn merge_creates_symlinks_for_fx_files() {
        let dir = tempdir().unwrap();
        let src_shaders = dir.path().join("repo/Shaders");
        let merged = dir.path().join("Merged/Shaders");
        std::fs::create_dir_all(&src_shaders).unwrap();
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(src_shaders.join("test.fx"), "// shader").unwrap();

        link_shader_files(&src_shaders, &merged).unwrap();

        let link = merged.join("test.fx");
        assert!(link.exists(), "symlink should exist");
        assert!(link.is_symlink(), "should be a symlink");
    }

    #[test]
    fn merge_does_not_overwrite_existing_symlink() {
        let dir = tempdir().unwrap();
        let src1 = dir.path().join("repo1/Shaders");
        let src2 = dir.path().join("repo2/Shaders");
        let merged = dir.path().join("Merged/Shaders");
        std::fs::create_dir_all(&src1).unwrap();
        std::fs::create_dir_all(&src2).unwrap();
        std::fs::create_dir_all(&merged).unwrap();
        std::fs::write(src1.join("common.fx"), "// v1").unwrap();
        std::fs::write(src2.join("common.fx"), "// v2").unwrap();

        link_shader_files(&src1, &merged).unwrap();
        link_shader_files(&src2, &merged).unwrap();

        // Should still point to src1 version (first wins)
        let target = std::fs::read_link(merged.join("common.fx")).unwrap();
        assert!(target.to_string_lossy().contains("repo1"), "expected repo1, got: {}", target.display());
    }

    #[test]
    fn rebuild_merged_excludes_disabled_repos() {
        let dir = tempdir().unwrap();
        let repos_dir = dir.path();
        let enabled = repos_dir.join("enabled-repo/Shaders");
        let disabled = repos_dir.join("disabled-repo/Shaders");
        std::fs::create_dir_all(&enabled).unwrap();
        std::fs::create_dir_all(&disabled).unwrap();
        std::fs::write(enabled.join("good.fx"), "// good").unwrap();
        std::fs::write(disabled.join("bad.fx"), "// bad").unwrap();

        rebuild_merged(repos_dir, &["disabled-repo".to_owned()]).unwrap();

        let merged = repos_dir.join("Merged/Shaders");
        assert!(merged.join("good.fx").exists());
        assert!(!merged.join("bad.fx").exists());
    }
}
