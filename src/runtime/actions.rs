use crate::docker::compose::ComposeInvocation;
use anyhow::{bail, Result};
use std::process::Stdio;
use tokio::process::Child;
use tokio::sync::mpsc;

use crate::docker::client::DockerClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub message: String,
    pub refresh_images: bool,
}

/// Executes project-level operations and owns long-lived child processes.
pub struct ActionExecutor {
    compose: ComposeInvocation,
    compose_watch: Option<Child>,
    tx: mpsc::UnboundedSender<Result<ActionOutcome, String>>,
    rx: mpsc::UnboundedReceiver<Result<ActionOutcome, String>>,
}

impl ActionExecutor {
    pub fn new(compose: ComposeInvocation) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            compose,
            compose_watch: None,
            tx,
            rx,
        }
    }

    pub async fn compose(&self, args: &[&str]) -> Result<()> {
        let output = self.compose.run(args).await?;
        if output.status.success() {
            Ok(())
        } else {
            bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
        }
    }

    pub fn spawn_compose(&self, args: Vec<&'static str>, success: &'static str, refresh_images: bool) {
        let compose = self.compose.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = match compose.run(&args).await {
                Ok(output) if output.status.success() => Ok(ActionOutcome {
                    message: success.to_string(),
                    refresh_images,
                }),
                Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = tx.send(result);
        });
    }

    pub fn spawn_image_pull(&self, docker: DockerClient, reference: String) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = docker
                .pull_image(&reference)
                .await
                .map(|()| ActionOutcome {
                    message: format!("Pulled {reference}"),
                    refresh_images: true,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    pub async fn recv(&mut self) -> Option<Result<ActionOutcome, String>> {
        self.rx.recv().await
    }

    pub async fn toggle_compose_watch(&mut self) -> Result<bool> {
        if let Some(mut child) = self.compose_watch.take() {
            child.kill().await?;
            return Ok(false);
        }
        let mut command = self.compose.command(&["watch"]);
        command.stdout(Stdio::null()).stderr(Stdio::null());
        self.compose_watch = Some(command.spawn()?);
        Ok(true)
    }

    pub fn poll_compose_watch(&mut self) -> Result<Option<bool>> {
        let Some(child) = self.compose_watch.as_mut() else {
            return Ok(None);
        };
        if child.try_wait()?.is_some() {
            self.compose_watch = None;
            Ok(Some(false))
        } else {
            Ok(Some(true))
        }
    }

    pub async fn shutdown(&mut self) {
        if let Some(mut child) = self.compose_watch.take() {
            let _ = child.kill().await;
        }
    }
}
