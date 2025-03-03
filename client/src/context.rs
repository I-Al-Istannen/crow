use crate::auth::{display_login, BackendAuth};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{IntoError, Location, NoneError, ResultExt, Snafu};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
        let url = Url::from_str(&format!("{}/tests/", self.backend_url))
            .expect("url is valid")
            .join(id)
            .expect("url is valid after join");
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
        input: &str,
        expected: &str,
    ) -> Result<(), CliContextError> {
        let url = Url::from_str(&format!("{}/tests/", self.backend_url))
            .expect("url is valid")
            .join(id)
            .expect("url is valid after join");

        let res = self
            .client
            .put(url)
            .headers(self.get_headers())
            .json(&serde_json::json!({
                "category": category,
                "input": input,
                "expectedOutput": expected,
            }))
            .send()
            .context(ReqwestSnafu)?;

        let _: serde_json::Value = self.get_json_response(res)?;

        Ok(())
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
    pub categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Test {
    pub id: String,
    pub creator_id: String,
    pub admin_authored: bool,
    #[serde(skip_serializing, default)]
    pub category: String,
    pub hash: String,
}

impl Test {
    pub fn path_input(&self, root: &Path) -> PathBuf {
        root.join(&self.category)
            .join(format!("{}.crow-test", self.id))
    }

    pub fn path_meta(&self, root: &Path) -> PathBuf {
        root.join(&self.category)
            .join(format!("{}.crow-test.meta", self.id))
    }

    pub fn path_expected(&self, root: &Path) -> PathBuf {
        root.join(&self.category)
            .join(format!("{}.crow-test.expected", self.id))
    }

    pub fn local_file_paths(&self, root: &Path) -> Vec<PathBuf> {
        vec![
            self.path_input(root),
            self.path_meta(root),
            self.path_expected(root),
        ]
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDetail {
    pub expected_output: String,
    pub input: String,
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
