use crate::containers::{
    ContainerCreateError, IntegrateSourceError, TaskContainer, TestRunError, WaitForContainerError,
};
use crate::docker::ImageId;
use rayon::ThreadPool;
use shared::{
    AbortedExecution, CompilerTask, ExecutionOutput, FinishedCompilerTask, FinishedExecution,
    FinishedTest, InternalError,
};
use snafu::{Location, Report, ResultExt, Snafu};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use std::time::{Instant, SystemTime};
use tempfile::TempPath;
use tracing::error;

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
        source: WaitForContainerError,
        #[snafu(implicit)]
        location: Location,
    },
}

pub struct ExecutingTask<'a> {
    pub inner: CompilerTask,
    pub pool: &'a ThreadPool,
    pub aborted: Arc<AtomicBool>,
}

pub fn execute_task(task: ExecutingTask, source_tar: TempPath) -> FinishedCompilerTask {
    let task_id = task.inner.task_id.clone();
    let start = SystemTime::now();
    let start_monotonic = Instant::now();

    match execute_task_impl(task, source_tar) {
        Ok(res) => res,
        Err(e) => task_run_error_to_task(start, start_monotonic, task_id, e),
    }
}

fn execute_task_impl(
    task: ExecutingTask,
    source_tar: TempPath,
) -> Result<FinishedCompilerTask, TaskRunError> {
    let start = SystemTime::now();
    let start_monotonic = Instant::now();

    let pool = task.pool;
    let aborted = task.aborted;
    let task = task.inner;
    let container = TaskContainer::new(&ImageId(task.image), &task.build_command)
        .context(ContainerCreateSnafu)?;

    container
        .integrate_source(source_tar)
        .context(IntegrateSourceSnafu)?;

    let container = container.run().context(ContainerRunSnafu)?;
    let container = container
        .wait_for_build(task.build_timeout, aborted.clone())
        .context(WaitForBuildSnafu)?;

    let test_results = pool.scope(|s| {
        let (tx, rx) = mpsc::channel();

        for test in task.tests {
            let tx = tx.clone();
            let container = &container;
            let aborted = aborted.clone();
            s.spawn(move |_| {
                let res = container.run_test(&test.run_command, test.timeout, aborted);
                let res = tx.send((test.test_id.clone(), res));
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
        while let Ok(val) = rx.recv() {
            results.push(val);
        }
        results
    });

    let task_id = task.task_id.clone();
    let mut finished_tests = Vec::new();
    for (test_id, res) in test_results {
        let output = match res {
            Ok(res) => ExecutionOutput::Finished(FinishedExecution {
                stdout: res.stdout,
                stderr: res.stderr,
                runtime: res.runtime,
                exit_status: None,
            }),
            Err(e) => {
                test_run_error_to_output(start_monotonic, task_id.clone(), test_id.clone(), e)
            }
        };

        finished_tests.push(FinishedTest { test_id, output });
    }

    Ok(FinishedCompilerTask::RanTests {
        start,
        build_output: FinishedExecution {
            stdout: container.data.stdout.clone(),
            stderr: container.data.stderr.clone(),
            runtime: container.data.runtime,
            exit_status: None,
        },
        tests: finished_tests,
    })
}

fn task_run_error_to_task(
    start: SystemTime,
    start_monotonic: Instant,
    task_id: String,
    e: TaskRunError,
) -> FinishedCompilerTask {
    if let TaskRunError::WaitForBuild { source, .. } = &e {
        if let Some(build_output) = execution_output_from_wait_error(source) {
            return FinishedCompilerTask::BuildFailed {
                start,
                build_output,
            };
        }
    }

    // We have *some* internal error
    let report = Report::from_error(e);
    error!(
        error = ?report,
        task_id = task_id.as_str(),
        "Internal error while building task"
    );

    FinishedCompilerTask::BuildFailed {
        start,
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

fn execution_output_from_wait_error(error: &WaitForContainerError) -> Option<ExecutionOutput> {
    if let WaitForContainerError::Timeout {
        runtime,
        stdout,
        stderr,
        ..
    } = error
    {
        return Some(ExecutionOutput::Timeout(FinishedExecution {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            runtime: *runtime,
            exit_status: None,
        }));
    }
    if let WaitForContainerError::Aborted {
        runtime,
        stdout,
        stderr,
        ..
    } = error
    {
        return Some(ExecutionOutput::Aborted(AbortedExecution {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            runtime: *runtime,
        }));
    }

    None
}
