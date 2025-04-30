use crate::containers::{
    execution_output_from_wait_error, Built, ContainerCreateError, IntegrateSourceError,
    TaskContainer, TestRunError,
};
use crate::docker::ImageId;
use rayon::ThreadPool;
use shared::{
    CompilerTask, CompilerTest, ExecutionOutput, FinishedCompilerTask, FinishedExecution,
    FinishedTaskInfo, FinishedTest, InternalError, RunnerUpdate, TestExecutionOutput,
};
use snafu::{location, Location, Report, ResultExt, Snafu};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant, SystemTime};
use tempfile::TempPath;
use tracing::{error, info};

#[derive(Debug, Snafu)]
pub enum TaskRunError {
    #[snafu(display("Could not create container at {location}"))]
    ContainerCreate {
        source: ContainerCreateError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not integrate source at {location}"))]
    IntegrateSource {
        source: IntegrateSourceError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not run container at {location}"))]
    ContainerRun {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not wait for build at {location}"))]
    WaitForBuild {
        output: ExecutionOutput,
        #[snafu(implicit)]
        location: Location,
    },
}

pub struct ExecutingTask<'a> {
    pub inner: CompilerTask,
    pub pool: &'a ThreadPool,
    pub aborted: Arc<AtomicBool>,
    pub message_channel: mpsc::Sender<RunnerUpdate>,
}

pub fn execute_task(task: ExecutingTask<'_>, source_tar: TempPath) -> FinishedCompilerTask {
    let task_id = task.inner.task_id.clone();
    let team_id = task.inner.team_id.clone();
    let revision_id = task.inner.revision_id.clone();
    let commit_message = task.inner.commit_message.clone();
    let start = SystemTime::now();
    let start_monotonic = Instant::now();
    let message_channel = task.message_channel.clone();

    let res = match execute_task_impl(task, source_tar) {
        Ok(res) => res,
        Err(e) => task_run_error_to_task(
            start,
            start_monotonic,
            task_id,
            team_id,
            revision_id,
            commit_message,
            e,
        ),
    };
    let _ = message_channel.send(RunnerUpdate::Done);

    res
}

fn execute_task_impl(
    task: ExecutingTask<'_>,
    source_tar: TempPath,
) -> Result<FinishedCompilerTask, TaskRunError> {
    let start = SystemTime::now();
    let start_monotonic = Instant::now();

    let pool = task.pool;
    let aborted = task.aborted;
    let message_channel = task.message_channel;
    let task = task.inner;
    let container = TaskContainer::new(&ImageId(task.image), &task.build_command)
        .context(ContainerCreateSnafu)?;

    container
        .integrate_source(source_tar)
        .context(IntegrateSourceSnafu)?;

    let container = container.run().context(ContainerRunSnafu)?;
    let _ = message_channel.send(RunnerUpdate::StartedBuild);
    let container = container
        .wait_for_build(task.build_timeout, aborted.clone())
        .map_err(|output| TaskRunError::WaitForBuild {
            output,
            location: location!(),
        })?;
    let build_output = FinishedExecution {
        stdout: container.data.stdout.clone(),
        stderr: container.data.stderr.clone(),
        runtime: container.data.runtime,
        exit_status: container.data.exit_status.code(),
    };
    let _ = message_channel.send(RunnerUpdate::FinishedBuild {
        result: build_output.clone(),
    });

    if !container.data.exit_status.success() {
        return Ok(FinishedCompilerTask::BuildFailed {
            info: FinishedTaskInfo {
                task_id: task.task_id,
                end: SystemTime::now(),
                start,
                team_id: task.team_id,
                revision_id: task.revision_id,
                commit_message: task.commit_message,
            },
            build_output: ExecutionOutput::Failure {
                execution: build_output,
                accumulated_errors: None,
            },
        });
    }

    let test_results = pool.scope(|s| {
        let (tx, rx) = mpsc::channel();

        for test in task.tests {
            let tx = tx.clone();
            let container = &container;
            let aborted = aborted.clone();
            let message_channel = message_channel.clone();
            s.spawn(move |_| {
                let _ = message_channel.send(RunnerUpdate::StartedTest {
                    test_id: test.test_id.clone(),
                });
                let res = container.run_test(&test, test.timeout, aborted);
                let res = tx.send((test.clone(), res));
                if let Err(e) = res {
                    error!(
                        test_id = test.test_id.as_str(),
                        error = ?e,
                        "Could not send test result"
                    );
                }
            });
        }

        // Drop the original tx so that the receiver is collected when all threads are done
        drop(tx);

        let mut results = Vec::new();
        while let Ok((test, res)) = rx.recv() {
            let result = match res {
                Ok(res) => res,
                Err(e) => TestExecutionOutput::Error {
                    output_so_far: test_run_error_to_output(
                        start_monotonic,
                        task.task_id.clone(),
                        test.test_id.clone(),
                        e,
                    ),
                },
            };
            let result = FinishedTest {
                test_id: test.test_id,
                output: result,
                provisional_for_category: test.provisional_for_category,
            };
            results.push(result.clone());
            let _ = message_channel.send(RunnerUpdate::FinishedTest { result });
        }
        results
    });

    Ok(FinishedCompilerTask::RanTests {
        info: FinishedTaskInfo {
            task_id: task.task_id,
            end: SystemTime::now(),
            start,
            team_id: task.team_id,
            revision_id: task.revision_id,
            commit_message: task.commit_message,
        },
        build_output,
        tests: test_results,
    })
}

fn task_run_error_to_task(
    start: SystemTime,
    start_monotonic: Instant,
    task_id: String,
    team_id: String,
    revision_id: String,
    commit_message: String,
    e: TaskRunError,
) -> FinishedCompilerTask {
    let info = FinishedTaskInfo {
        task_id: task_id.clone(),
        end: SystemTime::now(),
        start,
        team_id,
        revision_id,
        commit_message,
    };

    if let TaskRunError::WaitForBuild { output, .. } = e {
        return FinishedCompilerTask::BuildFailed {
            info,
            build_output: output,
        };
    }

    // We have *some* internal error
    let report = Report::from_error(e);
    error!(
        error = ?report,
        task_id = task_id.as_str(),
        "Internal error while building task"
    );

    FinishedCompilerTask::BuildFailed {
        info,
        build_output: ExecutionOutput::Error(InternalError {
            runtime: start_monotonic.elapsed(),
            message: format!("Internal error while building task:\n{}", report),
        }),
    }
}

fn test_run_error_to_output(
    start_monotonic: Instant,
    task_id: String,
    test_id: String,
    e: TestRunError,
) -> ExecutionOutput {
    if let TestRunError::Execution { source, .. } = &e {
        if let Some(res) = execution_output_from_wait_error(source) {
            return res;
        }
    }

    // We have *some* internal error
    let report = Report::from_error(e);
    error!(
        error = ?report,
        test_id = test_id.as_str(),
        task_id = task_id.as_str(),
        "Internal error while running test"
    );

    ExecutionOutput::Error(InternalError {
        runtime: start_monotonic.elapsed(),
        message: format!("Internal error while running test:\n{}", report),
    })
}

pub fn run_test(
    task_id: String,
    image_id: &ImageId,
    test: CompilerTest,
    shutdown_requested: Arc<AtomicBool>,
    base_container: Rc<RefCell<Option<TaskContainer<Built>>>>,
) -> TestExecutionOutput {
    let test_id = test.test_id.clone();
    let start = Instant::now();
    let res = run_test_impl(
        task_id.clone(),
        image_id,
        test,
        shutdown_requested,
        base_container,
    );

    match res {
        Ok(res) => res,
        Err(e) => {
            let report = Report::from_error(e);
            error!(
                error = ?report,
                task_id = task_id.as_str(),
                test_id = test_id.as_str(),
                "Internal error while setting up test"
            );

            TestExecutionOutput::Error {
                output_so_far: ExecutionOutput::Error(InternalError {
                    runtime: start.elapsed(),
                    message: format!("Internal error while setting up test:\n{}", report),
                }),
            }
        }
    }
}

fn run_test_impl(
    task_id: String,
    image_id: &ImageId,
    test: CompilerTest,
    shutdown_requested: Arc<AtomicBool>,
    base_container: Rc<RefCell<Option<TaskContainer<Built>>>>,
) -> Result<TestExecutionOutput, TaskRunError> {
    if base_container.borrow().is_none() {
        info!("Creating reference compiler container");
        // We have nothing really to do here, so we just use `true` as the builder.
        let container = TaskContainer::new(image_id, &["true".to_string()])
            .context(ContainerCreateSnafu)?
            .run()
            .context(ContainerRunSnafu)?;
        let container = container
            .wait_for_build(Duration::from_secs(10), shutdown_requested.clone())
            .map_err(|output| TaskRunError::WaitForBuild {
                output,
                location: location!(),
            })?;

        *base_container.borrow_mut() = Some(container);
    }
    let base_container = base_container.borrow();
    let base_container = base_container.as_ref().unwrap();

    let start = Instant::now();
    let res = base_container.run_test(&test, test.timeout, shutdown_requested.clone());

    let res = match res {
        Ok(res) => res,
        Err(e) => {
            return Ok(TestExecutionOutput::Error {
                output_so_far: test_run_error_to_output(
                    start,
                    task_id.to_string(),
                    test.test_id,
                    e,
                ),
            })
        }
    };

    Ok(res)
}
