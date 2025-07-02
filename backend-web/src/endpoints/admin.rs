use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::grading_formulas::{GradingPoints, get_grading_points_for_task};
use crate::types::{
    AppState, FinishedCompilerTaskStatistics, FinishedCompilerTaskSummary, TaskId, TeamId, Test,
    TestId, WorkItem,
};
use axum::extract::{Path, State};
use serde::Serialize;
use shared::TestModifier;
use snafu::{Report, location};
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

#[instrument(skip_all)]
pub async fn snapshot_state(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<SnapshotResponse>> {
    info!(triggered_by = %claims.sub, "Snapshotting state");

    let res = match snapshot(&state).await {
        Ok(res) => res,
        Err(e) => {
            error!(
                error = %Report::from_error(&e),
                "Failed to create snapshot"
            );
            return Err(WebError::internal_error(
                Report::from_error(&e).to_string(),
                location!(),
            ));
        }
    };

    Ok(Json(res))
}

#[instrument(skip_all)]
async fn snapshot(state: &AppState) -> Result<SnapshotResponse> {
    let mut errors = Vec::new();
    let mut exported = Vec::new();

    let start_time = jiff::Timestamp::now();
    let tmp_export_folder = state
        .grading_config
        .snapshot_path
        .join(format!(".{start_time}"));

    tokio::fs::create_dir_all(&tmp_export_folder)
        .await
        .map_err(|e| WebError::internal_error(Report::from_error(&e).to_string(), location!()))?;

    let repo_folder = tmp_export_folder.join("repos");
    tokio::fs::create_dir_all(&repo_folder)
        .await
        .map_err(|e| WebError::internal_error(Report::from_error(&e).to_string(), location!()))?;

    for repo in state.db.get_repos().await? {
        let res = state
            .local_repos
            .snapshot_repo(&repo, &repo_folder.join(repo.team.to_string()))
            .await;
        if let Err(e) = res {
            errors.push(Report::from_error(&e).to_string());
            warn!(
                error = %Report::from_error(&e),
                team = %repo.team,
                url = %repo.url,
                "Failed to snapshot repo, but continuing"
            )
        } else {
            exported.push(repo.team);
        }
    }

    state
        .db
        .snapshot_db(&tmp_export_folder.join("crow.db"))
        .await?;

    let export_folder = state
        .grading_config
        .snapshot_path
        .join(format!("{start_time}"));
    tokio::fs::rename(&tmp_export_folder, &export_folder)
        .await
        .map_err(|e| WebError::internal_error(Report::from_error(&e).to_string(), location!()))?;

    Ok(SnapshotResponse { errors, exported })
}

#[instrument(skip_all)]
pub async fn rerun_submissions(
    State(state): State<AppState>,
    Path(category_name): Path<String>,
    claims: Claims,
) -> Result<Json<RerunResponse>> {
    info!(triggered_by = %claims.sub, "Rerunning submissions");
    let mut errors = Vec::new();
    let mut submitted = Vec::new();

    let category_meta = state
        .test_config
        .categories
        .get(&category_name)
        .ok_or_else(|| {
            WebError::named_not_found(format!("Category `{category_name}`"), location!())
        })?;

    let mut new_tasks = HashMap::new();

    for team in state.db.get_teams().await? {
        let final_task = state
            .db
            .get_final_submitted_task_for_team_and_category(
                &team.id,
                &category_name,
                category_meta,
                false, // we do not care, we want to recompute/rerun it
            )
            .await?;

        if let Some(task) = final_task {
            let revision = task.summary().info().revision_id.clone();
            new_tasks.insert(team.id, revision);
        } else {
            errors.push(format!(
                "No final task found for team `{}` in category `{}`",
                team.id, category_name
            ));
        }
    }

    for (team, revision) in new_tasks {
        info!(
            team = %team,
            revision = %revision,
            triggered_by = %claims.sub,
            "Rerunning submission for team"
        );

        let task_id = TaskId::from(Uuid::new_v4().to_string());
        state
            .db
            .queue_task(WorkItem {
                id: task_id.clone(),
                team: team.clone(),
                revision,
                commit_message: format!("Grading rerun for category `{category_name}`"),
                insert_time: SystemTime::now(),
            })
            .await?;
        state
            .db
            .finalize_submission(&team, &task_id, &category_name)
            .await?;

        submitted.push((team, task_id));
    }

    Ok(Json(RerunResponse { errors, submitted }))
}

#[instrument(skip_all)]
pub async fn rehash_tests(State(state): State<AppState>, claims: Claims) -> Result<()> {
    info!(triggered_by = %claims.sub, "Rehashing tests");

    state.db.rehash_tests().await
}

#[instrument(skip_all)]
pub async fn team_statistics(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TeamStatistics>>> {
    let mut entries = Vec::new();
    let tests: HashMap<TeamId, Vec<Test>> =
        state
            .db
            .get_tests()
            .await?
            .into_iter()
            .fold(HashMap::new(), |mut acc, test| {
                acc.entry(test.owner.clone()).or_default().push(test);
                acc
            });

    for team in state.db.get_teams().await? {
        let id = team.id;
        let mut entry = TeamStatistics {
            team: id.clone(),
            tests_per_category: HashMap::new(),
            finalized_tasks_per_category: HashMap::new(),
        };
        for test in tests.get(&id).unwrap_or(&Vec::new()) {
            entry.absorb(test);
        }
        for (category, meta) in &state.test_config.categories {
            let finalized_task = state.db.fetch_finalized_task_id(&id, category).await?;

            if let Some(finalized_task) = finalized_task {
                let summaries = state
                    .db
                    .get_finished_test_summaries(&finalized_task)
                    .await?;
                let summaries = state.test_config.get_counting_tests(category, &summaries);
                let points =
                    get_grading_points_for_task(&state.test_config, category, meta, &summaries)?;
                let finalized_task = FinalizedTask {
                    task_id: finalized_task,
                    statistics: FinishedCompilerTaskStatistics::from(summaries.as_slice()),
                };
                entry
                    .finalized_tasks_per_category
                    .insert(category.clone(), (finalized_task, points));
            }
        }

        entries.push(entry);
    }

    Ok(Json(entries))
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotResponse {
    pub errors: Vec<String>,
    pub exported: Vec<TeamId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RerunResponse {
    pub errors: Vec<String>,
    pub submitted: Vec<(TeamId, TaskId)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStatistics {
    pub team: TeamId,
    pub tests_per_category: HashMap<String, TestClassification>,
    pub finalized_tasks_per_category: HashMap<String, (FinalizedTask, Option<GradingPoints>)>,
}

impl TeamStatistics {
    pub fn absorb(&mut self, test: &Test) {
        let category = test.category.clone();
        let compiler_error = test
            .compiler_modifiers
            .iter()
            .any(|it| matches!(it, TestModifier::ShouldFail { .. }));
        let runtime_error = test
            .binary_modifiers
            .iter()
            .any(|it| matches!(it, TestModifier::ShouldCrash { .. }));
        let exit_code = test.binary_modifiers.iter().any(|it| {
            matches!(
                it,
                TestModifier::ExitCode { .. } | TestModifier::ShouldSucceed
            )
        });
        let non_termination = test
            .binary_modifiers
            .iter()
            .any(|it| matches!(it, TestModifier::ShouldTimeout));
        let compiler_succeed_no_exec = test
            .compiler_modifiers
            .iter()
            .any(|it| matches!(it, TestModifier::ShouldSucceed))
            && test.binary_modifiers.is_empty();

        let entry = self.tests_per_category.entry(category).or_default();
        if runtime_error {
            entry.runtime_error += 1;
        } else if compiler_error {
            entry.compile_error += 1;
        } else if exit_code {
            entry.exit_code += 1;
        } else if non_termination {
            entry.non_termination += 1;
        } else if compiler_succeed_no_exec {
            entry.compiler_succeed_no_exec += 1;
        } else {
            entry.unclassified.push(test.id.clone());
        }
    }
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestClassification {
    pub runtime_error: usize,
    pub compile_error: usize,
    pub exit_code: usize,
    pub non_termination: usize,
    pub compiler_succeed_no_exec: usize,
    pub unclassified: Vec<TestId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedTask {
    pub task_id: TaskId,
    pub statistics: FinishedCompilerTaskStatistics,
}

impl From<FinishedCompilerTaskSummary> for FinalizedTask {
    fn from(value: FinishedCompilerTaskSummary) -> Self {
        let task_id = value.info().task_id.clone().into();
        let statistics = match value {
            FinishedCompilerTaskSummary::BuildFailed { .. } => Default::default(),
            FinishedCompilerTaskSummary::RanTests { statistics, .. } => statistics,
        };

        Self {
            task_id,
            statistics,
        }
    }
}
