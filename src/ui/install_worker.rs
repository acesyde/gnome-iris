//! Async worker for downloading and installing ReShade.

use std::path::PathBuf;

use relm4::{ComponentSender, Worker};

use crate::reshade::cache::UpdateCache;
use crate::reshade::game::{DllOverride, ExeArch};
use crate::reshade::{install, reshade};

/// Input commands for the install worker.
#[derive(Debug)]
pub enum Controls {
    /// Download the latest ReShade and install into the given game directory.
    Install {
        /// App data directory.
        data_dir: PathBuf,
        /// Game directory to install into.
        game_dir: PathBuf,
        /// DLL override to use.
        dll: DllOverride,
        /// Executable architecture.
        arch: ExeArch,
    },
    /// Remove ReShade from the given game directory.
    Uninstall {
        /// Game directory to uninstall from.
        game_dir: PathBuf,
        /// The DLL override currently in use.
        dll: DllOverride,
    },
    /// Download a specific ReShade version to the local cache (no game install).
    DownloadVersion {
        /// App data directory.
        data_dir: PathBuf,
        /// The version string to download, e.g. `"6.1.0"`.
        version: String,
    },
}

/// Output signals from the install worker.
#[derive(Debug)]
pub enum Signal {
    /// A step completed — carries a human-readable status message.
    Progress(String),
    /// Installation finished successfully.
    InstallComplete {
        /// The installed ReShade version string.
        version: String,
    },
    /// Uninstall finished successfully.
    UninstallComplete,
    /// Version download (cache only) completed.
    DownloadVersionComplete {
        /// The downloaded ReShade version string.
        version: String,
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

    fn init(_: (), _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::Install {
                data_dir,
                game_dir,
                dll,
                arch,
            } => {
                let sender2 = sender.clone();
                relm4::spawn(async move {
                    if let Err(e) =
                        do_install(&data_dir, &game_dir, dll, arch, &sender2).await
                    {
                        sender2.output(Signal::Error(e.to_string())).ok();
                    }
                });
            }
            Controls::Uninstall { game_dir, dll } => {
                match install::uninstall_reshade(&game_dir, dll) {
                    Ok(()) => {
                        sender.output(Signal::UninstallComplete).ok();
                    }
                    Err(e) => {
                        sender.output(Signal::Error(e.to_string())).ok();
                    }
                }
            }
            Controls::DownloadVersion { data_dir, version } => {
                let sender2 = sender.clone();
                relm4::spawn(async move {
                    if let Err(e) = do_download_version(&data_dir, &version, &sender2).await {
                        sender2
                            .output(Signal::DownloadVersionError(e.to_string()))
                            .ok();
                    }
                });
            }
        }
    }
}

async fn do_download_version(
    data_dir: &std::path::Path,
    version: &str,
    sender: &ComponentSender<InstallWorker>,
) -> anyhow::Result<()> {
    sender
        .output(Signal::Progress(format!("Downloading ReShade {version}...")))
        .ok();
    let version_dir = reshade::version_dir(data_dir, version);
    if !version_dir.join(ExeArch::X86_64.reshade_dll()).exists() {
        let url = reshade::download_url(version, false);
        reshade::download_and_extract(&url, &version_dir).await?;
        reshade::update_latest_symlink(data_dir, version)?;
    }
    let cache = UpdateCache::new(data_dir.to_path_buf());
    if let Err(e) = cache.add_installed(version) {
        log::warn!("Could not update installed versions cache: {e}");
    }
    sender
        .output(Signal::DownloadVersionComplete {
            version: version.to_owned(),
        })
        .ok();
    Ok(())
}

async fn do_install(
    data_dir: &std::path::Path,
    game_dir: &std::path::Path,
    dll: DllOverride,
    arch: ExeArch,
    sender: &ComponentSender<InstallWorker>,
) -> anyhow::Result<()> {
    sender
        .output(Signal::Progress(
            "Fetching latest ReShade version...".into(),
        ))
        .ok();
    let version = reshade::fetch_latest_version().await?;

    let version_dir = reshade::version_dir(data_dir, &version);
    if !version_dir.join(arch.reshade_dll()).exists() {
        sender
            .output(Signal::Progress(format!(
                "Downloading ReShade {version}..."
            )))
            .ok();
        let url = reshade::download_url(&version, false);
        reshade::download_and_extract(&url, &version_dir).await?;
        reshade::update_latest_symlink(data_dir, &version)?;
    }

    sender
        .output(Signal::Progress("Installing...".into()))
        .ok();
    install::install_reshade(data_dir, game_dir, &version, dll, arch)?;

    let cache = UpdateCache::new(data_dir.to_path_buf());
    if let Err(e) = cache.add_installed(&version) {
        log::warn!("Could not update installed versions cache: {e}");
    }

    sender
        .output(Signal::InstallComplete { version })
        .ok();
    Ok(())
}
