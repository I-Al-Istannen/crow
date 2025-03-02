use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary};
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use snafu::location;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn list_tests(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<ListTestsResponse>> {
    Ok(Json(ListTestsResponse {
        tests: state.db.get_test_summaries().await?,
        categories: state.test_config.categories,
    }))
}

#[instrument(skip_all)]
pub async fn set_test(
    State(state): State<AppState>,
    claims: Claims,
    Path(test_id): Path<TestId>,
    Json(payload): Json<AddTestPayload>,
) -> Result<Json<Test>> {
    // TODO: Validate test id
    let db = &state.db;

    if !claims.is_admin() {
        if let Some(existing) = db.fetch_test(&test_id).await? {
            if existing.owner != claims.team {
                return Err(WebError::unauthorized(location!()));
            }
        }
    }

    if !state.test_config.categories.contains(&payload.category) {
        return Err(WebError::named_not_found(payload.category, location!()));
    }

    Ok(Json(
        db.add_test(Test {
            id: test_id,
            expected_output: payload.expected_output,
            input: payload.input,
            owner: claims.team.clone(),
            admin_authored: claims.is_admin(),
            category: payload.category,
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
        return Err(WebError::not_found(location!()));
    };
    Ok(Json(test))
}

#[instrument(skip_all)]
pub async fn delete_test(
    State(AppState { db, .. }): State<AppState>,
    claims: Claims,
    Path(test_id): Path<TestId>,
) -> Result<()> {
    if !claims.is_admin() {
        let Some(test) = db.fetch_test(&test_id).await? else {
            return Err(WebError::not_found(location!()));
        };

        if !claims.is_admin() && test.owner != claims.team {
            return Err(WebError::unauthorized(location!()));
        }
    }

    db.delete_test(&test_id).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTestPayload {
    pub expected_output: String,
    pub input: String,
    pub category: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTestsResponse {
    pub tests: Vec<TestSummary>,
    pub categories: Vec<String>,
}
