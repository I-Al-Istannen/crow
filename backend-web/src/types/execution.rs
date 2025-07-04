use crate::types::queue::Queue;
use crate::types::{FinishedTestSummary, TeamId, TestId};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shared::{
    ExecutionOutput, FinishedCompilerTask, FinishedExecution, RunnerId, RunnerInfo,
    TestExecutionOutput, deserialize_system_time, serialize_system_time,
};
use snafu::{Location, Snafu, ensure};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::select;
use tokio::sync::broadcast;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct Runner {
    pub info: RunnerInfo,
    pub working_on: Option<WorkItem>,
    pub last_ping: SystemTime,
    pub test_taster: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum RunnerWorkForFrontend {
    Testing(WorkItem),
    TestTasting,
}

impl RunnerWorkForFrontend {
    pub fn task(&self) -> Option<&WorkItem> {
        match self {
            Self::Testing(it) => Some(it),
            Self::TestTasting => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerForFrontend {
    pub id: RunnerId,
    pub info: String,
    pub working_on: Option<RunnerWorkForFrontend>,
    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub last_seen: SystemTime,
    pub test_taster: bool,
}

impl From<&Runner> for RunnerForFrontend {
    fn from(value: &Runner) -> Self {
        Self {
            id: value.info.id.clone(),
            info: value.info.info.clone(),
            working_on: value.working_on.clone().map(RunnerWorkForFrontend::Testing),
            last_seen: value.last_ping,
            test_taster: value.test_taster,
        }
    }
}

struct InternalRunningTaskState {
    so_far: Vec<RunnerUpdateForFrontend>,
    sender: broadcast::Sender<RunnerUpdateForFrontend>,
}

pub struct RunningTaskState {
    pub so_far: Vec<RunnerUpdateForFrontend>,
    pub receiver: broadcast::Receiver<RunnerUpdateForFrontend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerUpdate {
    AllTests {
        tests: Vec<TestId>,
    },
    StartedBuild,
    FinishedBuild {
        result: FinishedExecution,
    },
    #[serde(rename_all = "camelCase")]
    StartedTest {
        test_id: String,
    },
    FinishedTest {
        result: FinishedTestSummary,
    },
    Done,
}

impl From<shared::RunnerUpdate> for RunnerUpdate {
    fn from(value: shared::RunnerUpdate) -> Self {
        match value {
            shared::RunnerUpdate::StartedBuild => Self::StartedBuild,
            shared::RunnerUpdate::FinishedBuild { result } => Self::FinishedBuild { result },
            shared::RunnerUpdate::StartedTest { test_id } => Self::StartedTest { test_id },
            shared::RunnerUpdate::FinishedTest { result } => Self::FinishedTest {
                result: result.into(),
            },
            shared::RunnerUpdate::Done => Self::Done,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunnerUpdateForFrontend {
    update: RunnerUpdate,
    #[serde(serialize_with = "serialize_system_time")]
    time: SystemTime,
}

impl From<RunnerUpdate> for RunnerUpdateForFrontend {
    fn from(value: RunnerUpdate) -> Self {
        Self {
            update: value,
            time: SystemTime::now(),
        }
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct ExecutorInfo {
    pub runners: Vec<RunnerForFrontend>,
    pub in_progress: Vec<(TaskId, usize)>,
}

pub struct Executor {
    runners: HashMap<RunnerId, Runner>,
    in_progress: HashMap<TaskId, InternalRunningTaskState>,
    _old_runner_cleanup: tokio::sync::oneshot::Sender<()>,
}

impl Executor {
    pub fn new() -> Arc<Mutex<Self>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let res = Arc::new(Mutex::new(Self {
            in_progress: HashMap::new(),
            runners: HashMap::new(),
            _old_runner_cleanup: tx,
        }));

        let res_clone = res.clone();
        tokio::task::spawn(async move {
            let periodic = async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    res_clone.lock().unwrap().runners.retain(|_, v| {
                        let time_since_ping =
                            v.last_ping.elapsed().unwrap_or(Duration::from_secs(0));

                        time_since_ping < Duration::from_secs(5 * 60)
                    });
                }
            };
            select! {
                _ = periodic => {},
                _ = rx => {},
            }
        });

        res
    }

    pub fn get_runners(&self, tasting_runners: HashSet<RunnerId>) -> Vec<RunnerForFrontend> {
        let mut result: Vec<RunnerForFrontend> =
            self.runners.values().map(|it| it.into()).collect();

        for runner in &mut result {
            if tasting_runners.contains(&runner.id) {
                runner.working_on = Some(RunnerWorkForFrontend::TestTasting);
            }
        }

        result
    }

    pub fn info(&self, tasting_runners: HashSet<RunnerId>) -> ExecutorInfo {
        ExecutorInfo {
            runners: self.get_runners(tasting_runners),
            in_progress: self
                .in_progress
                .iter()
                .map(|(k, v)| (k.clone(), v.sender.receiver_count()))
                .collect(),
        }
    }

    pub fn runner_pinged(&mut self, runner_id: &RunnerId) {
        if let Some(runner) = self.runners.get_mut(runner_id) {
            runner.last_ping = SystemTime::now();
        }
    }

    pub fn register_runner(&mut self, runner_info: &RunnerInfo) {
        self.runners.insert(
            runner_info.id.clone(),
            Runner {
                info: runner_info.clone(),
                working_on: None,
                last_ping: SystemTime::now(),
                test_taster: runner_info.test_taster,
            },
        );
    }

    pub fn get_running_task(&self, id: &TaskId) -> Option<RunningTaskState> {
        self.in_progress.get(id).map(|it| RunningTaskState {
            so_far: it.so_far.clone(),
            receiver: it.sender.subscribe(),
        })
    }

    pub fn update_task(&mut self, runner_id: &RunnerId, update: RunnerUpdate) {
        let Some(runner) = self.runners.get(runner_id) else {
            return;
        };
        let Some(task) = runner.working_on.as_ref() else {
            return;
        };
        let Some(state) = self.in_progress.get_mut(&task.id) else {
            return;
        };

        let update: RunnerUpdateForFrontend = update.into();
        state.so_far.push(update.clone());

        if let Err(e) = state.sender.send(update) {
            warn!(
                runner = %runner_id,
                task = %task.id,
                error = ?e,
                "Failed to send update to task"
            );
        }
    }

    pub fn assign_work(
        &mut self,
        runner_info: &RunnerInfo,
        all_tasks: &[WorkItem],
        test_ids: Vec<TestId>,
        queue: Arc<Mutex<Queue>>,
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

        let task = queue.lock().unwrap().poll_task(
            all_tasks
                .iter()
                .filter(|it| !taken.contains(&it.id))
                .cloned()
                .collect(),
        );

        let runner = self.runners.get_mut(&runner_id).unwrap();
        runner.working_on = task.clone();

        if let Some(task) = &task {
            let (sender, mut rx) = broadcast::channel(100);

            // Drain dummy receiver so sending will always work
            tokio::spawn(async move { while rx.recv().await.is_ok() {} });

            self.in_progress.insert(
                task.id.clone(),
                InternalRunningTaskState {
                    so_far: vec![RunnerUpdate::AllTests { tests: test_ids }.into()],
                    sender,
                },
            );
        }

        Ok(task)
    }

    pub fn get_current_task(&self, id: &RunnerId) -> Option<WorkItem> {
        self.runners.get(id).and_then(|it| it.working_on.clone())
    }

    pub fn finish_task(&mut self, runner: &RunnerId) {
        if let Some(runner) = self.runners.get_mut(runner) {
            if let Some(task) = &runner.working_on {
                self.in_progress.remove(&task.id);
            }
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
    pub commit_message: String,
    #[serde(serialize_with = "serialize_system_time")]
    #[serde(deserialize_with = "deserialize_system_time")]
    pub insert_time: SystemTime,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, From, sqlx::Type)]
pub enum ExecutionExitStatus {
    Aborted,
    Error,
    Failure,
    Success,
    Timeout,
}

impl From<&ExecutionOutput> for ExecutionExitStatus {
    fn from(value: &ExecutionOutput) -> Self {
        match value {
            ExecutionOutput::Aborted(_) => Self::Aborted,
            ExecutionOutput::Error(_) => Self::Error,
            ExecutionOutput::Success(_) => Self::Success,
            ExecutionOutput::Failure { .. } => Self::Failure,
            ExecutionOutput::Timeout(_) => Self::Timeout,
        }
    }
}

impl From<&TestExecutionOutput> for ExecutionExitStatus {
    fn from(value: &TestExecutionOutput) -> Self {
        value
            .binary_output()
            .unwrap_or(value.compiler_output())
            .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueuedTaskStatus {
    Aborted,
    Error,
    Failure,
    Queued,
    Running,
    Success,
    Timeout,
}

impl From<FinishedCompilerTask> for QueuedTaskStatus {
    fn from(value: FinishedCompilerTask) -> Self {
        let status: Vec<ExecutionExitStatus> = match value {
            FinishedCompilerTask::BuildFailed { build_output, .. } => {
                vec![(&build_output).into()]
            }
            FinishedCompilerTask::RanTests { tests, .. } => {
                tests.into_iter().map(|it| (&it.output).into()).collect()
            }
        };

        if status.contains(&ExecutionExitStatus::Aborted) {
            return Self::Aborted;
        }
        if status.contains(&ExecutionExitStatus::Error) {
            return Self::Error;
        }
        if status.contains(&ExecutionExitStatus::Timeout) {
            return Self::Error;
        }
        if status.contains(&ExecutionExitStatus::Failure) {
            return Self::Failure;
        }
        Self::Success
    }
}
