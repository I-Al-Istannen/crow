use crate::types::test::TestId;
use crate::types::TeamId;
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shared::{
    deserialize_system_time, serialize_system_time, ExecutionOutput, FinishedCompilerTask,
    FinishedTest, RunnerId, RunnerInfo,
};
use snafu::{ensure, Location, Snafu};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Runner {
    pub info: RunnerInfo,
    pub working_on: Option<WorkItem>,
    pub last_ping: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerForFrontend {
    pub id: RunnerId,
    pub info: String,
    pub working_on: Option<WorkItem>,
    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub last_seen: SystemTime,
}

impl From<&Runner> for RunnerForFrontend {
    fn from(value: &Runner) -> Self {
        Self {
            id: value.info.id.clone(),
            info: value.info.info.clone(),
            working_on: value.working_on.clone(),
            last_seen: value.last_ping,
        }
    }
}

#[derive(Default)]
pub struct Executor {
    runners: HashMap<RunnerId, Runner>,
}

#[derive(Debug, Snafu)]
pub enum ExecutorError {
    #[snafu(display("Runner `{runner_id}` not found at {location}"))]
    RunnerNotFound {
        runner_id: RunnerId,
        #[snafu(implicit)]
        location: Location,
    },
}

impl Executor {
    pub fn get_runners(&self) -> Vec<RunnerForFrontend> {
        self.runners.values().map(|it| it.into()).collect()
    }

    pub fn register_runner(&mut self, runner_info: &RunnerInfo) -> Option<TaskId> {
        if let Some(runner) = self.runners.get_mut(&runner_info.id) {
            runner.info = runner_info.clone();
            runner.last_ping = SystemTime::now();
            return runner.working_on.as_ref().map(|it| it.id.clone());
        }

        self.runners.insert(
            runner_info.id.clone(),
            Runner {
                info: runner_info.clone(),
                working_on: None,
                last_ping: SystemTime::now(),
            },
        );

        None
    }

    pub fn assign_work(
        &mut self,
        runner_info: &RunnerInfo,
        queue: &[WorkItem],
    ) -> Result<Option<WorkItem>, ExecutorError> {
        ensure!(
            self.runners.contains_key(&runner_info.id),
            RunnerNotFoundSnafu {
                runner_id: runner_info.id.clone()
            }
        );

        let runner_id = runner_info.id.clone();

        let taken: HashSet<TaskId> = self
            .runners
            .values()
            .filter(|it| it.info.id != runner_info.id)
            .flat_map(|it| it.working_on.clone())
            .map(|it| it.id)
            .collect();

        let task = queue.iter().find(|it| !taken.contains(&it.id)).cloned();

        let runner = self.runners.get_mut(&runner_id).unwrap();
        runner.working_on = task.clone();

        Ok(task)
    }

    pub fn get_current_task(&self, id: &RunnerId) -> Option<WorkItem> {
        self.runners.get(id).and_then(|it| it.working_on.clone())
    }

    pub fn finish_task(&mut self, runner: &RunnerId) {
        if let Some(runner) = self.runners.get_mut(runner) {
            runner.working_on = None;
        }
    }
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TaskId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: TaskId,
    pub team: TeamId,
    pub revision: String,
    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub insert_time: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkMessage {
    TestStarted(TestId),
    TestFinished(FinishedTest),
    TestUpdate {
        id: TestId,
        stdout: String,
        stderr: String,
    },
    TaskFinished(FinishedCompilerTask),
}

#[derive(Debug, Clone, Serialize, Deserialize, From, sqlx::Type)]
pub enum ExecutionExitStatus {
    Aborted,
    Error,
    Finished,
    Timeout,
}

impl From<&ExecutionOutput> for ExecutionExitStatus {
    fn from(value: &ExecutionOutput) -> Self {
        match value {
            ExecutionOutput::Aborted(_) => Self::Aborted,
            ExecutionOutput::Error(_) => Self::Error,
            ExecutionOutput::Finished(_) => Self::Finished,
            ExecutionOutput::Timeout(_) => Self::Timeout,
        }
    }
}
