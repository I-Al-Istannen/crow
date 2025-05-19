use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, TeamId};
use axum::extract::State;
use serde::Serialize;
use snafu::{location, Report};
use tracing::{error, info, warn};

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

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotResponse {
    pub errors: Vec<String>,
    pub exported: Vec<TeamId>,
}
