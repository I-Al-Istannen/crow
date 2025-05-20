use super::{Json, Path};
use crate::auth::Claims;
use crate::error::{Result, WebError};
use crate::types::{AppState, Test, TestId, TestSummary, TestWithTasteTesting};
use axum::extract::State;
use jiff::{Timestamp, Zoned};
use serde::{Deserialize, Serialize};
use shared::{TestExecutionOutput, TestModifier};
use snafu::location;
use std::collections::HashMap;
use tracing::{debug, info, instrument};

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
    let mut owner = claims.team.clone();
    let mut admin_authored = claims.is_admin();
    let mut limited_to_category = false;

    if let Some(existing) = db.fetch_test(&test_id).await? {
        if existing.owner != claims.team && !claims.is_admin() {
            return Err(WebError::unauthorized(location!()));
        }
        // Even if an admin edits a test, this stays the same
        owner = existing.owner;
        admin_authored = existing.admin_authored;
        limited_to_category = existing.limited_to_category;
    }

    let Some(category_meta) = state.test_config.categories.get(&payload.category) else {
        return Err(WebError::named_not_found(payload.category, location!()));
    };

    let provisional = category_meta.is_after_test_deadline();
    // If the time is up you can no longer edit finalized tests as a normal user
    // (just create new ones)
    if let Some(test) = state.db.fetch_test(&test_id).await? {
        if provisional && !claims.is_admin() && test.provisional_for_category.is_none() {
            return Err(WebError::named_unauthorized(
                "edit an existing test after test deadline".to_string(),
                location!(),
            ));
        }
    }
    let provisional_for_category = if provisional {
        Some(payload.category.clone())
    } else {
        None
    };

    let test = Test {
        id: test_id,
        owner: owner.clone(),
        admin_authored,
        category: payload.category,
        compiler_modifiers: payload.compiler_modifiers,
        binary_modifiers: payload.binary_modifiers,
        limited_to_category,
        provisional_for_category,
        last_updated: Timestamp::now(),
    };

    // Let the reference compiler taste it first
    let taste_testing_result = do_test_tasting(&state, &test).await?;

    if let Some(result) = &taste_testing_result {
        if !matches!(result, TestExecutionOutput::Success { .. }) && !payload.ignore_test_tasting {
            info!(
                test_id = %test.id,
                owner = %owner,
                team = %claims.team,
                user = %claims.sub,
                "Test failed testing"
            );
            return Ok(Json(SetTestResponse::TastingFailed {
                output: result.clone(),
            }));
        }
    }

    info!(
        test_id = %test.id,
        owner = %owner,
        team = %claims.team,
        user = %claims.sub,
        "Adding/modifying test"
    );

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
    State(AppState {
        db, test_config, ..
    }): State<AppState>,
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

        if let Some(category) = test_config.categories.get(&test.category) {
            if test.provisional_for_category.is_none() && category.is_after_test_deadline() {
                return Err(WebError::named_unauthorized(
                    "delete a finalized test".to_string(),
                    location!(),
                ));
            }
        }
    }

    info!(
        test_id = %test_id,
        team = %claims.team,
        user = %claims.sub,
        "Deleting test"
    );

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
    pub labs_end_at: Zoned,
    #[serde(serialize_with = "zoned_as_millis")]
    pub tests_end_at: Zoned,
}

impl From<crate::config::TestCategory> for TestCategory {
    fn from(value: crate::config::TestCategory) -> Self {
        Self {
            starts_at: value.starts_at,
            labs_end_at: value.labs_end_at,
            tests_end_at: value.tests_end_at,
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
