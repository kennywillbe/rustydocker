use anyhow::Result;
use bollard::models::{
    ContainerInspectResponse, ContainerStatsResponse, ContainerSummary, EventMessage, ImageHistoryResponseItem,
    ImageInspect, ImageSummary, Network, SystemInfo, Volume,
};
use bollard::query_parameters::{
    CreateImageOptions, EventsOptions, ListContainersOptions, ListImagesOptions, ListNetworksOptions,
    ListVolumesOptions, LogsOptions, PruneImagesOptions, PruneNetworksOptions, PruneVolumesOptions,
    RemoveContainerOptions, RemoveImageOptions, RestartContainerOptions, StartContainerOptions, StatsOptions,
    StopContainerOptions, TopOptions,
};
use bollard::Docker;
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;

use super::connection::ConnectionSpec;

#[derive(Clone)]
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new(connection: &ConnectionSpec) -> Result<Self> {
        let docker = match connection {
            ConnectionSpec::Local => Docker::connect_with_local_defaults()?,
            ConnectionSpec::Unix { socket } => {
                Docker::connect_with_unix(socket.trim_start_matches("unix://"), 120, bollard::API_DEFAULT_VERSION)?
            }
            ConnectionSpec::Http { host } => Docker::connect_with_http(host, 120, bollard::API_DEFAULT_VERSION)?,
            ConnectionSpec::Tls { host, cert_path } => Docker::connect_with_ssl(
                host,
                Path::new(&cert_path.join("key.pem")),
                Path::new(&cert_path.join("cert.pem")),
                Path::new(&cert_path.join("ca.pem")),
                120,
                bollard::API_DEFAULT_VERSION,
            )?,
            ConnectionSpec::Ssh { host, key_path } => {
                Docker::connect_with_ssh(host, 120, bollard::API_DEFAULT_VERSION, key_path.clone())?
            }
        };
        Ok(Self { docker })
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerSummary>> {
        let opts = ListContainersOptions {
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
        let opts = LogsOptions {
            follow: false,
            stdout: true,
            stderr: true,
            timestamps: true,
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
        self.container_logs_follow_since(id, i32::try_from(chrono::Utc::now().timestamp()).unwrap_or(i32::MAX))
    }

    pub fn container_logs_follow_since(
        &self,
        id: &str,
        since: i32,
    ) -> Pin<Box<dyn Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>> + Send + '_>> {
        let opts = LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            timestamps: true,
            since,
            tail: "0".to_string(),
            ..Default::default()
        };
        Box::pin(self.docker.logs(id, Some(opts)))
    }

    #[allow(dead_code)]
    pub fn container_stats(
        &self,
        id: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<ContainerStatsResponse, bollard::errors::Error>> + Send + '_>> {
        let opts = StatsOptions {
            stream: true,
            one_shot: false,
        };
        Box::pin(self.docker.stats(id, Some(opts)))
    }

    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.docker.start_container(id, None::<StartContainerOptions>).await?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> Result<()> {
        self.docker
            .stop_container(
                id,
                Some(StopContainerOptions {
                    t: Some(10),
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> Result<()> {
        self.docker
            .restart_container(
                id,
                Some(RestartContainerOptions {
                    t: Some(10),
                    ..Default::default()
                }),
            )
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
        let opts = ListImagesOptions {
            all: true,
            ..Default::default()
        };
        Ok(self.docker.list_images(Some(opts)).await?)
    }

    pub async fn inspect_image(&self, image: &str) -> Result<ImageInspect> {
        Ok(self.docker.inspect_image(image).await?)
    }

    pub async fn image_history(&self, image: &str) -> Result<Vec<ImageHistoryResponseItem>> {
        Ok(self.docker.image_history(image).await?)
    }

    pub async fn pull_image(&self, reference: &str) -> Result<()> {
        let options = CreateImageOptions {
            from_image: Some(reference.to_string()),
            ..Default::default()
        };
        let mut stream = self.docker.create_image(Some(options), None, None);
        while let Some(progress) = stream.next().await {
            progress?;
        }
        Ok(())
    }

    pub async fn remove_image(&self, image: &str, force: bool) -> Result<()> {
        self.docker
            .remove_image(
                image,
                Some(RemoveImageOptions {
                    force,
                    ..Default::default()
                }),
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn list_volumes(&self) -> Result<Vec<Volume>> {
        let resp = self.docker.list_volumes(None::<ListVolumesOptions>).await?;
        Ok(resp.volumes.unwrap_or_default())
    }

    pub async fn prune_images(&self) -> Result<()> {
        let mut filters = HashMap::new();
        filters.insert("dangling", vec!["true"]);
        self.docker
            .prune_images(Some(PruneImagesOptions {
                filters: Some(
                    filters
                        .into_iter()
                        .map(|(key, values)| (key.to_string(), values.into_iter().map(str::to_string).collect()))
                        .collect(),
                ),
            }))
            .await?;
        Ok(())
    }

    pub async fn prune_volumes(&self) -> Result<()> {
        self.docker.prune_volumes(None::<PruneVolumesOptions>).await?;
        Ok(())
    }

    pub fn docker_events(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<EventMessage, bollard::errors::Error>> + Send + '_>> {
        let opts = EventsOptions::default();
        Box::pin(self.docker.events(Some(opts)))
    }

    /// List processes running inside a container.
    pub async fn container_top(&self, id: &str) -> Result<Vec<Vec<String>>> {
        let top = self
            .docker
            .top_processes(
                id,
                Some(TopOptions {
                    ps_args: "aux".to_string(),
                }),
            )
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
        Ok(self.docker.list_networks(None::<ListNetworksOptions>).await?)
    }

    #[allow(dead_code)]
    pub async fn remove_network(&self, id: &str) -> Result<()> {
        self.docker.remove_network(id).await?;
        Ok(())
    }

    pub async fn prune_networks(&self) -> Result<()> {
        self.docker.prune_networks(None::<PruneNetworksOptions>).await?;
        Ok(())
    }

    /// Fetch a single stats snapshot for a container (non-streaming).
    pub async fn container_stats_oneshot(&self, id: &str) -> Result<ContainerStatsResponse> {
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
