use crate::containers::ImageRegistry;
use crate::docker::DockerClient;
use crate::executor::Executor;
use bollard::Docker;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;
mod docker;
mod executor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let docker_client = DockerClient::new(Docker::connect_with_defaults()?);
    docker_client
        .export_image_unpacked("alpine:latest", "target/images/alpine-latest")
        .await?;

    // Base: Image registry that maps image names to rootfs directories
    let registry = ImageRegistry::new("target/images");
    let image = registry.get_images()?[0].clone();
    let executor = Executor::new(docker_client, registry);

    let res = executor
        .build_main_container(image, &["/bin/sh", "-c", "echo Hello, world!"])
        .await?;

    info!("Compiler build result: {:?}", res);

    let build_container = res.container;

    let args = [
        "/bin/sh",
        "-c",
        "echo 'hey' >> /tmp/foo.txt && cat /tmp/foo.txt",
    ];
    println!(
        "{:?}",
        executor
            .run_test(build_container.container(), &args)
            .await?
    );
    println!(
        "{:?}",
        executor
            .run_test(build_container.container(), &args)
            .await?
    );
    println!(
        "{:?}",
        executor
            .run_test(build_container.container(), &args)
            .await?
    );

    build_container.destroy().await?;

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
