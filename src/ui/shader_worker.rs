//! Async worker for cloning and updating shader repositories.

use std::path::PathBuf;

use relm4::{ComponentSender, Worker};

use crate::reshade::config::ShaderRepo;
use crate::reshade::services::ShaderSyncService;
use crate::ui::worker_types::ProgressEvent;

/// Input commands for the shader worker.
#[derive(Debug)]
pub enum Controls {
    /// Clone/update all given repos then rebuild the Merged directory.
    SyncAll {
        /// Repos to sync (in priority order).
        repos: Vec<ShaderRepo>,
        /// Base data directory containing `ReShade_shaders/`.
        data_dir: PathBuf,
        /// Repo `local_name`s to exclude from the merge.
        disabled_repos: Vec<String>,
    },
    /// Clone/update a single repo without rebuilding the Merged directory.
    SyncOne {
        /// Repo to sync.
        repo: ShaderRepo,
        /// Base data directory containing `ReShade_shaders/`.
        data_dir: PathBuf,
    },
}

/// Output signals from the shader worker.
#[derive(Debug)]
pub enum Signal {
    /// Currently syncing this repo.
    Progress(ProgressEvent),
    /// All repos synced and merged successfully.
    Complete,
    /// A non-fatal error on one repo (sync continues with remaining repos).
    RepoError {
        /// The repo's `local_name`.
        repo_name: String,
        /// Error message.
        error: String,
    },
    /// A fatal error that stopped all syncing.
    Error(String),
}

/// Background shader sync worker generic over a [`ShaderSyncService`].
pub struct ShaderWorker<S: ShaderSyncService> {
    service: S,
}

#[allow(missing_docs)]
impl<S: ShaderSyncService> Worker for ShaderWorker<S> {
    type Init = S;
    type Input = Controls;
    type Output = Signal;

    fn init(service: S, _sender: ComponentSender<Self>) -> Self {
        Self { service }
    }

    fn update(&mut self, msg: Controls, sender: ComponentSender<Self>) {
        match msg {
            Controls::SyncAll {
                repos,
                data_dir,
                disabled_repos,
            } => {
                let repos_dir = data_dir.join("ReShade_shaders");
                if let Err(e) = std::fs::create_dir_all(&repos_dir) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                for repo in &repos {
                    sender
                        .output(Signal::Progress(ProgressEvent::SyncingRepo {
                            name: repo.local_name.clone(),
                        }))
                        .ok();
                    if let Err(e) = self.service.sync_repo(repo, &repos_dir) {
                        sender
                            .output(Signal::RepoError {
                                repo_name: repo.local_name.clone(),
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                if let Err(e) = self.service.rebuild_merged(&repos_dir, &disabled_repos) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                sender.output(Signal::Complete).ok();
            },
            Controls::SyncOne { repo, data_dir } => {
                let repos_dir = data_dir.join("ReShade_shaders");
                if let Err(e) = std::fs::create_dir_all(&repos_dir) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                sender
                    .output(Signal::Progress(ProgressEvent::SyncingRepo {
                        name: repo.local_name.clone(),
                    }))
                    .ok();
                match self.service.sync_repo(&repo, &repos_dir) {
                    Ok(()) => sender.output(Signal::Complete).ok(),
                    Err(e) => sender
                        .output(Signal::RepoError {
                            repo_name: repo.local_name.clone(),
                            error: e.to_string(),
                        })
                        .ok(),
                };
            },
        }
    }
}
