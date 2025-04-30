use crate::config::SshConfig;
use crate::error::{HttpError, WebError};
use crate::types::{Repo, TeamId};
use axum::http::StatusCode;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use shared::indent;
use snafu::{location, Location, Snafu};
use snafu::{Report, ResultExt};
use std::path::{Path, PathBuf};
use std::process::Output;
use sync::mpsc;
use tokio::process::Command;
use tokio::sync;
use tokio::sync::mpsc::Receiver;
use tracing::{info, warn};

#[derive(Debug, Snafu)]
pub enum GitError {
    #[snafu(display("Failed to clone repository for `{team}` at {location}"))]
    NotCloned {
        source: std::io::Error,
        team: TeamId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to check out revision `{revision}` for `{team}` at {location}"))]
    NotCheckedOut {
        source: std::io::Error,
        team: TeamId,
        revision: RevisionId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Failed to update submodules at revision `{revision}` for `{team}` at {location}"
    ))]
    SubmodulesNotUpdated {
        source: std::io::Error,
        team: TeamId,
        revision: RevisionId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Failed to clean repository at revision `{revision}` for `{team}` at {location}"
    ))]
    NotCleaned {
        source: std::io::Error,
        team: TeamId,
        revision: RevisionId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to create temporary directory at {location}"))]
    TempDirCreation {
        source: std::io::Error,
        team: TeamId,
        revision: RevisionId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Failed to tar repository at revision `{revision}` for `{team}` at {location}"
    ))]
    NotTared {
        source: std::io::Error,
        team: TeamId,
        revision: RevisionId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to update repository at `{path:?}` for `{team}` at {location}"))]
    NotUpdated {
        source: std::io::Error,
        team: TeamId,
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Failed to update repository url at `{path:?}` for `{team}` to `{url}` at {location}"
    ))]
    UrlNotChanged {
        source: std::io::Error,
        team: TeamId,
        path: PathBuf,
        url: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to fetch repository url at `{path:?}` for `{team}` at {location}"))]
    UrlNotFetched {
        source: std::io::Error,
        team: TeamId,
        path: PathBuf,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to send update request to updater for `{team}` at {location}"))]
    UpdaterSend {
        source: mpsc::error::SendError<RepoUpdateRequest>,
        team: TeamId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to wait for update result for `{team}` at {location}"))]
    UpdaterWait {
        source: sync::oneshot::error::RecvError,
        team: TeamId,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to look up commit revision `{revision}` at {location}"))]
    LookupCommitRev {
        source: std::io::Error,
        revision: String,
        #[snafu(implicit)]
        location: Location,
    },
}

impl HttpError for GitError {
    fn to_http_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_error_code(&self) -> &'static str {
        "git_error"
    }
}

impl From<GitError> for WebError {
    fn from(value: GitError) -> Self {
        warn!(error = ?Report::from_error(&value), "A git error occurred");

        Self::http_error(value, location!())
    }
}

#[derive(Debug, Clone, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct RevisionId(String);

#[derive(Debug, Clone)]
pub struct LocalRepos {
    path: PathBuf,
    updater: mpsc::Sender<RepoUpdateRequest>,
}

impl LocalRepos {
    pub fn new(path: PathBuf, ssh_config: Option<SshConfig>) -> Self {
        let (tx, rx) = mpsc::channel(100);

        // Start the repo updater in the background
        tokio::spawn(repo_updater(rx, ssh_config));

        Self { path, updater: tx }
    }

    pub async fn update_repo(&self, repo: &Repo) -> Result<(), GitError> {
        let path = self.get_repo_path(&repo.team);
        let (done_tx, done_rx) = sync::oneshot::channel();

        self.updater
            .send(RepoUpdateRequest::UpdateRepo {
                done: done_tx,
                repo: repo.clone(),
                path: path.clone(),
            })
            .await
            .context(UpdaterSendSnafu {
                team: repo.team.clone(),
            })?;

        done_rx.await.context(UpdaterWaitSnafu {
            team: repo.team.clone(),
        })?
    }

    pub async fn get_revision(
        &self,
        repo: &Repo,
        revision: &str,
    ) -> Result<Option<RevisionId>, GitError> {
        let path = self.get_repo_path(&repo.team);
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg("--end-of-options")
            .arg(format!("{revision}^{{commit}}"))
            .current_dir(&path)
            .output()
            .await
            .context(LookupCommitRevSnafu {
                revision: revision.to_string(),
            })?;

        if output.status.success() {
            Ok(Some(RevisionId(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            )))
        } else {
            Ok(None)
        }
    }

    pub async fn get_revision_message(
        &self,
        repo: &Repo,
        revision_id: &RevisionId,
    ) -> Result<String, GitError> {
        let path = self.get_repo_path(&repo.team);

        let output = Command::new("git")
            .arg("rev-list")
            .arg("--format=%s")
            .arg("--max-count=1")
            .arg(revision_id.to_string())
            .current_dir(&path)
            .handle_exitcode()
            .await
            .context(LookupCommitRevSnafu {
                revision: revision_id.to_string(),
            })?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .lines()
            .skip(1)
            .collect())
    }

    pub async fn export_repo(
        &self,
        repo: &Repo,
        target: &Path,
        revision: &RevisionId,
    ) -> Result<(), GitError> {
        let path = self.get_repo_path(&repo.team);

        let tempdir = tempfile::tempdir().context(TempDirCreationSnafu {
            team: repo.team.clone(),
            revision: revision.clone(),
        })?;

        Command::new("git")
            .arg("clone")
            .arg("--recursive")
            .arg("--recurse-submodules")
            .arg(&path)
            .arg(tempdir.path())
            .handle_exitcode()
            .await
            .context(NotClonedSnafu {
                team: repo.team.clone(),
            })?;

        Command::new("git")
            .arg("checkout")
            .arg(revision.to_string())
            .current_dir(tempdir.path())
            .handle_exitcode()
            .await
            .context(NotCheckedOutSnafu {
                team: repo.team.clone(),
                revision: revision.clone(),
            })?;

        Command::new("git")
            .arg("submodule")
            .arg("update")
            .arg("--force")
            .arg("--init")
            .arg("--recursive")
            .current_dir(tempdir.path())
            .handle_exitcode()
            .await
            .context(SubmodulesNotUpdatedSnafu {
                team: repo.team.clone(),
                revision: revision.clone(),
            })?;

        Command::new("git")
            .arg("clean")
            .arg("-fdx")
            .arg(revision.to_string())
            .current_dir(tempdir.path())
            .handle_exitcode()
            .await
            .context(NotCleanedSnafu {
                team: repo.team.clone(),
                revision: revision.clone(),
            })?;

        Command::new("tar")
            .arg("cfa")
            .arg(target)
            .arg(".")
            .current_dir(tempdir.path())
            .handle_exitcode()
            .await
            .context(NotTaredSnafu {
                team: repo.team.clone(),
                revision: revision.clone(),
            })?;

        // Make it explicit that we clean up the tempdir here
        drop(tempdir);

        Ok(())
    }

    fn get_repo_path(&self, team: &TeamId) -> PathBuf {
        self.path.join(team.to_string())
    }
}

async fn clone_mirror(repo: &Repo, path: &Path, ssh_key: Option<&String>) -> Result<(), GitError> {
    Command::new("git")
        .arg("clone")
        .arg("--mirror")
        .arg(&repo.url)
        .arg(path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .with_ssh_key(ssh_key)
        .handle_exitcode()
        .await
        .context(NotClonedSnafu {
            team: repo.team.clone(),
        })?;

    Ok(())
}

async fn update_mirror(repo: &Repo, path: &Path, ssh_key: Option<&String>) -> Result<(), GitError> {
    if !path.exists() {
        return clone_mirror(repo, path, ssh_key).await;
    }

    let current_url = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(path)
        .handle_exitcode()
        .await
        .context(UrlNotFetchedSnafu {
            team: repo.team.clone(),
            path: path.to_path_buf(),
        })?;

    let current_url = String::from_utf8_lossy(&current_url.stdout)
        .trim()
        .to_string();
    if current_url != repo.url {
        info!(
            team = %repo.team.clone(),
            path = %path.display(),
            current_url = %current_url,
            new_url = %repo.url,
            "Updating repository URL (nuking repo)",
        );
        Command::new("rm")
            .arg("-rf")
            .arg(path)
            .handle_exitcode()
            .await
            .context(UrlNotChangedSnafu {
                url: repo.url.clone(),
                team: repo.team.clone(),
                path: path.to_path_buf(),
            })?;
        return clone_mirror(repo, path, ssh_key).await;
    }

    Command::new("git")
        .arg("fetch")
        .arg("--all")
        .arg("--prune")
        .current_dir(path)
        .with_ssh_key(ssh_key)
        .handle_exitcode()
        .await
        .context(NotUpdatedSnafu {
            team: repo.team.clone(),
            path: path.to_path_buf(),
        })?;

    Ok(())
}

async fn repo_updater(mut rx: Receiver<RepoUpdateRequest>, ssh_config: Option<SshConfig>) {
    let team_to_key = ssh_config.map(|it| it.team_to_key).unwrap_or_default();

    while let Some(request) = rx.recv().await {
        match request {
            RepoUpdateRequest::UpdateRepo { repo, path, done } => {
                let ssh_key = team_to_key.get(&repo.team);
                let res = update_mirror(&repo, &path, ssh_key).await;
                if let Err(e) = done.send(res) {
                    warn!(
                        error = ?e,
                        team = %repo.team,
                        path = %path.display(),
                        "Failed to send update result back to caller"
                    );
                }
            }
        }
    }
}

pub enum RepoUpdateRequest {
    UpdateRepo {
        repo: Repo,
        path: PathBuf,
        done: sync::oneshot::Sender<Result<(), GitError>>,
    },
}

enum OptionalSsh<'a> {
    WithoutSsh(&'a mut Command),
    WithSsh {
        command: &'a mut Command,
        ssh_key: String,
    },
}

trait HandleExitcode {
    async fn handle_exitcode(self) -> std::io::Result<Output>;
}

impl HandleExitcode for OptionalSsh<'_> {
    async fn handle_exitcode(self) -> std::io::Result<Output> {
        match self {
            OptionalSsh::WithoutSsh(command) => command.handle_exitcode().await,
            OptionalSsh::WithSsh { command, ssh_key } => {
                let file = tempfile::NamedTempFile::new()?;
                tokio::fs::write(file.path(), ssh_key.as_bytes()).await?;
                tokio::fs::set_permissions(
                    file.path(),
                    std::os::unix::fs::PermissionsExt::from_mode(0o600),
                )
                .await?;

                command
                    .env(
                        "GIT_SSH_COMMAND",
                        format!(
                            "ssh -F /dev/null -o StrictHostKeyChecking=no -o UpdateHostKeys=no -i {}",
                            file.path().display()
                        ),
                    )
                    .handle_exitcode()
                    .await
            }
        }
    }
}

impl HandleExitcode for &mut Command {
    async fn handle_exitcode(self) -> std::io::Result<Output> {
        let output = self.output().await?;

        if output.status.success() {
            return Ok(output);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        let mut response = "".to_string();
        if let Some(code) = output.status.code() {
            response.push_str(&format!("Exited with code {code}\n"));
        }
        if !stdout.trim().is_empty() {
            response.push_str(&format!("stdout:\n{}\n", indent(stdout.trim(), 2)));
        }
        if !stderr.trim().is_empty() {
            response.push_str(&format!("stderr:\n{}", indent(stderr.trim(), 2)));
        }

        Err(std::io::Error::new(std::io::ErrorKind::Other, response))
    }
}

trait WithSsh<'a> {
    fn with_ssh_key(self, key: Option<&String>) -> OptionalSsh<'a>;
}

impl<'a> WithSsh<'a> for &'a mut Command {
    fn with_ssh_key(self, key: Option<&String>) -> OptionalSsh<'a> {
        if let Some(ssh_key) = key {
            OptionalSsh::WithSsh {
                command: self,
                ssh_key: ssh_key.clone(),
            }
        } else {
            OptionalSsh::WithoutSsh(self)
        }
    }
}
