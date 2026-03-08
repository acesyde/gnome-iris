//! Async worker for cloning and updating shader repositories.

use std::path::PathBuf;

use relm4::{ComponentSender, Worker};

use crate::reshade::config::ShaderRepo;
use crate::reshade::shaders;

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
    Progress(String),
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

/// Background shader sync worker.
pub struct ShaderWorker;

#[allow(missing_docs)]
impl Worker for ShaderWorker {
    type Init = ();
    type Input = Controls;
    type Output = Signal;

    fn init(_: (), _sender: ComponentSender<Self>) -> Self {
        Self
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
                        .output(Signal::Progress(format!(
                            "Syncing {}...",
                            repo.local_name
                        )))
                        .ok();
                    if let Err(e) = shaders::sync_repo(repo, &repos_dir) {
                        sender
                            .output(Signal::RepoError {
                                repo_name: repo.local_name.clone(),
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                if let Err(e) = shaders::rebuild_merged(&repos_dir, &disabled_repos) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                sender.output(Signal::Complete).ok();
            }
            Controls::SyncOne { repo, data_dir } => {
                let repos_dir = data_dir.join("ReShade_shaders");
                if let Err(e) = std::fs::create_dir_all(&repos_dir) {
                    sender.output(Signal::Error(e.to_string())).ok();
                    return;
                }
                sender
                    .output(Signal::Progress(format!(
                        "Syncing {}...",
                        repo.local_name
                    )))
                    .ok();
                match shaders::sync_repo(&repo, &repos_dir) {
                    Ok(()) => sender.output(Signal::Complete).ok(),
                    Err(e) => sender
                        .output(Signal::RepoError {
                            repo_name: repo.local_name.clone(),
                            error: e.to_string(),
                        })
                        .ok(),
                };
            }
        }
    }
}
