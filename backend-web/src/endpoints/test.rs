use crate::auth::Claims;
use crate::endpoints::Json;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary};
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use shared::{ExecutionOutput, FinishedTest};
use snafu::location;
use tracing::{debug, instrument};

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
) -> Result<Json<SetTestResponse>> {
    // TODO: Validate test id format
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

    let test = Test {
        id: test_id,
        expected_output: payload.expected_output,
        input: payload.input,
        owner: claims.team.clone(),
        admin_authored: claims.is_admin(),
        category: payload.category,
    };

    // Let the reference compiler taste it first
    if !state.execution_config.tasting_disabled() && !payload.ignore_test_tasting {
        let taste_result = state.test_tasting.lock().unwrap().add_tasting(test.clone());
        let taste_result = match taste_result.await {
            Ok(output) => output,
            Err(_) => {
                return Err(WebError::internal_error(
                    "No test result received".to_string(),
                    location!(),
                ))
            }
        };

        if !matches!(taste_result, ExecutionOutput::Success(_)) {
            return Ok(Json(SetTestResponse::TastingFailed(FinishedTest {
                test_id: test.id.to_string(),
                output: taste_result,
            })));
        }
    } else {
        debug!(test_id = %test.id, "Tasting disabled, skipping it for");
    }

    Ok(Json(SetTestResponse::TestAdded(db.add_test(test).await?)))
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
    pub ignore_test_tasting: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTestsResponse {
    pub tests: Vec<TestSummary>,
    pub categories: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum SetTestResponse {
    TestAdded(Test),
    TastingFailed(FinishedTest),
}
