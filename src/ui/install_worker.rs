//! Async worker for downloading and installing `ReShade`.

use std::path::PathBuf;

use anyhow::Context as _;
use relm4::{ComponentSender, Worker};

use crate::reshade::cache::UpdateCache;
use crate::reshade::game::{DllOverride, ExeArch};
use crate::reshade::{d3dcompiler, install, reshade};

/// Input commands for the install worker.
#[derive(Debug)]
pub enum Controls {
    /// Install a pre-cached `ReShade` version into the given game directory.
    Install {
        /// App data directory.
        data_dir: PathBuf,
        /// Game directory to install into.
        game_dir: PathBuf,
        /// DLL override to use.
        dll: DllOverride,
        /// Executable architecture.
        arch: ExeArch,
        /// The cached version key to install, e.g. `"v6.3.0"` or `"v6.3.0-Addon"`.
        version: String,
    },
    /// Remove `ReShade` from the given game directory.
    Uninstall {
        /// Game directory to uninstall from.
        game_dir: PathBuf,
        /// The DLL override currently in use.
        dll: DllOverride,
    },
    /// Download a specific `ReShade` version to the local cache (no game install).
    DownloadVersion {
        /// App data directory.
        data_dir: PathBuf,
        /// The version string to download, e.g. `"6.1.0"`.
        version: String,
        /// Whether to download the Addon Support variant.
        addon: bool,
    },
}

/// Output signals from the install worker.
#[derive(Debug)]
pub enum Signal {
    /// A step completed — carries a human-readable status message.
    Progress(String),
    /// Installation finished successfully.
    InstallComplete {
        /// The installed `ReShade` version string.
        version: String,
    },
    /// Uninstall finished successfully.
    UninstallComplete,
    /// Version download (cache only) completed.
    DownloadVersionComplete {
        /// The full version key, e.g. `"6.1.0"` or `"6.1.0-Addon"`.
        version_key: String,
    },
    /// Version download (cache only) failed — carries a human-readable message.
    DownloadVersionError(String),
    /// An error occurred during a game install or uninstall.
    Error(String),
}

/// Background install worker (no widget tree).
pub struct InstallWorker;

#[allow(missing_docs)]
impl Worker for InstallWorker {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    fn init((): (), _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Install {
                data_dir,
                game_dir,
                dll,
                arch,
                version,
            } => {
                relm4::spawn(async move {
                    if let Err(e) = do_install(&data_dir, &game_dir, dll, arch, &version, &sender) {
                        sender.output(Signal::Error(e.to_string())).ok();
                    }
                });
            },
            Controls::Uninstall { game_dir, dll } => match install::uninstall_reshade(&game_dir, dll) {
                Ok(()) => {
                    sender.output(Signal::UninstallComplete).ok();
                },
                Err(e) => {
                    sender.output(Signal::Error(e.to_string())).ok();
                },
            },
            Controls::DownloadVersion {
                data_dir,
                version,
                addon,
            } => {
                relm4::spawn(async move {
                    if let Err(e) = do_download_version(&data_dir, &version, addon, &sender).await {
                        sender.output(Signal::DownloadVersionError(e.to_string())).ok();
                    }
                });
            },
        }
    }
}

async fn do_download_version(
    data_dir: &std::path::Path,
    version: &str,
    addon: bool,
    sender: &ComponentSender<InstallWorker>,
) -> anyhow::Result<()> {
    let dir_key = if addon { format!("{version}-Addon") } else { version.to_owned() };
    sender.output(Signal::Progress(format!("Downloading ReShade {dir_key}..."))).ok();
    let version_dir = reshade::version_dir(data_dir, &dir_key);
    if !version_dir.join(ExeArch::X86_64.reshade_dll()).exists() {
        let url = reshade::download_url(version, addon);
        reshade::download_and_extract(&url, &version_dir).await?;
    }
    let cache = UpdateCache::new(data_dir.to_path_buf());
    if let Err(e) = cache.add_installed(&dir_key) {
        log::warn!("Could not update installed versions cache: {e}");
    }
    sender
        .output(Signal::DownloadVersionComplete {
            version_key: dir_key,
        })
        .ok();
    Ok(())
}

fn do_install(
    data_dir: &std::path::Path,
    game_dir: &std::path::Path,
    dll: DllOverride,
    arch: ExeArch,
    version: &str,
    sender: &ComponentSender<InstallWorker>,
) -> anyhow::Result<()> {
    let version_dir = reshade::version_dir(data_dir, version);
    if !version_dir.join(arch.reshade_dll()).exists() {
        anyhow::bail!("ReShade {version} is not cached locally — download it in Preferences first");
    }

    if !d3dcompiler::is_installed(data_dir, arch) {
        sender.output(Signal::Progress("Installing d3dcompiler_47.dll...".into())).ok();
    }
    d3dcompiler::ensure(data_dir, arch).context("Failed to install d3dcompiler_47.dll")?;

    sender.output(Signal::Progress("Installing...".into())).ok();
    install::install_reshade(data_dir, game_dir, version, dll, arch)?;

    // cache.add_installed intentionally omitted: the version is already
    // registered in the cache from the Preferences download step.

    sender
        .output(Signal::InstallComplete {
            version: version.to_owned(),
        })
        .ok();
    Ok(())
}
