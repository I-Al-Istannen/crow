use crate::types::test::TestId;
use crate::types::TeamId;
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shared::{FinishedCompilerTask, FinishedTest};
use std::collections::HashMap;

#[derive(Default)]
pub struct Executor {
    pub todo: Vec<WorkItem>,
    pub in_progress: HashMap<TaskId, WorkItem>,
}

impl Executor {
    pub fn add_task(&mut self, task: WorkItem) {
        self.todo.push(task);
    }

    pub fn pop_task(&mut self) -> Option<WorkItem> {
        match self.todo.pop() {
            Some(task) => {
                self.in_progress.insert(task.id.clone(), task.clone());
                Some(task)
            }
            None => None,
        }
    }

    pub fn get_running_task(&self, id: &TaskId) -> Option<WorkItem> {
        self.in_progress.get(id).cloned()
    }

    pub fn finish_task(&mut self, id: &TaskId, _result: &FinishedCompilerTask) {
        self.in_progress.remove(id);
    }
}

#[derive(Debug, Clone, Hash, From, PartialEq, Eq, Display, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TaskId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: TaskId,
    pub team: TeamId,
    pub revision: String,
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
