use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary};
use axum::extract::{Path, State};
use serde::Deserialize;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn list_tests(
    State(AppState { db, .. }): State<AppState>,
    _claims: Claims,
) -> Result<Json<Vec<TestSummary>>> {
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
    Ok(Json(test))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTestPayload {
    pub name: String,
    pub expected_output: String,
}
