use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary};
use axum::extract::{Path, State};
use serde::Deserialize;
use std::time::Duration;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn list_tests(
    State(AppState { db, .. }): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TestSummary>>> {
    // sleep 1 sec
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(Json(db.get_test_summaries().await?))
}

#[instrument(skip_all)]
pub async fn set_test(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(test_id): Path<TestId>,
    Json(payload): Json<AddTestPayload>,
) -> Result<Json<Test>> {
    let Some(team) = db.get_user(&claims.sub).await?.user.team else {
        return Err(WebError::NotInTeam);
    };

    if !claims.is_admin() {
        if let Some(existing) = db.fetch_test(&test_id).await? {
            if existing.owner != team {
                return Err(WebError::NoPermissions);
            }
        }
    }

    Ok(Json(
        db.add_test(Test {
            id: test_id,
            name: payload.name,
            expected_output: payload.expected_output,
            owner: team,
        })
        .await?,
    ))
}

#[instrument(skip_all)]
pub async fn get_test(
    State(AppState { db, .. }): State<AppState>,
    _claims: Claims,
    Path(test_id): Path<TestId>,
) -> Result<Json<Test>> {
    let Some(test) = db.fetch_test(&test_id).await? else {
        return Err(WebError::NotFound);
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(Json(test))
}

#[instrument(skip_all)]
pub async fn delete_test(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(test_id): Path<TestId>,
) -> Result<()> {
    let Some(team) = db.get_user(&claims.sub).await?.user.team else {
        return Err(WebError::NotInTeam);
    };

    if !claims.is_admin() {
        let Some(test) = db.fetch_test(&test_id).await? else {
            return Err(WebError::NotFound);
        };

        if !claims.is_admin() && test.owner != team {
            return Err(WebError::NoPermissions);
        }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    db.delete_test(&test_id).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTestPayload {
    pub name: String,
    pub expected_output: String,
}
