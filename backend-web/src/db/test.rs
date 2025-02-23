use crate::error::{Result, WebError};
use crate::types::{TeamId, Test, TestId, TestSummary};
use sqlx::{SqliteConnection, query, query_as};
use tracing::{Instrument, info_span, instrument};

#[instrument(skip_all)]
pub(super) async fn add_test(con: &mut SqliteConnection, test: Test) -> Result<Test> {
    query!(
        r#"
        INSERT INTO Tests 
            (id, expected_output, input, owner, admin_authored, category)
        VALUES
            (?, ?, ?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            expected_output = excluded.expected_output,
            input = excluded.input,
            admin_authored = excluded.admin_authored,
            category = excluded.category
        "#,
        test.id,
        test.expected_output,
        test.input,
        test.owner,
        test.admin_authored,
        test.category
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_test"))
    .await?;

    let test = query_as!(
        Test,
        r#"
        SELECT
            id as "id!: TestId",
            expected_output,
            input,
            owner as "owner!: TeamId",
            admin_authored,
            category
        FROM Tests
        WHERE id = ?"#,
        test.id
    )
    .fetch_one(con)
    .instrument(info_span!("sqlx_add_get_test"))
    .await?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn get_tests(con: &mut SqliteConnection) -> Result<Vec<Test>> {
    Ok(query_as!(
        Test,
        r#"
        SELECT
            id as "id!: TestId",
            expected_output,
            input,
            owner as "owner!: TeamId",
            admin_authored,
            category
        FROM Tests
        "#
    )
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_tests"))
    .await?)
}

#[instrument(skip_all)]
pub(super) async fn get_tests_summaries(con: &mut SqliteConnection) -> Result<Vec<TestSummary>> {
    Ok(query_as!(
        TestSummary,
        r#"
        SELECT
            Tests.id as "id!: TestId",
            Teams.display_name as "creator_name",
            Teams.id as "creator_id!: TeamId",
            Tests.admin_authored,
            Tests.category
        FROM Tests
        JOIN Teams ON Tests.owner = Teams.id
        "#
    )
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_test_summaries"))
    .await?)
}

#[instrument(skip_all)]
pub(super) async fn fetch_test(
    con: &mut SqliteConnection,
    test_id: &TestId,
) -> Result<Option<Test>> {
    let test = query_as!(
        Test,
        r#"
        SELECT 
            id as "id!: TestId",
            expected_output,
            input,
            owner as "owner!: TeamId",
            admin_authored,
            category
        FROM Tests
        WHERE id = ?
        "#,
        test_id
    )
    .fetch_optional(con)
    .instrument(info_span!("sqlx_fetch_test"))
    .await?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn delete_test(con: &mut SqliteConnection, test_id: &TestId) -> Result<()> {
    let res = query!(r#"DELETE FROM Tests WHERE id = ?"#, test_id)
        .execute(con)
        .instrument(info_span!("sqlx_delete_test"))
        .await?;

    if res.rows_affected() == 0 {
        return Err(WebError::NotFound);
    }

    Ok(())
}
