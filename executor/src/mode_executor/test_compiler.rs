use crate::containers::LimitsConfig;
use crate::docker::Docker;
use crate::mode_executor::{backoff, start_update_listener, CliExecutorArgs};
use crate::task_executor::{execute_task, ExecutingTask};
use crate::{AnyError, Endpoints, ReqwestSnafu, TempFileSnafu, NO_TASK_BACKOFF};
use rayon::{ThreadPool, ThreadPoolBuilder};
use reqwest::blocking::Client;
use shared::{RunnerInfo, RunnerWorkResponse};
use snafu::{location, Report, ResultExt};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct TestCompilerState {
    thread_pool: ThreadPool,
    docker: Docker,
    build_limits: LimitsConfig,
    test_limits: LimitsConfig,
}

impl TestCompilerState {
    pub fn new(
        docker: Docker,
        max_parallelism: usize,
        build_limits: LimitsConfig,
        test_limits: LimitsConfig,
    ) -> Result<Self, AnyError> {
        let thread_pool = match ThreadPoolBuilder::new()
            .num_threads(max_parallelism)
            .build()
        {
            Err(e) => {
                return Err(AnyError::ThreadPoolBuild {
                    source: e,
                    location: location!(),
                });
            }
            Ok(result) => result,
        };

        Ok(Self {
            thread_pool,
            docker,
            build_limits,
            test_limits,
        })
    }
}

impl super::Iteration for TestCompilerState {
    fn iteration(
        &mut self,
        args: &CliExecutorArgs,
        endpoints: &Endpoints,
        current_backoff: &mut Duration,
        shutdown_requested: &Arc<AtomicBool>,
        client: &Client,
        runner_info: &RunnerInfo,
    ) -> Result<(), AnyError> {
        client
            .post(&endpoints.register)
            .basic_auth(&args.id, Some(&args.token))
            .json(&runner_info)
            .send()
            .context(ReqwestSnafu)?;

        let response = match client
            .post(&endpoints.work)
            .json(&runner_info)
            .basic_auth(&args.id, Some(&args.token))
            .send()
        {
            Err(e) => {
                warn!(
                    error = ?Report::from_error(e),
                    endpoint = %endpoints.work,
                    next_retry = ?current_backoff,
                    "Failed to request work"
                );
                backoff(current_backoff, shutdown_requested);
                return Ok(());
            }
            Ok(response) => response,
        };
        if !response.status().is_success() {
            warn!(
                status = %response.status(),
                endpoint = %endpoints.work,
                next_retry = ?current_backoff,
                "Failed to request work"
            );
            backoff(current_backoff, shutdown_requested);
            return Ok(());
        }
        let task = match response.json::<RunnerWorkResponse>() {
            Err(e) => {
                warn!(
                    error = ?Report::from_error(e),
                    endpoint = %endpoints.work,
                    next_retry = ?current_backoff,
                    "Failed to parse task"
                );
                backoff(current_backoff, shutdown_requested);
                return Ok(());
            }
            Ok(task) => task,
        };
        let Some(task) = task.task else {
            let current_backoff = &mut NO_TASK_BACKOFF.clone();
            debug!(backoff = ?current_backoff, "No task received");
            backoff(current_backoff, shutdown_requested);
            return Ok(());
        };
        let task_id = task.task_id.clone();

        info!(id = task_id, "Received task");
        let (tx, rx) = std::sync::mpsc::channel();
        let source_tar = tempfile::NamedTempFile::new().context(TempFileSnafu)?;
        client
            .get(&endpoints.tar)
            .basic_auth(&args.id, Some(&args.token))
            .send()
            .context(ReqwestSnafu)?
            .copy_to(&mut source_tar.as_file())
            .context(ReqwestSnafu)?;

        let task = ExecutingTask {
            inner: task,
            pool: &self.thread_pool,
            aborted: shutdown_requested.clone(),
            message_channel: tx,
        };
        start_update_listener(args, endpoints, rx);
        let res = execute_task(
            task,
            source_tar.into_temp_path(),
            &self.docker,
            &self.build_limits,
            &self.test_limits,
        );

        info!(id = task_id, res = ?res.info(), "Task finished");
        let res = client
            .post(&endpoints.done)
            .json(&res)
            .basic_auth(&args.id, Some(&args.token))
            .send()
            .context(ReqwestSnafu)?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res
                .text()
                .unwrap_or_else(|_| "Failed to read response".to_string());
            warn!(
                status = %status,
                endpoint = %endpoints.done,
                body = %body,
                "Failed to send task result"
            );
        }

        Ok(())
    }
}
