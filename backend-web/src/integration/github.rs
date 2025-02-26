use crate::config::GithubConfig;
use crate::error::WebError;
use crate::types::{
    AppState, CreatedExternalRun, ExternalRunId, ExternalRunStatus, QueuedTaskStatus, Repo, TaskId,
    TeamIntegrationToken,
};
use axum::http;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use crypto_box::PublicKey;
use jsonwebtoken::EncodingKey;
use octocrab::models::repos::secrets::CreateRepositorySecret;
use octocrab::models::{
    AppId, Installation, InstallationRepositories, InstallationToken, Repository,
};
use octocrab::params::apps::CreateInstallationAccessToken;
use octocrab::params::checks::{CheckRunConclusion, CheckRunStatus};
use octocrab::params::repos::Reference;
use octocrab::repos::RepoHandler;
use octocrab::Octocrab;
use rand::rngs::OsRng;
use snafu::{IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};
use url::Url;

#[derive(Debug, Snafu)]
pub enum GitHubError {
    #[snafu(display("Failed to parse expiry date at {location}"))]
    InvalidExpirationDate {
        date: String,
        source: time::error::Parse,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Octocrab error at {location}"))]
    Octocrab {
        source: octocrab::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("No app installation found for `{repo_full_name}` at {location}"))]
    NoAppInstallation {
        repo_full_name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error communicating with the rest of the backend at {location}"))]
    OurBackend {
        source: WebError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error sending to the event channel at {location}"))]
    SendToEventChannel {
        source: mpsc::error::SendError<EventForGithub>,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone)]
pub struct RepoFullName {
    owner: String,
    repo: String,
}

impl RepoFullName {
    pub fn from_url(url: &str) -> Option<Self> {
        parse_url_to_repo_owner(url).map(|(owner, repo)| Self { owner, repo })
    }
}

impl Display for RepoFullName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

fn parse_path_to_repo_owner(path: &str) -> Option<(String, String)> {
    let parts = path.split('/').take(2).collect::<Vec<_>>();
    if parts.len() != 2 {
        info!(path = %path, parts = ?parts, "Not correct path amount");
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}

// noinspection HttpUrlsUsage
fn parse_url_to_repo_owner(url: &str) -> Option<(String, String)> {
    let url = url.to_lowercase();
    if !url.contains("github.com") {
        return None;
    }

    if let Some(rest) = url.strip_prefix("git@github.com:") {
        return parse_path_to_repo_owner(rest);
    }

    let url = url.replace("http://", "").replace("https://", "");

    if let Some(rest) = url.strip_prefix("github.com/") {
        return parse_path_to_repo_owner(rest);
    }

    None
}

#[derive(Debug, Clone)]
pub struct CheckCreateData {
    pub repo_name: RepoFullName,
    pub commitish: String,
    pub task_id: TaskId,
    pub details_url: String,
}

#[derive(Debug, Clone)]
pub struct CheckStartData {
    pub repo_name: RepoFullName,
    pub check_run_id: u64,
}

#[derive(Debug, Clone)]
pub struct CheckFinishData {
    pub repo_name: RepoFullName,
    pub check_run_id: u64,
    pub conclusion: CheckRunConclusion,
}

#[derive(Debug, Clone)]
struct ClientCrab {
    installation: Installation,
    octocrab: Octocrab,
    master_crab: Octocrab,
    expires: OffsetDateTime,
}

impl ClientCrab {
    #[instrument(skip_all)]
    pub async fn new(
        installation: Installation,
        master_crab: Octocrab,
    ) -> Result<Self, GitHubError> {
        let (octocrab, expires) = Self::login(&installation, &master_crab).await?;

        Ok(Self {
            installation,
            octocrab,
            master_crab,
            expires,
        })
    }

    #[instrument(skip_all)]
    async fn login(
        installation: &Installation,
        master_crab: &Octocrab,
    ) -> Result<(Octocrab, OffsetDateTime), GitHubError> {
        let access_token_url =
            Url::parse(installation.access_tokens_url.as_ref().unwrap()).unwrap();
        let create_access_token = CreateInstallationAccessToken::default();
        let access: InstallationToken = master_crab
            .post(access_token_url.path(), Some(&create_access_token))
            .await
            .context(OctocrabSnafu)?;

        let octocrab = Octocrab::builder()
            .personal_token(access.token)
            .build()
            .context(OctocrabSnafu)?;

        let expires = access.expires_at.unwrap();
        let expires =
            OffsetDateTime::parse(&expires, &time::format_description::well_known::Rfc3339)
                .context(InvalidExpirationDateSnafu { date: expires })?;

        Ok((octocrab, expires))
    }

    #[instrument(skip_all)]
    async fn get_repositories(&mut self) -> Result<Vec<Repository>, GitHubError> {
        let installed_repos: InstallationRepositories = self
            .get_crab()
            .await?
            .get("/installation/repositories", None::<&()>)
            .await
            .context(OctocrabSnafu)?;
        Ok(installed_repos.repositories)
    }

    #[instrument(skip_all)]
    async fn create_check(&mut self, data: CheckCreateData) -> Result<u64, GitHubError> {
        let res = self
            .get_crab()
            .await?
            .checks(&data.repo_name.owner, &data.repo_name.repo)
            .create_check_run("crow", &data.commitish)
            .status(CheckRunStatus::Queued)
            .details_url(&data.details_url)
            .send()
            .await
            .context(OctocrabSnafu)?;

        Ok(*res.id)
    }

    #[instrument(skip_all)]
    async fn start_check(&mut self, data: CheckStartData) -> Result<(), GitHubError> {
        self.get_crab()
            .await?
            .checks(&data.repo_name.owner, &data.repo_name.repo)
            .update_check_run(data.check_run_id.into())
            .status(CheckRunStatus::InProgress)
            .send()
            .await
            .context(OctocrabSnafu)?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn finish_check(&mut self, data: CheckFinishData) -> Result<(), GitHubError> {
        self.get_crab()
            .await?
            .checks(&data.repo_name.owner, &data.repo_name.repo)
            .update_check_run(data.check_run_id.into())
            .status(CheckRunStatus::Completed)
            .conclusion(data.conclusion)
            .send()
            .await
            .context(OctocrabSnafu)?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn init_workflow(
        &mut self,
        repo_name: &RepoFullName,
        workflow_path: &str,
        workflow_template: &str,
        team_integration_token: &TeamIntegrationToken,
    ) -> Result<(), GitHubError> {
        let crab = self.get_crab().await?;
        let handler = crab.repos(&repo_name.owner, &repo_name.repo);

        let repo = handler.get().await.context(OctocrabSnafu)?;

        let Some(default_branch) = repo.default_branch else {
            return Ok(());
        };

        Self::create_integration_token_secret(&handler, repo_name, team_integration_token).await?;

        let (existing_sha, different) = Self::get_existing_sha_and_difference(
            &handler,
            workflow_path,
            workflow_template,
            &default_branch,
            repo_name,
        )
        .await?;

        // No need to do anything, we integrated into default branch
        if !different {
            debug!(
                owner = repo_name.owner,
                repo = repo_name.repo,
                workflow_file = workflow_path,
                "Workflow already exists and is identical, skipping setup"
            );
            return Ok(());
        }

        let (_, different) = Self::get_existing_sha_and_difference(
            &handler,
            workflow_path,
            workflow_template,
            "crow-init",
            repo_name,
        )
        .await?;

        // No need to do anything, our branch is already at the correct state
        if !different {
            debug!(
                owner = repo_name.owner,
                repo = repo_name.repo,
                workflow_file = workflow_path,
                "Workflow already exists in `crow-init` and is identical, skipping setup"
            );
            return Ok(());
        }

        info!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            existing_file = ?existing_sha,
            "Creating workflow"
        );

        // Delete potentially existing branch
        let _ = handler
            .delete_ref(&Reference::Branch("crow-init".to_string()))
            .await;

        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            "Deleted potentially existing branch"
        );

        let current_commit = crab
            .commits(&repo_name.owner, &repo_name.repo)
            .get(&default_branch)
            .await
            .context(OctocrabSnafu)?;

        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            current_commit = %current_commit.sha,
            "Got current commit"
        );

        handler
            .create_ref(
                &Reference::Branch("crow-init".to_string()),
                current_commit.sha,
            )
            .await
            .context(OctocrabSnafu)?;

        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            "Created branch"
        );

        let update_file_builder = match existing_sha {
            Some(sha) => handler.update_file(
                workflow_path,
                "Update crow workflow integration",
                workflow_template,
                &sha,
            ),
            None => handler.create_file(
                workflow_path,
                "Add crow workflow integration",
                workflow_template,
            ),
        };

        update_file_builder
            .branch("crow-init")
            .send()
            .await
            .context(OctocrabSnafu)?;

        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            "Created workflow file"
        );

        let pr = crab
            .pulls(&repo_name.owner, &repo_name.repo)
            .create(
                "Add crow workflow integration",
                "crow-init",
                &default_branch,
            )
            .send()
            .await
            .context(OctocrabSnafu)?;

        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            workflow_file = workflow_path,
            url = pr
                .html_url
                .map(|it| it.to_string())
                .unwrap_or("unknown url?".to_string()),
            "Created pull request"
        );

        Ok(())
    }

    #[instrument(skip_all)]
    async fn get_existing_sha_and_difference(
        handler: &RepoHandler<'_>,
        workflow_path: &str,
        workflow_template: &str,
        reference: &str,
        repo_name: &RepoFullName,
    ) -> Result<(Option<String>, bool), GitHubError> {
        let existing = handler
            .get_content()
            .path(workflow_path)
            .r#ref(reference)
            .send()
            .await;
        let existing = match existing {
            Ok(mut it) => it.take_items(),
            Err(octocrab::Error::GitHub {
                source:
                    octocrab::GitHubError {
                        status_code: http::StatusCode::NOT_FOUND,
                        ..
                    },
                ..
            }) => vec![],
            Err(e) => return Err(OctocrabSnafu.into_error(e)),
        };

        // Verify we actually need to change anything
        if !existing.is_empty() {
            debug!(
                owner = repo_name.owner,
                repo = repo_name.repo,
                workflow_file = workflow_path,
                reference = reference,
                "Workflow file already exists"
            );
            let existing = &existing[0];

            let existing_content = existing
                .content
                .as_ref()
                .map(|it| it.to_string())
                .unwrap_or_default();
            let existing_content = B64.decode(existing_content.trim()).unwrap();
            let existing_content = String::from_utf8_lossy(&existing_content);

            if existing_content == workflow_template {
                return Ok((None, false));
            }

            return Ok((Some(existing.sha.clone()), true));
        }
        Ok((None, true))
    }

    #[instrument(skip_all)]
    async fn create_integration_token_secret(
        handler: &RepoHandler<'_>,
        repo_name: &RepoFullName,
        team_integration_token: &TeamIntegrationToken,
    ) -> Result<(), GitHubError> {
        debug!(
            owner = repo_name.owner,
            repo = repo_name.repo,
            "Creating or updating integration token secret"
        );
        let secrets_handler = handler.secrets();
        let public_key = secrets_handler
            .get_public_key()
            .await
            .context(OctocrabSnafu)?;

        let crypto_pk = {
            let pk_bytes = B64.decode(public_key.key).unwrap();
            let pk_array: [u8; crypto_box::KEY_SIZE] = pk_bytes.try_into().unwrap();
            PublicKey::from(pk_array)
        };
        let encrypted_value = crypto_pk
            .seal(&mut OsRng, team_integration_token.to_string().as_bytes())
            .unwrap();

        secrets_handler
            .create_or_update_secret(
                "CROW_INTEGRATION_TOKEN",
                &CreateRepositorySecret {
                    encrypted_value: &B64.encode(&encrypted_value),
                    key_id: &public_key.key_id,
                },
            )
            .await
            .context(OctocrabSnafu)?;

        Ok(())
    }

    async fn get_crab(&mut self) -> Result<&Octocrab, GitHubError> {
        if OffsetDateTime::now_utc() > self.expires {
            let (crab, expires) = Self::login(&self.installation, &self.master_crab).await?;
            self.expires = expires;
            self.octocrab = crab;
        }
        Ok(&self.octocrab)
    }
}

struct GitHub {
    // Bearer of the one ring
    master_crab: Octocrab,
    crabs: HashMap<String, ClientCrab>,
}

impl GitHub {
    pub async fn new(config: GithubConfig) -> Result<Self, GitHubError> {
        let crab = Octocrab::builder()
            .app(
                AppId::from(config.app_id),
                EncodingKey::from_rsa_pem(config.app_private_key.as_bytes()).unwrap(),
            )
            .build()
            .context(OctocrabSnafu)?;

        Ok(Self {
            crabs: HashMap::new(),
            master_crab: crab,
        })
    }

    #[instrument(skip_all)]
    pub async fn create_check(
        &mut self,
        data: CheckCreateData,
    ) -> Result<ExternalRunId, GitHubError> {
        self.get_client_crab(&data.repo_name)
            .await?
            .create_check(data)
            .await
            .map(|it| it.into())
    }

    #[instrument(skip_all)]
    pub async fn start_check(&mut self, data: CheckStartData) -> Result<(), GitHubError> {
        self.get_client_crab(&data.repo_name)
            .await?
            .start_check(data)
            .await
    }

    #[instrument(skip_all)]
    pub async fn finish_check(&mut self, data: CheckFinishData) -> Result<(), GitHubError> {
        self.get_client_crab(&data.repo_name)
            .await?
            .finish_check(data)
            .await
    }

    #[instrument(skip_all)]
    pub async fn init_workflow(
        &mut self,
        repo_name: &RepoFullName,
        workflow_path: &str,
        workflow_template: &str,
        team_integration_token: &TeamIntegrationToken,
    ) -> Result<(), GitHubError> {
        self.get_client_crab(repo_name)
            .await?
            .init_workflow(
                repo_name,
                workflow_path,
                workflow_template,
                team_integration_token,
            )
            .await
    }

    #[instrument(skip_all)]
    async fn get_client_crab(
        &mut self,
        repo_full_name: &RepoFullName,
    ) -> Result<ClientCrab, GitHubError> {
        let key = repo_full_name.to_string().to_lowercase();
        if let Some(crab) = self.crabs.get(&key) {
            return Ok(crab.clone());
        }

        let installations = self
            .master_crab
            .apps()
            .installations()
            .send()
            .await
            .context(OctocrabSnafu)?
            .take_items();

        for installation in installations {
            let mut client = ClientCrab::new(installation, self.master_crab.clone()).await?;
            for repo in client.get_repositories().await? {
                self.crabs.insert(
                    RepoFullName {
                        owner: repo.owner.unwrap().login,
                        repo: repo.name,
                    }
                    .to_string()
                    .to_lowercase(),
                    client.clone(),
                );
            }
        }

        if let Some(crab) = self.crabs.get(&key) {
            return Ok(crab.clone());
        }

        Err(NoAppInstallationSnafu {
            repo_full_name: key,
        }
        .into_error(NoneError))
    }
}

#[derive(Debug, Clone)]
pub enum EventForGithub {
    Queued(CheckCreateData),
    Running(CheckStartData),
    Finished {
        repo_name: RepoFullName,
        check_run_id: u64,
        task_id: TaskId,
    },
}

#[instrument(skip_all)]
async fn create_check(
    github: &mut GitHub,
    state: &AppState,
    data: CheckCreateData,
) -> Result<(), GitHubError> {
    debug!(task_id = %data.task_id, "Creating check");
    let run_id = match github.create_check(data.clone()).await {
        Ok(res) => res,
        Err(e) => return Err(e),
    };

    debug!(task_id = %data.task_id, check_run_id = %run_id, "Check created!");

    state
        .db
        .add_external_run(&CreatedExternalRun {
            run_id,
            task_id: data.task_id.clone(),
            owner: data.repo_name.owner,
            repo: data.repo_name.repo,
            platform: "github".to_string(),
            status: ExternalRunStatus::Queued,
            revision: data.commitish,
        })
        .await
        .context(OurBackendSnafu)?;

    debug!(task_id = %data.task_id, check_run_id = %run_id, "External run created in db");

    Ok(())
}

#[instrument(skip_all)]
async fn start_check(
    github: &mut GitHub,
    state: &AppState,
    data: CheckStartData,
) -> Result<(), GitHubError> {
    debug!(check_run_id = %data.check_run_id, "Starting check");
    github.start_check(data.clone()).await?;
    debug!(check_run_id = %data.check_run_id, "Check started!");

    state
        .db
        .update_external_run_status(&data.check_run_id.into(), ExternalRunStatus::Running)
        .await
        .context(OurBackendSnafu)?;

    debug!(check_run_id = %data.check_run_id, "Status updated in db");

    Ok(())
}

#[instrument(skip_all)]
async fn finish_check(
    github: &mut GitHub,
    state: &AppState,
    repo_name: RepoFullName,
    check_run_id: u64,
    task_id: TaskId,
) -> Result<(), GitHubError> {
    debug!(task_id = %task_id, check_run_id = %check_run_id, "Finishing check");

    let task = state.db.get_task(&task_id).await;
    let conclusion = match task {
        // Task is not in queue but also not finished?
        Err(WebError::NotFound { .. }) => CheckRunConclusion::Stale,
        Err(e) => return Err(OurBackendSnafu.into_error(e)),
        Ok(task) => {
            let status: QueuedTaskStatus = task.into();
            match status {
                QueuedTaskStatus::Error => CheckRunConclusion::Failure,
                QueuedTaskStatus::Timeout => CheckRunConclusion::TimedOut,
                QueuedTaskStatus::Aborted => CheckRunConclusion::Cancelled,
                QueuedTaskStatus::Success => CheckRunConclusion::Success,
                _ => CheckRunConclusion::Neutral,
            }
        }
    };

    debug!(
        task_id = %task_id,
        check_run_id = %check_run_id,
        conclusion = ?conclusion,
        "Finishing check"
    );

    github
        .finish_check(CheckFinishData {
            repo_name,
            check_run_id,
            conclusion,
        })
        .await?;

    debug!(
        task_id = %task_id,
        check_run_id = %check_run_id,
        conclusion = ?conclusion,
        "Finished check"
    );

    state
        .db
        .delete_external_run(&check_run_id.into())
        .await
        .context(OurBackendSnafu)?;

    debug!(
        task_id = %task_id,
        check_run_id = %check_run_id,
        conclusion = ?conclusion,
        "Deleted external run from db"
    );

    Ok(())
}

#[instrument(skip_all)]
async fn drain_github_events(
    github: Arc<tokio::sync::Mutex<GitHub>>,
    state: AppState,
    mut rx: mpsc::Receiver<EventForGithub>,
) {
    while let Some(event) = rx.recv().await {
        debug!(event = ?event, "Processing event");
        let github = &mut github.lock().await;
        let res = match event.clone() {
            EventForGithub::Queued(data) => create_check(github, &state, data).await,
            EventForGithub::Running(data) => start_check(github, &state, data).await,
            EventForGithub::Finished {
                repo_name,
                check_run_id,
                task_id,
            } => finish_check(github, &state, repo_name, check_run_id, task_id).await,
        };

        if let Err(e) = res {
            info!(error = %Report::from_error(e), event = ?event, "Failed to process event");
        }
    }
}

#[instrument(skip_all)]
async fn github_iteration(
    tx: &mpsc::Sender<EventForGithub>,
    state: &AppState,
) -> Result<(), GitHubError> {
    let repos = state
        .db
        .get_repos()
        .await
        .context(OurBackendSnafu)?
        .into_iter()
        .flat_map(|repo| RepoFullName::from_url(&repo.url).map(|name| (repo.team, name)))
        .collect::<HashMap<_, _>>();

    let existing_runs = state
        .db
        .get_external_runs("github")
        .await
        .context(OurBackendSnafu)?
        .into_iter()
        .map(|run| (run.task_id.clone(), run))
        .collect::<HashMap<_, _>>();

    let queued_tasks = state
        .db
        .get_queued_tasks()
        .await
        .context(OurBackendSnafu)?
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect::<HashMap<_, _>>();

    for existing_run in existing_runs.values() {
        let repo_full_name = RepoFullName {
            owner: existing_run.owner.clone(),
            repo: existing_run.repo.clone(),
        };
        if !queued_tasks.contains_key(&existing_run.task_id) {
            debug!(task_id = %existing_run.task_id, "Task no longer queued");
            // Transition: Run is now done
            tx.send(EventForGithub::Finished {
                repo_name: repo_full_name,
                check_run_id: *existing_run.run_id,
                task_id: existing_run.task_id.clone(),
            })
            .await
            .context(SendToEventChannelSnafu)?;

            continue;
        }
        if existing_run.status == ExternalRunStatus::Running {
            debug!(task_id = %existing_run.task_id, "Task already running");
            // Run is still running
            continue;
        }

        if state
            .executor
            .lock()
            .unwrap()
            .get_running_task(&existing_run.task_id)
            .is_none()
        {
            debug!(task_id = %existing_run.task_id, "Task still queued");
            // Run is still queued
            continue;
        }

        debug!(task_id = %existing_run.task_id, "Task is now running");
        // Transition: Run is now running and was queued before!
        tx.send(EventForGithub::Running(CheckStartData {
            repo_name: repo_full_name,
            check_run_id: *existing_run.run_id,
        }))
        .await
        .context(SendToEventChannelSnafu)?;
    }

    for (task_id, work) in queued_tasks {
        if existing_runs.contains_key(&task_id) {
            debug!(task_id = %task_id, "Task already has a run");
            // Already known externally
            continue;
        }
        let Some(repo_name) = repos.get(&work.team) else {
            // We have no idea where their repo is (at least not with us)
            debug!(task_id = %task_id, "No repo found for team");
            continue;
        };

        debug!(task_id = %task_id, repo_name = ?repo_name, "Creating check for task");

        // Transition: New queued run
        tx.send(EventForGithub::Queued(CheckCreateData {
            repo_name: repo_name.clone(),
            commitish: work.revision.clone(),
            task_id: task_id.clone(),
            details_url: format!("http://localhost:5173/task-detail/{}", task_id),
        }))
        .await
        .context(SendToEventChannelSnafu)?;
    }

    Ok(())
}

#[instrument(skip_all)]
async fn init_workflow_single_repo(
    state: &AppState,
    github: &mut GitHub,
    repo: &Repo,
    repo_name: &RepoFullName,
    config: &GithubConfig,
) -> Result<(), GitHubError> {
    let integration_token = state
        .db
        .get_team_integration_token(&repo.team)
        .await
        .context(OurBackendSnafu)?;
    github
        .init_workflow(
            repo_name,
            &config.workflow_path,
            &config.workflow_template,
            &integration_token,
        )
        .await?;
    Ok(())
}

#[instrument(skip_all)]
async fn init_workflow_iteration(
    config: &GithubConfig,
    github: Arc<tokio::sync::Mutex<GitHub>>,
    state: &AppState,
) -> Result<(), GitHubError> {
    for repo in state.db.get_repos().await.context(OurBackendSnafu)? {
        debug!(repo = ?repo, "Checking workflow integration");
        if let Some(repo_name) = RepoFullName::from_url(&repo.url) {
            debug!(repo = ?repo_name, "Updating workflow");

            let mut github = github.lock().await;
            if let Err(e) =
                init_workflow_single_repo(state, &mut github, &repo, &repo_name, config).await
            {
                warn!(
                    error = %Report::from_error(e),
                    repo_name = %repo_name,
                    team_id = %repo.team,
                    "Failed to update workflow integration"
                );
            }
            drop(github);
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn update_workflow_task(
    github: Arc<tokio::sync::Mutex<GitHub>>,
    config: GithubConfig,
    state: AppState,
) {
    loop {
        info!("Checking for workflow updates");

        let res = init_workflow_iteration(&config, github.clone(), &state).await;
        if let Err(e) = res {
            warn!(error = %Report::from_error(e), "Failed to check for workflow updates");
        }
        info!("Finished checking for workflow updates");
        tokio::time::sleep(config.workflow_check_interval).await;
    }
}

#[instrument(skip_all)]
pub async fn run_github_app(config: GithubConfig, state: AppState) -> Result<(), GitHubError> {
    info!("Starting github app integration");
    let github = GitHub::new(config.clone()).await?;
    let github = Arc::new(tokio::sync::Mutex::new(github));

    let (tx, rx) = mpsc::channel(10);

    spawn(drain_github_events(github.clone(), state.clone(), rx));
    spawn(update_workflow_task(
        github.clone(),
        config.clone(),
        state.clone(),
    ));

    loop {
        debug!("Starting github status iteration");

        let res = github_iteration(&tx, &state).await;
        if let Err(e) = res {
            warn!(error = %Report::from_error(e), "Failed to perform github status iteration");
        }

        tokio::time::sleep(config.status_check_interval).await;
    }
}
