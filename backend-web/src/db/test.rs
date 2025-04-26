use crate::error::{Result, SqlxSnafu, WebError};
use crate::types::{TeamId, Test, TestId, TestSummary, TestWithTasteTesting};
use jiff::Timestamp;
use sha2::{Digest, Sha256};
use shared::{TestExecutionOutput, TestExecutionOutputType};
use snafu::{location, ResultExt};
use sqlx::{query, query_as, Acquire, Sqlite, SqliteConnection};
use tracing::{info_span, instrument, Instrument};

#[instrument(skip_all)]
pub(super) async fn add_test(
    con: impl Acquire<'_, Database = Sqlite>,
    test: Test,
    test_tasting: Option<TestExecutionOutput>,
) -> Result<Test> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let compiler_modifiers =
        serde_json::to_string(&test.compiler_modifiers).expect("Unexpected json serialize error");
    let binary_modifiers =
        serde_json::to_string(&test.binary_modifiers).expect("Unexpected json serialize error");

    let mut hash = Sha256::new();
    hash.update(compiler_modifiers.as_bytes());
    hash.update(binary_modifiers.as_bytes());
    hash.update(test.owner.to_string().as_bytes());
    hash.update([test.admin_authored as u8]);
    hash.update(test.category.as_bytes());
    let hash = format!("{:x}", hash.finalize());

    let last_updated = test.last_updated.as_millisecond();
    query!(
        r#"
        INSERT INTO Tests
            (id, owner, category, compiler_modifiers, binary_modifiers, admin_authored, hash,
             provisional, last_updated)
        VALUES
            (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET
            compiler_modifiers = excluded.compiler_modifiers,
            binary_modifiers = excluded.binary_modifiers,
            admin_authored = excluded.admin_authored,
            category = excluded.category,
            hash = excluded.hash,
            last_updated = excluded.last_updated
        "#,
        test.id,
        test.owner,
        test.category,
        compiler_modifiers,
        binary_modifiers,
        test.admin_authored,
        hash,
        test.provisional,
        last_updated,
    )
    .execute(&mut *con)
    .instrument(info_span!("sqlx_add_test"))
    .await
    .context(SqlxSnafu)?;

    if let Some(output) = test_tasting {
        let (compiler_exec, test_exec) =
            super::task::record_test_execution(&mut con, &output).await?;
        let status = TestExecutionOutputType::from(&output).to_string();
        query!(
            r#"
            INSERT INTO TestTastingResults
                (test_id, compiler_exec_id, binary_exec_id, status)
            VALUES
                (?, ?, ?, ?)
            ON CONFLICT DO UPDATE SET
                compiler_exec_id = excluded.compiler_exec_id,
                binary_exec_id = excluded.binary_exec_id,
                status = excluded.status
            "#,
            test.id,
            compiler_exec,
            test_exec,
            status
        )
        .execute(&mut *con)
        .instrument(info_span!("sqlx_add_test_tasting"))
        .await
        .context(SqlxSnafu)?;
    }

    let test = query_as!(
        DbTest,
        r#"
        SELECT
            id as "id!: TestId",
            owner as "owner!: TeamId",
            category,
            compiler_modifiers,
            binary_modifiers,
            admin_authored,
            provisional,
            last_updated
        FROM Tests
        WHERE id = ?"#,
        test.id
    )
    .map(Test::from)
    .fetch_one(&mut *con)
    .instrument(info_span!("sqlx_add_get_test"))
    .await
    .context(SqlxSnafu)?;

    con.commit().await.context(SqlxSnafu)?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn get_tests(con: &mut SqliteConnection) -> Result<Vec<Test>> {
    query_as!(
        DbTest,
        r#"
        SELECT
            id as "id!: TestId",
            owner as "owner!: TeamId",
            category,
            compiler_modifiers,
            binary_modifiers,
            admin_authored,
            provisional,
            last_updated
        FROM Tests
        "#
    )
    .map(Test::from)
    .fetch_all(con)
    .instrument(info_span!("sqlx_get_tests"))
    .await
    .context(SqlxSnafu)
}

#[instrument(skip_all)]
pub(super) async fn get_tests_summaries(con: &mut SqliteConnection) -> Result<Vec<TestSummary>> {
    let success_status = TestExecutionOutputType::Success.to_string();
    query_as!(
        TestSummary,
        r#"
        SELECT
            Tests.id as "id!: TestId",
            Teams.display_name as "creator_name",
            Teams.id as "creator_id!: TeamId",
            Tests.admin_authored,
            Tests.category,
            Tests.hash,
            (SELECT status == ? FROM TestTastingResults WHERE test_id = Tests.id)
                as "test_taste_success?: bool",
            Tests.provisional,
            Tests.last_updated as "last_updated!: DbMillis"
        FROM Tests
        JOIN Teams ON Tests.owner = Teams.id
        "#,
        success_status
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
        DbTest,
        r#"
        SELECT
            id as "id!: TestId",
            owner as "owner!: TeamId",
            category,
            compiler_modifiers,
            binary_modifiers,
            admin_authored,
            provisional,
            last_updated
        FROM Tests
        WHERE id = ?
        "#,
        test_id
    )
    .map(Test::from)
    .fetch_optional(con)
    .instrument(info_span!("sqlx_fetch_test"))
    .await
    .context(SqlxSnafu)?;

    Ok(test)
}

#[instrument(skip_all)]
pub(super) async fn fetch_test_with_tasting(
    con: impl Acquire<'_, Database = Sqlite>,
    test_id: &TestId,
) -> Result<Option<TestWithTasteTesting>> {
    let mut con = con.begin().await.context(SqlxSnafu)?;

    let Some(test) = fetch_test(&mut con, test_id).await? else {
        return Ok(None);
    };

    let taste_test_execution = query!(
        "SELECT compiler_exec_id, binary_exec_id, status FROM TestTastingResults WHERE test_id = ?",
        test_id
    )
    .fetch_optional(&mut *con)
    .await
    .context(SqlxSnafu)?;

    let test_tasting_result = match taste_test_execution {
        Some(exec) => Some(
            super::task::get_test_execution(
                &mut con,
                &exec.compiler_exec_id,
                exec.binary_exec_id,
                exec.status.parse().unwrap(),
            )
            .await?
            .into(),
        ),
        None => None,
    };

    Ok(Some(TestWithTasteTesting {
        test,
        test_tasting_result,
    }))
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

struct DbTest {
    id: TestId,
    owner: TeamId,
    category: String,
    compiler_modifiers: String,
    binary_modifiers: String,
    admin_authored: bool,
    provisional: bool,
    last_updated: i64,
}

impl From<DbTest> for Test {
    fn from(value: DbTest) -> Self {
        Self {
            id: value.id,
            owner: value.owner,
            category: value.category,
            compiler_modifiers: serde_json::from_str(&value.compiler_modifiers)
                .expect("Unexpected json serialize error"),
            binary_modifiers: serde_json::from_str(&value.binary_modifiers)
                .expect("Unexpected json serialize error"),
            admin_authored: value.admin_authored,
            provisional: value.provisional,
            last_updated: DbMillis(value.last_updated).into(),
        }
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
struct DbMillis(i64);

impl From<DbMillis> for Timestamp {
    fn from(value: DbMillis) -> Self {
        Self::from_millisecond(value.0).expect("time is valid")
    }
}
