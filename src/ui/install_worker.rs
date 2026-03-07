//! Async worker for downloading and installing ReShade.

use std::path::PathBuf;

use relm4::{ComponentSender, Worker};

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
    /// An error occurred.
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
        }
    }
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

    sender
        .output(Signal::InstallComplete { version })
        .ok();
    Ok(())
}
