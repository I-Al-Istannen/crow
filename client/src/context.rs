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
}

impl CliContext {
    pub fn new(auth: BackendAuth, client: Client) -> Self {
        Self { auth, client }
    }

    pub fn get_remote_tests(&self) -> Result<RemoteTests, CliContextError> {
        let res = self
            .client
            .get("http://localhost:3000/tests")
            .headers(self.get_headers())
            .send()
            .context(ReqwestSnafu)?;

        get_json_response(res)
    }

    pub fn get_test_detail(&self, id: &str) -> Result<TestDetail, CliContextError> {
        let url = Url::from_str("http://localhost:3000/tests/")
            .expect("url is valid")
            .join(id)
            .expect("url is valid after join");
        let res = self
            .client
            .get(url)
            .headers(self.get_headers())
            .send()
            .context(ReqwestSnafu)?;

        get_json_response(res)
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
}

fn get_json_response<T: DeserializeOwned>(response: Response) -> Result<T, CliContextError> {
    if response.status() == StatusCode::UNAUTHORIZED {
        display_login()
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

    pub fn local_file_paths(&self, root: &Path) -> Vec<PathBuf> {
        vec![
            root.join(&self.category)
                .join(format!("{}.crow-test", self.id)),
            root.join(&self.category)
                .join(format!("{}.crow-test.meta", self.id)),
            root.join(&self.category)
                .join(format!("{}.crow-test.expected", self.id)),
        ]
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDetail {
    pub expected_output: String,
    pub input: String,
}
