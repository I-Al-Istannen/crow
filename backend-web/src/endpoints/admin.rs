use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, TaskId, TeamId, WorkItem};
use axum::extract::{Path, State};
use serde::Serialize;
use snafu::{location, Report, ResultExt};
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::{error, info, warn};
use uuid::Uuid;

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

async fn snapshot(state: &AppState) -> Result<SnapshotResponse> {
    let mut errors = Vec::new();
    let mut exported = Vec::new();

    let start_time = jiff::Timestamp::now();
    let tmp_export_folder = state
        .grading_config
        .snapshot_path
        .join(format!(".{}", start_time));

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
        .join(format!("{}", start_time));
    tokio::fs::rename(&tmp_export_folder, &export_folder)
        .await
        .map_err(|e| WebError::internal_error(Report::from_error(&e).to_string(), location!()))?;

    Ok(SnapshotResponse { errors, exported })
}

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
            WebError::named_not_found(format!("Category `{}`", category_name), location!())
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
                commit_message: format!("Grading rerun for category `{}`", category_name),
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
