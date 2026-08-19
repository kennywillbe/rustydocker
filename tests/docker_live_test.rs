use futures_util::StreamExt;
use rustydocker::config::AppConfig;
use rustydocker::docker::client::DockerClient;
use rustydocker::docker::connection;
use std::process::Command;
use std::time::Duration;

struct ContainerCleanup(String);

impl Drop for ContainerCleanup {
    fn drop(&mut self) {
        let _ = Command::new("docker").args(["rm", "-f", &self.0]).output();
    }
}

#[tokio::test]
#[ignore = "requires a running Docker daemon and network access"]
async fn live_image_and_log_workflows() {
    let resolved = connection::resolve(None, &AppConfig::default()).unwrap();
    let docker = DockerClient::new(&resolved.spec).unwrap();
    let image = "alpine:3.21";
    let image_existed = docker
        .list_images()
        .await
        .unwrap()
        .iter()
        .any(|candidate| candidate.repo_tags.iter().any(|tag| tag == image));

    docker.pull_image(image).await.unwrap();
    let inspect = docker.inspect_image(image).await.unwrap();
    assert_eq!(inspect.os.as_deref(), Some("linux"));
    assert!(!docker.image_history(image).await.unwrap().is_empty());

    let name = format!("rustydocker-v050-smoke-{}", std::process::id());
    let cleanup = ContainerCleanup(name.clone());
    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &name,
            image,
            "sh",
            "-c",
            "echo smoke-ready; sleep 1; echo smoke-follow; sleep 10",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "docker run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let container = docker
        .list_containers()
        .await
        .unwrap()
        .into_iter()
        .find(|container| {
            container
                .names
                .as_ref()
                .is_some_and(|names| names.iter().any(|candidate| candidate == &format!("/{name}")))
        })
        .unwrap();
    let id = container.id.unwrap();
    let since = i32::try_from(chrono::Utc::now().timestamp() - 2).unwrap();
    let mut stream = docker.container_logs_follow_since(&id, since);
    let received_follow = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(result) = stream.next().await {
            if result.unwrap().to_string().contains("smoke-follow") {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(received_follow);
    drop(stream);

    docker.remove_container(&id).await.unwrap();
    drop(cleanup);
    if !image_existed {
        docker.remove_image(image, false).await.unwrap();
    }
}
