use crate::auth::{BackendAuth, display_login};
use indicatif::ProgressBar;
use jiff::Zoned;
use jiff::tz::TimeZone;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shared::{TestExecutionOutput, TestModifier};
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[derive(Debug, Snafu)]
pub enum CliContextError {
    #[snafu(display("Request error at {location}"))]
    Reqwest {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Error deserializing backend response at {location}"))]
    Deserialization {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "Server replied with an unexpected code ({status_code}) saying {message} at {location}"
    ))]
    BackendStatusCode {
        status_code: StatusCode,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone)]
pub struct CliContext {
    auth: BackendAuth,
    client: Client,
    backend_url: String,
    frontend_url: String,
}

impl CliContext {
    pub fn new(
        auth: BackendAuth,
        client: Client,
        backend_url: String,
        frontend_url: String,
    ) -> Self {
        Self {
            auth,
            client,
            backend_url,
            frontend_url,
        }
    }

    pub fn get_myself(&self) -> Result<Myself, CliContextError> {
        let res = self
            .client
            .get(format!("{}/users/me", self.backend_url))
            .headers(self.get_headers())
            .send()
            .context(ReqwestSnafu)?;

        let myself: MyselfResponse = self.get_json_response(res)?;
        Ok(myself.user)
    }

    pub fn get_remote_tests(&self) -> Result<RemoteTests, CliContextError> {
        let res = self
            .client
            .get(format!("{}/tests", self.backend_url))
            .headers(self.get_headers())
            .send()
            .context(ReqwestSnafu)?;

        self.get_json_response(res)
    }

    pub fn get_test_detail(&self, id: &str) -> Result<TestDetail, CliContextError> {
        let mut url = Url::from_str(&format!("{}/tests", self.backend_url)).expect("url is valid");
        url.path_segments_mut().expect("url is a base url").push(id);

        let res = self
            .client
            .get(url)
            .headers(self.get_headers())
            .send()
            .context(ReqwestSnafu)?;

        self.get_json_response(res)
    }

    pub fn upload_test(
        &self,
        id: &str,
        category: &str,
        detail: &TestDetail,
        should_taste_test: bool,
    ) -> Result<SetTestResponse, CliContextError> {
        let url = Url::from_str(&format!("{}/tests/", self.backend_url))
            .expect("url is valid")
            .join(id)
            .expect("url is valid after join");

        let res = thread::scope(|s| {
            let show_progress_bar = Arc::new(AtomicBool::new(true));
            let show_progress_bar_clone = show_progress_bar.clone();

            let progress_task = s.spawn(move || {
                let spinner = ProgressBar::new_spinner().with_message("Uploading test");
                while show_progress_bar_clone.load(Ordering::Acquire) {
                    spinner.tick();
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                spinner.finish_with_message("Request completed");
            });
            let computing_task = s.spawn(move || {
                self.client
                    .put(url)
                    .headers(self.get_headers())
                    .json(&serde_json::json!({
                        "compilerModifiers": detail.compiler_modifiers,
                        "binaryModifiers": detail.binary_modifiers,
                        "category": category,
                        "ignoreTestTasting": !should_taste_test,
                    }))
                    .send()
                    .context(ReqwestSnafu)
            });
            let res = computing_task.join().expect("computing task panicked");
            show_progress_bar.store(false, Ordering::Release);
            progress_task.join().expect("progress task panicked");

            res
        });
        let res = res?;

        self.get_json_response(res)
    }

    fn get_headers(&self) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        header_map.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.auth))
                .expect("header value is valid ascii"),
        );

        header_map
    }

    fn get_json_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, CliContextError> {
        if response.status() == StatusCode::UNAUTHORIZED {
            display_login(&self.frontend_url)
        }
        if response.status() == StatusCode::OK {
            return response.json::<T>().context(DeserializationSnafu);
        }

        Err(BackendStatusCodeSnafu {
            status_code: response.status(),
            message: response.text().unwrap_or("N/A".to_string()),
        }
        .into_error(NoneError))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTests {
    pub tests: Vec<Test>,
    pub categories: HashMap<String, TestCategory>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCategory {
    #[serde(deserialize_with = "zoned_as_millis")]
    pub starts_at: Zoned,
    #[serde(deserialize_with = "zoned_as_millis")]
    pub labs_end_at: Zoned,
    #[serde(deserialize_with = "zoned_as_millis")]
    pub _tests_end_at: Zoned,
}
fn zoned_as_millis<'de, D>(deserializer: D) -> Result<Zoned, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let res = jiff::fmt::serde::timestamp::millisecond::required::deserialize(deserializer)?;
    Ok(res.to_zoned(TimeZone::system()))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub id: String,
    pub creator_id: String,
    pub admin_authored: bool,
    pub limited_to_category: bool,
    #[serde(skip_serializing, default)]
    pub category: String,
    pub hash: String,
}

impl Test {
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(&self.category)
            .join(format!("{}.crow-test.md", self.id))
    }

    pub fn local_file_paths(&self, root: &Path) -> Vec<PathBuf> {
        vec![self.path(root)]
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDetail {
    pub compiler_modifiers: Vec<TestModifier>,
    pub binary_modifiers: Vec<TestModifier>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Myself {
    pub display_name: String,
    pub team: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyselfResponse {
    pub user: Myself,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum SetTestResponse {
    TestAdded(#[allow(dead_code)] serde_json::Value),
    TastingFailed { output: TestExecutionOutput },
}
