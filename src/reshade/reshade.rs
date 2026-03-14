//! `ReShade` version fetching and extraction.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

#[derive(Deserialize)]
struct GithubTag {
    name: String,
}

/// Fetches the latest `ReShade` version string from the GitHub tags API.
///
/// Queries `https://api.github.com/repos/crosire/reshade/tags` and returns
/// the tag name of the most recent release.
///
/// # Errors
/// Returns an error if the network request fails, the response is not valid
/// JSON, or GitHub returns an empty tag list.
pub async fn fetch_latest_version() -> Result<String> {
    let tags: Vec<GithubTag> = reqwest::Client::new()
        .get("https://api.github.com/repos/crosire/reshade/tags")
        .header(reqwest::header::USER_AGENT, "gnome-iris")
        .send()
        .await
        .context("Failed to connect to GitHub tags API")?
        .error_for_status()
        .context("GitHub tags API returned an error status")?
        .json()
        .await
        .context("Failed to parse GitHub tags API response")?;
    tags.into_iter()
        .next()
        .map(|t| t.name)
        .ok_or_else(|| anyhow!("GitHub tags API returned an empty list"))
}

/// Builds the download URL for a given `ReShade` version.
#[must_use]
pub fn download_url(version: &str, addon_support: bool) -> String {
    let v = version.strip_prefix('v').unwrap_or(version);
    if addon_support {
        format!("https://reshade.me/downloads/ReShade_Setup_{v}_Addon.exe")
    } else {
        format!("https://reshade.me/downloads/ReShade_Setup_{v}.exe")
    }
}

/// Downloads a `ReShade` `.exe` and extracts it to `dest_dir`.
///
/// The `.exe` is a self-extracting zip. We extract it directly with the `zip` crate.
/// Returns an error on network failure, timeout, or a non-2xx HTTP status.
///
/// # Errors
/// Returns an error if the network request fails or if extraction fails.
pub async fn download_and_extract(url: &str, dest_dir: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?
        .error_for_status()
        .with_context(|| format!("Server returned an error for {url}"))?;
    let bytes = response.bytes().await.with_context(|| format!("Failed to read response from {url}"))?;
    std::fs::create_dir_all(dest_dir)?;
    extract_zip_from_bytes(&bytes, dest_dir)?;
    Ok(())
}

/// Extracts all `.dll` entries from a zip archive contained in `bytes` into `dest_dir`.
///
/// # Errors
/// Returns an error if the bytes are not a valid zip archive or if file creation fails.
pub fn extract_zip_from_bytes(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Not a valid zip archive")?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        // Only extract DLL files we care about
        if !std::path::Path::new(&name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("dll")) {
            continue;
        }
        let dest = dest_dir.join(&name);
        let mut out = std::fs::File::create(&dest).with_context(|| format!("Cannot create {}", dest.display()))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Returns the versioned directory for a given `ReShade` version.
#[must_use]
pub fn version_dir(base: &Path, version: &str) -> PathBuf {
    base.join("reshade").join(version)
}

/// Parses a version key (e.g. `"6.7.3"` or `"6.7.3-Addon"`) into a sortable tuple.
///
/// The `-Addon` suffix sorts after the base version of the same number.
fn parse_version_key(s: &str) -> ((u64, u64, u64), bool) {
    let addon = s.ends_with("-Addon");
    let base = s.strip_suffix("-Addon").unwrap_or(s);
    let mut parts = base.splitn(3, '.').map(|p| p.parse::<u64>().unwrap_or(0));
    ((parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0)), addon)
}

/// Returns all installed `ReShade` versions found under `base/reshade/`,
/// sorted in ascending semver order. The `latest` symlink is excluded.
///
/// # Errors
/// Returns an error if the `reshade` directory cannot be read.
pub fn list_installed_versions(base: &Path) -> Result<Vec<String>> {
    let reshade_dir = base.join("reshade");
    if !reshade_dir.exists() {
        return Ok(Vec::new());
    }
    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&reshade_dir).with_context(|| format!("Cannot read {}", reshade_dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() || !path.is_dir() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            versions.push(name.to_owned());
        }
    }
    versions.sort_by_key(|a| parse_version_key(a));
    Ok(versions)
}

/// Returns `true` if `installed` is strictly older than `latest`.
///
/// Both strings may optionally carry a leading `v` (e.g. `"v6.3.0"` or `"6.3.0"`).
/// Returns `false` if either string cannot be parsed as a semver version.
#[must_use]
pub fn is_version_outdated(installed: &str, latest: &str) -> bool {
    use semver::Version;
    let strip_v = |s: &str| s.strip_prefix('v').unwrap_or(s).to_owned();
    match (Version::parse(&strip_v(installed)), Version::parse(&strip_v(latest))) {
        (Ok(i), Ok(l)) => i < l,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_download_url_standard() {
        assert_eq!(download_url("6.7.3", false), "https://reshade.me/downloads/ReShade_Setup_6.7.3.exe");
        // v-prefixed tag names (GitHub API) must be stripped
        assert_eq!(download_url("v6.7.3", false), "https://reshade.me/downloads/ReShade_Setup_6.7.3.exe");
    }

    #[test]
    fn build_download_url_addon() {
        assert_eq!(download_url("6.7.3", true), "https://reshade.me/downloads/ReShade_Setup_6.7.3_Addon.exe");
        assert_eq!(download_url("v6.7.3", true), "https://reshade.me/downloads/ReShade_Setup_6.7.3_Addon.exe");
    }

    #[test]
    fn list_versions_skips_symlink_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        let reshade = dir.path().join("reshade");
        std::fs::create_dir_all(reshade.join("6.0.0")).unwrap();
        std::fs::create_dir_all(reshade.join("6.1.0")).unwrap();
        // Symlinks (e.g. stale "latest" from old installs) must be skipped.
        std::os::unix::fs::symlink("6.1.0", reshade.join("stale-link")).unwrap();
        let versions = list_installed_versions(dir.path()).unwrap();
        assert_eq!(versions, vec!["6.0.0", "6.1.0"]);
    }

    #[test]
    fn list_versions_with_addon_sorts_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let reshade = dir.path().join("reshade");
        std::fs::create_dir_all(reshade.join("6.7.3")).unwrap();
        std::fs::create_dir_all(reshade.join("6.7.3-Addon")).unwrap();
        std::fs::create_dir_all(reshade.join("6.8.0")).unwrap();
        let versions = list_installed_versions(dir.path()).unwrap();
        assert_eq!(versions, vec!["6.7.3", "6.7.3-Addon", "6.8.0"]);
    }

    #[test]
    fn list_versions_empty_when_dir_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_installed_versions(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn version_outdated_when_installed_is_older() {
        assert!(is_version_outdated("6.3.0", "v6.7.3"));
    }

    #[test]
    fn version_not_outdated_when_equal() {
        assert!(!is_version_outdated("6.7.3", "v6.7.3"));
    }

    #[test]
    fn version_not_outdated_when_newer() {
        assert!(!is_version_outdated("6.8.0", "v6.7.3"));
    }

    #[test]
    fn version_not_outdated_on_parse_failure() {
        assert!(!is_version_outdated("unknown", "v6.7.3"));
        assert!(!is_version_outdated("6.7.3", "unknown"));
    }

    #[test]
    fn version_outdated_strips_v_prefix() {
        // installed v-prefixed, latest bare
        assert!(is_version_outdated("v6.3.0", "6.7.3"));
        // both bare
        assert!(is_version_outdated("6.3.0", "6.7.3"));
        // both v-prefixed
        assert!(is_version_outdated("v6.3.0", "v6.7.3"));
    }

    #[test]
    fn extract_zip_extracts_dlls_only() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zip.start_file("ReShade64.dll", opts).unwrap();
            zip.write_all(b"fake dll contents").unwrap();
            zip.start_file("readme.txt", opts).unwrap();
            zip.write_all(b"readme").unwrap();
            zip.finish().unwrap();
        }

        let dir = tempfile::tempdir().unwrap();
        extract_zip_from_bytes(&buf, dir.path()).unwrap();

        assert!(dir.path().join("ReShade64.dll").exists());
        assert!(!dir.path().join("readme.txt").exists());
    }
}
