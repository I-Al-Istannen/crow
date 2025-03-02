use sha2::{Digest, Sha256};
use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{TeamId, Test, TestId, TestSummary};
use snafu::{location, ResultExt};
use sqlx::{query, query_as, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_test(con: &mut SqliteConnection, test: Test) -> Result<Test> {
    let mut hash = Sha256::new();
    hash.update(test.expected_output.as_bytes());
    hash.update(test.input.as_bytes());
    hash.update(test.owner.to_string().as_bytes());
    hash.update([test.admin_authored as u8]);
    hash.update(test.category.as_bytes());
    let hash = format!("{:x}", hash.finalize());

    query!(
        r#"
        INSERT INTO Tests 
            (id, expected_output, input, owner, admin_authored, category, hash)
        VALUES
            (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            expected_output = excluded.expected_output,
            input = excluded.input,
            admin_authored = excluded.admin_authored,
            category = excluded.category,
            hash = excluded.hash
        "#,
        test.id,
        test.expected_output,
        test.input,
        test.owner,
        test.admin_authored,
        test.category,
        hash
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_test"))
    .await
    .context(SqlxSnafu)?;

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
    .await
    .context(SqlxSnafu)?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn get_tests(con: &mut SqliteConnection) -> Result<Vec<Test>> {
    query_as!(
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
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_tests_summaries(con: &mut SqliteConnection) -> Result<Vec<TestSummary>> {
    query_as!(
        TestSummary,
        r#"
        SELECT
            Tests.id as "id!: TestId",
            Teams.display_name as "creator_name",
            Teams.id as "creator_id!: TeamId",
            Tests.admin_authored,
            Tests.category,
            Tests.hash
        FROM Tests
        JOIN Teams ON Tests.owner = Teams.id
        "#
    )
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_test_summaries"))
    .await
    .context(SqlxSnafu)
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
    .await
    .context(SqlxSnafu)?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn delete_test(con: &mut SqliteConnection, test_id: &TestId) -> Result<()> {
    let res = query!(r#"DELETE FROM Tests WHERE id = ?"#, test_id)
        .execute(con)
        .instrument(info_span!("sqlx_delete_test"))
        .await
        .context(SqlxSnafu)?;

    if res.rows_affected() == 0 {
        return Err(WebError::not_found(location!()));
    }

    Ok(())
}
