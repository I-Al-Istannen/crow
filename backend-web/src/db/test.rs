use crate::error::WebError;
use crate::types::{TeamId, Test, TestId};
use sqlx::{query, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_test(con: &mut SqliteConnection, test: Test) -> Result<Test, WebError> {
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

    let test = query!(
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
    .map(|it| Test {
        id: it.id,
        name: it.name,
        expected_output: it.expected_output,
        owner: it.owner,
    })
    .fetch_one(con)
    .instrument(info_span!("sqlx_add_get_test"))
    .await?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn get_tests(con: &mut SqliteConnection) -> Result<Vec<Test>, WebError> {
    Ok(query!(
        r#"
        SELECT
            id as "id!: TestId",
            name,
            expected_output,
            owner as "owner!: TeamId"
        FROM Tests
        "#
    )
    .map(|it| Test {
        id: it.id,
        name: it.name,
        expected_output: it.expected_output,
        owner: it.owner,
    })
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_tests"))
    .await?)
}

#[instrument(skip_all)]
pub(super) async fn fetch_test(
    con: &mut SqliteConnection,
    test_id: &TestId,
) -> Result<Option<Test>, WebError> {
    let test = query!(
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
    .map(|it| Test {
        id: it.id,
        name: it.name,
        expected_output: it.expected_output,
        owner: it.owner,
    })
    .fetch_optional(con)
    .instrument(info_span!("sqlx_fetch_test"))
    .await?;

    Ok(test)
}
