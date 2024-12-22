use crate::containers::{ContainerCreateError, TaskContainer, TestRunError, WaitForContainerError};
use crate::docker::ImageId;
use crate::types::{
    CompilerTask, FinishedCompilerTask, FinishedExecution, FinishedTest, InternalError,
};
use rayon::ThreadPool;
use snafu::{Location, Report, ResultExt, Snafu};
use std::sync::mpsc;
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

pub fn execute_task(task: CompilerTask, pool: &ThreadPool) -> FinishedCompilerTask {
    let task_id = task.run_id.clone();
    match execute_task_impl(task, pool) {
        Ok(res) => res,
        Err(e) => finished_task_from_task_run_error(task_id, e),
    }
}

fn execute_task_impl(
    task: CompilerTask,
    pool: &ThreadPool,
) -> Result<FinishedCompilerTask, TaskRunError> {
    let container = TaskContainer::new(&ImageId(task.image), &task.build_command)
        .context(ContainerCreateSnafu)?;
    let container = container.run().context(ContainerRunSnafu)?;
    let container = container
        .wait_for_build(task.build_timeout)
        .context(WaitForBuildSnafu)?;

    let test_results = pool.scope(|s| {
        let (tx, rx) = mpsc::channel();

        for test in task.tests {
            let tx = tx.clone();
            let container = &container;
            s.spawn(move |_| {
                let res = container.run_test(&test.run_command, test.timeout);
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
            Ok(res) => Ok(FinishedExecution {
                stdout: res.stdout,
                stderr: res.stderr,
                runtime: res.runtime,
                exit_status: None,
                timeout: false,
            }),
            Err(e) => result_from_test_run_error(task_id.clone(), test_id.clone(), e),
        };

        finished_tests.push(FinishedTest { test_id, output });
    }

    Ok(FinishedCompilerTask::RanTests {
        build_output: FinishedExecution {
            stdout: container.data.stdout.clone(),
            stderr: container.data.stderr.clone(),
            runtime: container.data.runtime,
            exit_status: None,
            timeout: false,
        },
        tests: finished_tests,
    })
}

fn finished_task_from_task_run_error(task_id: String, e: TaskRunError) -> FinishedCompilerTask {
    if let TaskRunError::WaitForBuild {
        source:
            WaitForContainerError::Timeout {
                runtime,
                stdout,
                stderr,
                ..
            },
        ..
    } = &e
    {
        return FinishedCompilerTask::BuildFailed {
            build_output: Ok(FinishedExecution {
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                runtime: *runtime,
                exit_status: None,
                timeout: true,
            }),
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
        build_output: Err(InternalError {
            message: format!("Internal error while building task:\n{}", report),
            id: task_id,
        }),
    }
}

fn result_from_test_run_error(
    task_id: String,
    test_id: String,
    e: TestRunError,
) -> Result<FinishedExecution, InternalError> {
    if let TestRunError::Execution {
        source:
            WaitForContainerError::Timeout {
                runtime,
                stdout,
                stderr,
                ..
            },
        ..
    } = &e
    {
        return Ok(FinishedExecution {
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            runtime: *runtime,
            exit_status: None,
            timeout: true,
        });
    }

    // We have *some* internal error
    let report = Report::from_error(e);
    error!(
        error = ?report,
        test_id = test_id.as_str(),
        task_id = task_id.as_str(),
        "Internal error while running test"
    );

    Err(InternalError {
        message: format!("Internal error while running test:\n{}", report),
        id: test_id,
    })
}
