use crate::types::Test;
use serde::{Deserialize, Serialize};
use shared::{ExecutionOutput, RunnerId, TestTasteId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::oneshot;
use tokio::{select, spawn};
use tracing::error;

pub struct TestTasting {
    open_tastings: Vec<OpenTestTaste>,
    in_progress_tastings: HashMap<TestTasteId, (OpenTestTaste, RunnerId)>,
    _drop_guard: oneshot::Sender<()>,
}

impl TestTasting {
    pub fn new() -> Arc<Mutex<Self>> {
        let (tx, rx) = oneshot::channel();
        let result = Self {
            open_tastings: Vec::new(),
            in_progress_tastings: HashMap::new(),
            _drop_guard: tx,
        };
        let result = Arc::new(Mutex::new(result));

        // cleanup
        let result_clone = result.clone();
        spawn(async move {
            let work = async {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                    result_clone
                        .lock()
                        .unwrap()
                        .in_progress_tastings
                        .retain(|_, (taste, _)| !taste.expired());
                    result_clone
                        .lock()
                        .unwrap()
                        .open_tastings
                        .retain(|taste| !taste.expired());
                }
            };

            select! {
                _ = work => {},
                _ = rx => {}
            }
        });

        result
    }

    pub fn get_tasting_runners(&self) -> HashSet<RunnerId> {
        self.in_progress_tastings
            .values()
            .map(|(_, runner_id)| runner_id.clone())
            .collect()
    }

    pub fn add_tasting(&mut self, test: Test) -> oneshot::Receiver<ExecutionOutput> {
        let (tx, rx) = oneshot::channel();
        self.open_tastings.push(OpenTestTaste {
            result_channel: tx,
            test,
            start_time: Instant::now(),
            id: uuid::Uuid::new_v4().to_string().into(),
        });

        rx
    }

    pub fn poll_tasting(&mut self, runner_id: RunnerId) -> Option<TestTastingTask> {
        let taste = self.open_tastings.pop()?;
        let test = taste.test.clone();
        let id = taste.id.clone();

        self.in_progress_tastings
            .insert(taste.id.clone(), (taste, runner_id));

        Some(TestTastingTask { test, taste_id: id })
    }

    pub fn finish_tasting(&mut self, id: TestTasteId, output: ExecutionOutput) {
        if let Some((taste, _)) = self.in_progress_tastings.remove(&id) {
            let _ = taste.result_channel.send(output);
        } else {
            error!(
                id = %id,
                "Tried to finish a test tasting that was not in progress"
            );
        }
    }
}

struct OpenTestTaste {
    pub result_channel: oneshot::Sender<ExecutionOutput>,
    pub test: Test,
    pub start_time: Instant,
    pub id: TestTasteId,
}

impl OpenTestTaste {
    fn expired(&self) -> bool {
        self.result_channel.is_closed() || self.start_time.elapsed().as_secs() > 60 * 5
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTastingTask {
    pub test: Test,
    pub taste_id: TestTasteId,
}
