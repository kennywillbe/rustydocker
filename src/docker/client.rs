use anyhow::Result;
use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, Stats,
    StatsOptions, StopContainerOptions, TopOptions,
};
use bollard::image::{ListImagesOptions, PruneImagesOptions};
use bollard::models::{
    ContainerInspectResponse, ContainerSummary, EventMessage, ImageSummary, Network, SystemInfo, Volume,
};
use bollard::network::{ListNetworksOptions, PruneNetworksOptions};
use bollard::system::EventsOptions;
use bollard::volume::{ListVolumesOptions, PruneVolumesOptions};
use bollard::Docker;
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new(docker_host: Option<&str>) -> Result<Self> {
        let docker = match docker_host {
            Some(host) if host.starts_with("unix://") => {
                Docker::connect_with_unix(host.trim_start_matches("unix://"), 120, bollard::API_DEFAULT_VERSION)?
            }
            Some(host) => Docker::connect_with_http(host, 120, bollard::API_DEFAULT_VERSION)?,
            None => Docker::connect_with_local_defaults()?,
        };
        Ok(Self { docker })
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerSummary>> {
        let opts = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        Ok(self.docker.list_containers(Some(opts)).await?)
    }

    /// Inspect a container by ID.
    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspectResponse> {
        Ok(self.docker.inspect_container(id, None).await?)
    }

    /// Fetch last N log lines (non-streaming, returns all at once)
    pub async fn container_logs_batch(&self, id: &str, tail: &str) -> Result<Vec<String>> {
        let opts = LogsOptions::<String> {
            follow: false,
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
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
    ) -> Pin<Box<dyn Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + Send + '_>> {
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

    #[allow(dead_code)]
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

    pub async fn pause_container(&self, id: &str) -> Result<()> {
        self.docker.pause_container(id).await?;
        Ok(())
    }

    pub async fn unpause_container(&self, id: &str) -> Result<()> {
        self.docker.unpause_container(id).await?;
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
        let resp = self.docker.list_volumes(None::<ListVolumesOptions<String>>).await?;
        Ok(resp.volumes.unwrap_or_default())
    }

    pub async fn prune_images(&self) -> Result<()> {
        let mut filters = HashMap::new();
        filters.insert("dangling", vec!["true"]);
        self.docker.prune_images(Some(PruneImagesOptions { filters })).await?;
        Ok(())
    }

    pub async fn prune_volumes(&self) -> Result<()> {
        self.docker.prune_volumes(None::<PruneVolumesOptions<String>>).await?;
        Ok(())
    }

    pub fn docker_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, bollard::errors::Error>> + Send + '_>> {
        let opts = EventsOptions::<String> { ..Default::default() };
        Box::pin(self.docker.events(Some(opts)))
    }

    /// List processes running inside a container.
    pub async fn container_top(&self, id: &str) -> Result<Vec<Vec<String>>> {
        let top = self
            .docker
            .top_processes(id, Some(TopOptions { ps_args: "aux" }))
            .await?;
        let mut rows = vec![];
        if let Some(titles) = top.titles {
            rows.push(titles);
        }
        if let Some(processes) = top.processes {
            rows.extend(processes);
        }
        Ok(rows)
    }

    pub async fn list_networks(&self) -> Result<Vec<Network>> {
        Ok(self.docker.list_networks(None::<ListNetworksOptions<String>>).await?)
    }

    #[allow(dead_code)]
    pub async fn remove_network(&self, id: &str) -> Result<()> {
        self.docker.remove_network(id).await?;
        Ok(())
    }

    pub async fn prune_networks(&self) -> Result<()> {
        self.docker.prune_networks(None::<PruneNetworksOptions<String>>).await?;
        Ok(())
    }

    /// Fetch a single stats snapshot for a container (non-streaming).
    pub async fn container_stats_oneshot(&self, id: &str) -> Result<Stats> {
        let opts = StatsOptions {
            stream: false,
            one_shot: true,
        };
        let mut stream = self.docker.stats(id, Some(opts));
        stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No stats response"))?
            .map_err(|e| e.into())
    }

    /// Get Docker system information. Used for future features.
    #[allow(dead_code)]
    pub async fn system_info(&self) -> Result<SystemInfo> {
        Ok(self.docker.info().await?)
    }
}
