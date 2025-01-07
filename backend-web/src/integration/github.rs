use crate::config::GithubConfig;
use crate::error::WebError;
use crate::types::{
    AppState, CreatedExternalRun, ExternalRunId, ExternalRunStatus, QueuedTaskStatus, TaskId,
};
use jsonwebtoken::EncodingKey;
use octocrab::models::{
    AppId, Installation, InstallationRepositories, InstallationToken, Repository,
};
use octocrab::params::apps::CreateInstallationAccessToken;
use octocrab::params::checks::{CheckRunConclusion, CheckRunStatus};
use octocrab::Octocrab;
use snafu::{IntoError, Location, NoneError, Report, ResultExt, Snafu};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, info};
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
        parse_url_to_repo_owner(url).map(|(owner, repo)| RepoFullName { owner, repo })
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

    async fn get_repositories(&mut self) -> Result<Vec<Repository>, GitHubError> {
        let installed_repos: InstallationRepositories = self
            .get_crab()
            .await?
            .get("/installation/repositories", None::<&()>)
            .await
            .context(OctocrabSnafu)?;
        Ok(installed_repos.repositories)
    }

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

    pub async fn start_check(&mut self, data: CheckStartData) -> Result<(), GitHubError> {
        self.get_client_crab(&data.repo_name)
            .await?
            .start_check(data)
            .await
    }

    pub async fn finish_check(&mut self, data: CheckFinishData) -> Result<(), GitHubError> {
        self.get_client_crab(&data.repo_name)
            .await?
            .finish_check(data)
            .await
    }

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
        Err(WebError::NotFound) => CheckRunConclusion::Stale,
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

async fn drain_github_events(
    mut github: GitHub,
    state: AppState,
    mut rx: mpsc::Receiver<EventForGithub>,
) {
    while let Some(event) = rx.recv().await {
        debug!(event = ?event, "Processing event");

        let res = match event.clone() {
            EventForGithub::Queued(data) => create_check(&mut github, &state, data).await,
            EventForGithub::Running(data) => start_check(&mut github, &state, data).await,
            EventForGithub::Finished {
                repo_name,
                check_run_id,
                task_id,
            } => finish_check(&mut github, &state, repo_name, check_run_id, task_id).await,
        };

        if let Err(e) = res {
            info!(error = %Report::from_error(e), event = ?event, "Failed to process event");
        }
    }
}

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

pub async fn run_github_app(config: GithubConfig, state: AppState) -> Result<(), GitHubError> {
    let check_interval = config.check_interval;
    let github = GitHub::new(config).await?;

    let (tx, rx) = mpsc::channel(10);

    spawn(drain_github_events(github, state.clone(), rx));

    loop {
        github_iteration(&tx, &state).await?;

        tokio::time::sleep(check_interval).await;
    }
}
