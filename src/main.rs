use crate::containers::{create_build_container, run_container, ImageRegistry};
use tokio::io::AsyncBufReadExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Base: Image registry that maps image names to rootfs directories
    let registry = ImageRegistry::new("target/images");
    let image = registry.get_images()?[0].clone();

    let container =
        create_build_container(&registry, &image, &["/bin/sh", "-c", "echo Hello, world!"]).await?;
    let mut container = run_container(container).await?;
    let stdout = container.process().stdout.take().unwrap();
    let stderr = container.process().stderr.take().unwrap();

    tokio::spawn(async {
        let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
        while let Some(line) = stdout_lines.next_line().await.unwrap() {
            println!("stdout: {}", line);
        }
        let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
        while let Some(line) = stderr_lines.next_line().await.unwrap() {
            println!("stderr: {}", line);
        }
    });
    println!("Exit code: {}", container.process().wait().await?);
    container.destroy().await?;

    // Create and run the build container
    //  1. Call a standardized entrypoint
    //    1. Stream the log output directly to a higher layer (might take a while to build).
    //       maybe even force colors or emulate a PTY in the build inside the container.
    //    2. stdout/stderr are both recorded and passed through
    //    3. size limits on output?
    //  2. Capture results (stdout, stderr, exit code, time)
    //  3. Commit rootfs somehow (previously created overlayfs? Or copy the original rootfs and use
    //     the resulting, modified FS as new rootfs?)

    // Create a test container
    //   1. Create a working dir
    //   2. Create the overlayfs mount (or let runc do it)
    //   3. Render the runc config
    // Run the test container
    //   1. Build the test case program and execute it (standardized entrypoint)
    //   2. Wait for death
    //     1. Stream the log output directly to a higher layer? Or buffer with size limit and return
    //        at the end?
    //     2. Capture results (stdout, stderr, exit code, time)
    //     3. Kill container after timeout
    //   3. On container death
    //     1. Unmount all mounts (none if runc did it)
    //     2. Delete the workdir (mounts must be dead at this point)
    //   4. Return result

    Ok(())
}
