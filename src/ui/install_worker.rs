//! Async worker for downloading and installing `ReShade`.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use relm4::{ComponentSender, Worker};

use crate::reshade::game::{DllOverride, ExeArch};
use crate::reshade::services::ReShadeProvider;
use crate::reshade::{d3dcompiler, install, reshade};
use crate::ui::worker_types::ProgressEvent;

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
        /// The version string to download, e.g. `"6.1.0"`.
        version: String,
        /// Whether to download the Addon Support variant.
        addon: bool,
    },
}

/// Output signals from the install worker.
#[derive(Debug)]
pub enum Signal {
    /// A step completed — carries a typed progress event.
    Progress(ProgressEvent),
    /// Installation finished successfully.
    InstallComplete {
        /// The installed `ReShade` version string.
        version: String,
        /// DLL override that was installed.
        dll: DllOverride,
        /// Executable architecture that was targeted.
        arch: ExeArch,
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

/// Background install worker generic over a [`ReShadeProvider`].
pub struct InstallWorker<S: ReShadeProvider> {
    service: Arc<S>,
}

#[allow(missing_docs)]
impl<S: ReShadeProvider> Worker for InstallWorker<S> {
    type Init = S;
    type Input = Controls;
    type Output = Signal;

    fn init(service: S, _sender: ComponentSender<Self>) -> Self {
        Self {
            service: Arc::new(service),
        }
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
            Controls::DownloadVersion { version, addon } => {
                let service = Arc::clone(&self.service);
                let version_key = if addon { format!("{version}-Addon") } else { version.clone() };
                sender
                    .output(Signal::Progress(ProgressEvent::Downloading {
                        version: version_key.clone(),
                    }))
                    .ok();
                relm4::spawn(async move {
                    if let Err(e) = service.download_and_extract(&version, addon).await {
                        sender.output(Signal::DownloadVersionError(e.to_string())).ok();
                    } else {
                        sender.output(Signal::DownloadVersionComplete { version_key }).ok();
                    }
                });
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reshade::game::{DllOverride, ExeArch};

    #[test]
    fn install_complete_signal_carries_dll_and_arch() {
        let sig = Signal::InstallComplete {
            version: "6.3.0".to_owned(),
            dll: DllOverride::Dxgi,
            arch: ExeArch::X86_64,
        };
        assert!(matches!(
            sig,
            Signal::InstallComplete {
                dll: DllOverride::Dxgi,
                arch: ExeArch::X86_64,
                ..
            }
        ));
    }
}

fn do_install(
    data_dir: &std::path::Path,
    game_dir: &std::path::Path,
    dll: DllOverride,
    arch: ExeArch,
    version: &str,
    sender: &ComponentSender<InstallWorker<impl ReShadeProvider>>,
) -> anyhow::Result<()> {
    let version_dir = reshade::version_dir(data_dir, version);
    if !version_dir.join(arch.reshade_dll()).exists() {
        anyhow::bail!("ReShade {version} is not cached locally — download it in Preferences first");
    }

    if !d3dcompiler::is_installed(data_dir, arch) {
        sender.output(Signal::Progress(ProgressEvent::InstallingD3dcompiler)).ok();
    }
    d3dcompiler::ensure(data_dir, arch).context("Failed to install d3dcompiler_47.dll")?;

    sender.output(Signal::Progress(ProgressEvent::Installing)).ok();
    install::install_reshade(data_dir, game_dir, version, dll, arch)?;

    // cache.add_installed intentionally omitted: the version is already
    // registered in the cache from the Preferences download step.

    sender
        .output(Signal::InstallComplete {
            version: version.to_owned(),
            dll,
            arch,
        })
        .ok();
    Ok(())
}
