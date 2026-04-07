use anyhow::Result;
use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, RestartContainerOptions,
    StartContainerOptions, StopContainerOptions, Stats, StatsOptions,
};
use bollard::image::{ListImagesOptions, PruneImagesOptions};
use bollard::models::{ContainerInspectResponse, ContainerSummary, ImageSummary, SystemInfo, Volume};
use bollard::volume::{ListVolumesOptions, PruneVolumesOptions};
use bollard::Docker;
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

use crate::config;

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerSummary>> {
        let opts = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        Ok(self.docker.list_containers(Some(opts)).await?)
    }

    /// Inspect a container by ID. Used for future features.
    #[allow(dead_code)]
    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspectResponse> {
        Ok(self.docker.inspect_container(id, None).await?)
    }

    /// Fetch last N log lines (non-streaming, returns all at once)
    pub async fn container_logs_batch(&self, id: &str) -> Result<Vec<String>> {
        let opts = LogsOptions::<String> {
            follow: false,
            stdout: true,
            stderr: true,
            tail: config::LOG_TAIL_LINES.to_string(),
            ..Default::default()
        };
        let mut stream = self.docker.logs(id, Some(opts));
        let mut lines = vec![];
        while let Some(result) = stream.next().await {
            if let Ok(output) = result {
                lines.push(output.to_string());
            }
        }
        Ok(lines)
    }

    /// Stream new log lines (follow mode, no tail backlog)
    pub fn container_logs_follow(
        &self,
        id: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + Send + '_>>
    {
        let opts = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            since: chrono::Utc::now().timestamp(),
            tail: "0".to_string(),
            ..Default::default()
        };
        Box::pin(self.docker.logs(id, Some(opts)))
    }

    pub fn container_stats(
        &self,
        id: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<Stats, bollard::errors::Error>> + Send + '_>> {
        let opts = StatsOptions {
            stream: true,
            one_shot: false,
        };
        Box::pin(self.docker.stats(id, Some(opts)))
    }

    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.docker
            .start_container(id, None::<StartContainerOptions<String>>)
            .await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> Result<()> {
        self.docker
            .stop_container(id, Some(StopContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> Result<()> {
        self.docker
            .restart_container(id, Some(RestartContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str) -> Result<()> {
        let opts = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        self.docker.remove_container(id, Some(opts)).await?;
        Ok(())
    }

    pub async fn list_images(&self) -> Result<Vec<ImageSummary>> {
        let opts = ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        };
        Ok(self.docker.list_images(Some(opts)).await?)
    }

    pub async fn list_volumes(&self) -> Result<Vec<Volume>> {
        let resp = self
            .docker
            .list_volumes(None::<ListVolumesOptions<String>>)
            .await?;
        Ok(resp.volumes.unwrap_or_default())
    }

    pub async fn prune_images(&self) -> Result<()> {
        let mut filters = HashMap::new();
        filters.insert("dangling", vec!["true"]);
        self.docker
            .prune_images(Some(PruneImagesOptions { filters }))
            .await?;
        Ok(())
    }

    pub async fn prune_volumes(&self) -> Result<()> {
        self.docker
            .prune_volumes(None::<PruneVolumesOptions<String>>)
            .await?;
        Ok(())
    }

    /// Get Docker system information. Used for future features.
    #[allow(dead_code)]
    pub async fn system_info(&self) -> Result<SystemInfo> {
        Ok(self.docker.info().await?)
    }
}
