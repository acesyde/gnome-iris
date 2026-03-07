//! ReShade version fetching and extraction.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use regex::Regex;

/// Fetches the latest ReShade version string from `reshade.me`.
pub async fn fetch_latest_version() -> Result<String> {
    let html = reqwest::get("https://reshade.me")
        .await
        .context("Failed to connect to reshade.me")?
        .text()
        .await?;
    parse_version_from_html(&html).context("Could not parse ReShade version from reshade.me")
}

/// Parses the ReShade version string from the HTML of `reshade.me`.
pub fn parse_version_from_html(html: &str) -> Result<String> {
    let re = Regex::new(r#"/downloads/ReShade_(\d+\.\d+\.\d+)(?:_Addon)?\.exe"#)
        .expect("static regex");
    re.captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .ok_or_else(|| anyhow!("No ReShade version found in HTML"))
}

/// Builds the download URL for a given version.
pub fn download_url(version: &str, addon_support: bool) -> String {
    if addon_support {
        format!("https://reshade.me/downloads/ReShade_{version}_Addon.exe")
    } else {
        format!("https://reshade.me/downloads/ReShade_{version}.exe")
    }
}

/// Downloads a ReShade `.exe` and extracts it to `dest_dir`.
///
/// The `.exe` is a self-extracting zip. We extract it directly with the `zip` crate.
pub async fn download_and_extract(url: &str, dest_dir: &Path) -> Result<()> {
    let bytes = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to download {url}"))?
        .bytes()
        .await?;
    std::fs::create_dir_all(dest_dir)?;
    extract_zip_from_bytes(&bytes, dest_dir)?;
    Ok(())
}

/// Extracts all `.dll` entries from a zip archive contained in `bytes` into `dest_dir`.
pub fn extract_zip_from_bytes(bytes: &[u8], dest_dir: &Path) -> Result<()> {
    use std::io::Cursor;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Not a valid zip archive")?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        // Only extract DLL files we care about
        if !name.ends_with(".dll") {
            continue;
        }
        let dest = dest_dir.join(&name);
        let mut out = std::fs::File::create(&dest)
            .with_context(|| format!("Cannot create {}", dest.display()))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Returns the versioned directory for a given ReShade version.
pub fn version_dir(base: &Path, version: &str) -> PathBuf {
    base.join("reshade").join(version)
}

/// Updates the `latest` symlink to point to `version_dir`.
pub fn update_latest_symlink(base: &Path, version: &str) -> Result<()> {
    let latest = base.join("reshade/latest");
    let target = PathBuf::from(version);
    if latest.exists() || latest.is_symlink() {
        std::fs::remove_file(&latest)?;
    }
    std::os::unix::fs::symlink(target, latest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_from_html_standard() {
        let html = r#"<html><body>
            <a href="/downloads/ReShade_6.1.0.exe">Download ReShade 6.1.0</a>
            </body></html>"#;
        let version = parse_version_from_html(html).unwrap();
        assert_eq!(version, "6.1.0");
    }

    #[test]
    fn parse_version_from_html_addon() {
        let html = r#"<a href="/downloads/ReShade_6.1.0_Addon.exe">Download</a>"#;
        let version = parse_version_from_html(html).unwrap();
        assert_eq!(version, "6.1.0");
    }

    #[test]
    fn parse_version_returns_err_when_not_found() {
        let html = "<html>no reshade here</html>";
        assert!(parse_version_from_html(html).is_err());
    }

    #[test]
    fn build_download_url_standard() {
        let url = download_url("6.1.0", false);
        assert_eq!(url, "https://reshade.me/downloads/ReShade_6.1.0.exe");
    }

    #[test]
    fn build_download_url_addon() {
        let url = download_url("6.1.0", true);
        assert_eq!(url, "https://reshade.me/downloads/ReShade_6.1.0_Addon.exe");
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
