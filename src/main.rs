use crate::containers::{DockerImage, ImageRegistry};
use containers::{ContainerRegistry, RuncTemplate};
use std::path::Path;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod containers;

const RUNC_TEMPLATE: &str = include_str!("../config-rootless.json");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let registry = ImageRegistry::new(Path::new("target/images"));
    let mut container_registry =
        ContainerRegistry::new(registry, RuncTemplate::new(RUNC_TEMPLATE.to_string()));

    let image = DockerImage {
        name: "alpine".to_string(),
        tag: "latest".to_string(),
    };
    let container_id = container_registry.create_container(image).await?;
    println!("{:?}", container_id);

    let container_id = container_registry.run_container(container_id).await?;
    container_registry.kill_container(container_id).await?;

    println!("Done!");

    Ok(())
}
