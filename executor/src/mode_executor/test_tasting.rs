use crate::containers::{Built, TaskContainer};
use crate::docker::ImageId;
use crate::mode_executor::CliExecutorArgs;
use crate::{task_executor, AnyError, Endpoints, ReqwestSnafu, NO_TASK_BACKOFF};
use reqwest::blocking::Client;
use shared::{RunnerInfo, RunnerWorkTasteTestDone, RunnerWorkTasteTestResponse};
use snafu::{Report, ResultExt};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub struct TestTastingState {
    pub container: Rc<RefCell<Option<TaskContainer<Built>>>>,
}

impl TestTastingState {
    pub fn new() -> Self {
        Self {
            container: Rc::new(RefCell::new(None)),
        }
    }
}

impl super::Iteration for TestTastingState {
    fn iteration(
        &mut self,
        args: &CliExecutorArgs,
        endpoints: &Endpoints,
        current_backoff: &mut Duration,
        shutdown_requested: &Arc<AtomicBool>,
        client: &Client,
        runner_info: &RunnerInfo,
    ) -> Result<(), AnyError> {
        let response = match client
            .post(&endpoints.work_taste_test)
            .json(runner_info)
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
                super::backoff(current_backoff, shutdown_requested);
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
            super::backoff(current_backoff, shutdown_requested);
            return Ok(());
        }
        let task = match response.json::<RunnerWorkTasteTestResponse>() {
            Err(e) => {
                warn!(
                    error = ?Report::from_error(e),
                    endpoint = %endpoints.work,
                    next_retry = ?current_backoff,
                    "Failed to parse task"
                );
                super::backoff(current_backoff, shutdown_requested);
                return Ok(());
            }
            Ok(task) => task,
        };
        let Some(task) = task.task else {
            let current_backoff = &mut NO_TASK_BACKOFF.clone();
            info!(backoff = ?current_backoff, "No test received");
            super::backoff(current_backoff, shutdown_requested);
            return Ok(());
        };
        let task_id = task.id;

        info!(
            task = %task_id,
            test = %task.test.test_id,
            "Received test to taste"
        );
        let res = task_executor::run_test(
            task_id.to_string(),
            &ImageId(task.image_id),
            task.test,
            shutdown_requested.clone(),
            self.container.clone(),
        );
        let res = RunnerWorkTasteTestDone {
            output: res,
            id: task_id.clone(),
        };

        info!(id = %task_id, res = ?res, "Task finished");
        client
            .post(&endpoints.done_taste_test)
            .json(&res)
            .basic_auth(&args.id, Some(&args.token))
            .send()
            .context(ReqwestSnafu)?;

        Ok(())
    }
}
