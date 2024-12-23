use crate::containers::{ContainerCreateError, TaskContainer, TestRunError, WaitForContainerError};
use crate::docker::ImageId;
use rayon::ThreadPool;
use shared::{
    AbortedExecution, CompilerTask, ExecutionOutput, FinishedCompilerTask, FinishedExecution,
    FinishedTest, InternalError,
};
use snafu::{Location, Report, ResultExt, Snafu};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};
use tracing::error;

#[derive(Debug, Snafu)]
pub enum TaskRunError {
    #[snafu(display("Could not create container at {location}"))]
    ContainerCreate {
        source: ContainerCreateError,
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

pub fn execute_task(task: ExecutingTask) -> FinishedCompilerTask {
    let task_id = task.inner.run_id.clone();
    match execute_task_impl(task) {
        Ok(res) => res,
        Err(e) => task_run_error_to_task(task_id, e),
    }
}

fn execute_task_impl(task: ExecutingTask) -> Result<FinishedCompilerTask, TaskRunError> {
    let pool = task.pool;
    let aborted = task.aborted;
    let task = task.inner;
    let container = TaskContainer::new(&ImageId(task.image), &task.build_command)
        .context(ContainerCreateSnafu)?;
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

    let task_id = task.run_id.clone();
    let mut finished_tests = Vec::new();
    for (test_id, res) in test_results {
        let output = match res {
            Ok(res) => ExecutionOutput::Finished(FinishedExecution {
                stdout: res.stdout,
                stderr: res.stderr,
                runtime: res.runtime,
                exit_status: None,
            }),
            Err(e) => test_run_error_to_output(task_id.clone(), test_id.clone(), e),
        };

        finished_tests.push(FinishedTest { test_id, output });
    }

    Ok(FinishedCompilerTask::RanTests {
        build_output: FinishedExecution {
            stdout: container.data.stdout.clone(),
            stderr: container.data.stderr.clone(),
            runtime: container.data.runtime,
            exit_status: None,
        },
        tests: finished_tests,
    })
}

fn task_run_error_to_task(task_id: String, e: TaskRunError) -> FinishedCompilerTask {
    if let TaskRunError::WaitForBuild { source, .. } = &e {
        if let Some(build_output) = execution_output_from_wait_error(source) {
            return FinishedCompilerTask::BuildFailed { build_output };
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
        build_output: ExecutionOutput::Error(InternalError {
            message: format!("Internal error while building task:\n{}", report),
            id: task_id,
        }),
    }
}

fn test_run_error_to_output(task_id: String, test_id: String, e: TestRunError) -> ExecutionOutput {
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
        message: format!("Internal error while running test:\n{}", report),
        id: test_id,
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
