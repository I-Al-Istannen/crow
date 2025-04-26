use super::{Json, Path};
use crate::auth::Claims;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary, TestWithTasteTesting};
use axum::extract::State;
use jiff::Zoned;
use serde::{Deserialize, Serialize};
use shared::{TestExecutionOutput, TestModifier};
use snafu::location;
use std::collections::HashMap;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn list_tests(
    State(state): State<AppState>,
    _claims: Claims,
) -> Result<Json<ListTestsResponse>> {
    Ok(Json(ListTestsResponse {
        tests: state.db.get_test_summaries().await?,
        categories: state
            .test_config
            .categories
            .into_iter()
            .map(|(name, category)| (name, category.into()))
            .collect(),
    }))
}

#[instrument(skip_all)]
pub async fn set_test(
    State(state): State<AppState>,
    claims: Claims,
    Path(test_id): Path<TestId>,
    Json(payload): Json<AddTestPayload>,
) -> Result<Json<SetTestResponse>> {
    let db = &state.db;

    if !claims.is_admin() {
        if let Some(existing) = db.fetch_test(&test_id).await? {
            if existing.owner != claims.team {
                return Err(WebError::unauthorized(location!()));
            }
        }
    }

    if !state.test_config.categories.contains_key(&payload.category) {
        return Err(WebError::named_not_found(payload.category, location!()));
    }

    let test = Test {
        id: test_id,
        owner: claims.team.clone(),
        admin_authored: claims.is_admin(),
        category: payload.category,
        compiler_modifiers: payload.compiler_modifiers,
        binary_modifiers: payload.binary_modifiers,
    };

    // Let the reference compiler taste it first
    let taste_testing_result = do_test_tasting(&state, &test).await?;

    if let Some(result) = &taste_testing_result {
        if !matches!(result, TestExecutionOutput::Success { .. }) && !payload.ignore_test_tasting {
            return Ok(Json(SetTestResponse::TastingFailed {
                output: result.clone(),
            }));
        }
    }

    Ok(Json(SetTestResponse::TestAdded(
        db.add_test(test, taste_testing_result).await?,
    )))
}

async fn do_test_tasting(state: &AppState, test: &Test) -> Result<Option<TestExecutionOutput>> {
    if !state.execution_config.tasting_disabled() {
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

        return Ok(Some(taste_result));
    } else {
        debug!(test_id = %test.id, "Tasting disabled, skipping it for");
    }
    Ok(None)
}

#[instrument(skip_all)]
pub async fn get_test(
    State(AppState { db, .. }): State<AppState>,
    _claims: Claims,
    Path(test_id): Path<TestId>,
) -> Result<Json<TestWithTasteTesting>> {
    let Some(test) = db.fetch_test_with_tasting(&test_id).await? else {
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
    pub compiler_modifiers: Vec<TestModifier>,
    pub binary_modifiers: Vec<TestModifier>,
    pub category: String,
    pub ignore_test_tasting: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTestsResponse {
    pub tests: Vec<TestSummary>,
    pub categories: HashMap<String, TestCategory>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCategory {
    #[serde(serialize_with = "zoned_as_millis")]
    pub starts_at: Zoned,
    #[serde(serialize_with = "zoned_as_millis")]
    pub ends_at: Zoned,
}

impl From<crate::config::TestCategory> for TestCategory {
    fn from(value: crate::config::TestCategory) -> Self {
        Self {
            starts_at: value.starts_at,
            ends_at: value.ends_at,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum SetTestResponse {
    TestAdded(Test),
    TastingFailed { output: TestExecutionOutput },
}

fn zoned_as_millis<S>(zoned: &Zoned, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    jiff::fmt::serde::timestamp::millisecond::required::serialize(&zoned.timestamp(), serializer)
}
