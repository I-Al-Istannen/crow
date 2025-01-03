use crate::error::Result;
use crate::types::{TeamId, Test, TestId, TestSummary};
use sqlx::{query, query_as, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_test(con: &mut SqliteConnection, test: Test) -> Result<Test> {
    query!(
        r#"
        INSERT INTO Tests 
            (id, name, expected_output, owner)
        VALUES
            (?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            name = excluded.name,
            expected_output = excluded.expected_output
        "#,
        test.id,
        test.name,
        test.expected_output,
        test.owner
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_test"))
    .await?;

    let test = query_as!(
        Test,
        r#"
        SELECT
            id as "id!: TestId",
            name,
            expected_output,
            owner as "owner!: TeamId"
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
            name,
            expected_output,
            owner as "owner!: TeamId"
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
            Tests.name,
            Teams.display_name as "creator_name",
            Teams.id as "creator_id!: TeamId"
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
            name,
            expected_output,
            owner as "owner!: TeamId"
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
