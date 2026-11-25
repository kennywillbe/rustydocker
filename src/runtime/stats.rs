use crate::docker::client::DockerClient;
use crate::docker::model::container_state;
use crate::docker::stats::{StatsSample, StatsSampler};
use bollard::models::{ContainerStatsResponse, ContainerSummary};
use futures_util::{Stream, StreamExt};
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq)]
pub struct StatsEvent {
    pub container_id: String,
    pub sample: StatsSample,
}

type StatsStream<'a> = Pin<Box<dyn Stream<Item = Result<ContainerStatsResponse, bollard::errors::Error>> + Send + 'a>>;

/// Where a stats task gets its frames from; a seam so the reconnect and stall
/// handling can be tested without a Docker daemon.
trait StatsSource {
    fn connect(&self) -> StatsStream<'_>;
}

struct ContainerStats {
    docker: DockerClient,
    container_id: String,
}

impl StatsSource for ContainerStats {
    fn connect(&self) -> StatsStream<'_> {
        self.docker.container_stats(&self.container_id)
    }
}

/// Container events after which the graph must start over. `die` is the one
/// event that stop, crash and restart all emit.
pub fn resets_stats(event_action: &str) -> bool {
    matches!(event_action, "die" | "pause")
}

#[derive(Debug, Clone, Copy)]
struct Timing {
    reconnect: Duration,
    // Docker sends a frame every second, so silence this long means a
    // half-open connection (laptop sleep, network switch) that never ends.
    stall: Duration,
}

const TIMING: Timing = Timing {
    reconnect: Duration::from_millis(500),
    stall: Duration::from_secs(5),
};

struct StatsTask {
    generation: u64,
    handle: JoinHandle<()>,
}

/// Owns one reconnecting Docker stats stream for every running container, so
/// the UI loop only ever receives finished samples and never waits on Docker.
pub struct StatsManager {
    tasks: HashMap<String, StatsTask>,
    next_generation: u64,
    tx: mpsc::UnboundedSender<(u64, StatsEvent)>,
    rx: mpsc::UnboundedReceiver<(u64, StatsEvent)>,
}

impl Default for StatsManager {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tasks: HashMap::new(),
            next_generation: 0,
            tx,
            rx,
        }
    }
}

impl StatsManager {
    pub fn sync(&mut self, docker: &DockerClient, containers: &[ContainerSummary]) {
        let running: HashSet<String> = containers
            .iter()
            .filter(|container| container_state(container) == Some("running"))
            .filter_map(|container| container.id.clone())
            .collect();
        self.sync_running(running, |id| ContainerStats {
            docker: docker.clone(),
            container_id: id.to_string(),
        });
    }

    fn sync_running<S>(&mut self, running: HashSet<String>, mut source_for: impl FnMut(&str) -> S)
    where
        S: StatsSource + Send + Sync + 'static,
    {
        self.tasks.retain(|id, task| {
            if running.contains(id) && !task.handle.is_finished() {
                true
            } else {
                task.handle.abort();
                false
            }
        });

        for id in running {
            if self.tasks.contains_key(&id) {
                continue;
            }
            let source = source_for(&id);
            self.spawn(id, source);
        }
    }

    /// Stops the container's stream right away; the next sync opens a fresh
    /// one if it is running again (a restart).
    pub fn forget(&mut self, container_id: &str) {
        if let Some(task) = self.tasks.remove(container_id) {
            task.handle.abort();
        }
    }

    pub async fn recv(&mut self) -> Option<StatsEvent> {
        loop {
            let (generation, event) = self.rx.recv().await?;
            // Samples queued by a task that was since aborted or replaced
            // belong to the container's previous life and would leak into
            // the fresh graph.
            let current = self.tasks.get(&event.container_id).map(|task| task.generation);
            if current == Some(generation) {
                return Some(event);
            }
        }
    }

    fn spawn(&mut self, container_id: String, source: impl StatsSource + Send + Sync + 'static) {
        self.next_generation += 1;
        let generation = self.next_generation;
        let handle = tokio::spawn(pump(source, container_id.clone(), generation, self.tx.clone(), TIMING));
        self.tasks.insert(container_id, StatsTask { generation, handle });
    }
}

async fn pump(
    source: impl StatsSource,
    container_id: String,
    generation: u64,
    tx: mpsc::UnboundedSender<(u64, StatsEvent)>,
    timing: Timing,
) {
    loop {
        // A fresh sampler per connection: Docker's first frame of every stream
        // has no previous sample, and counters may have reset in between.
        let mut sampler = StatsSampler::default();
        let mut stream = source.connect();
        while let Ok(Some(Ok(frame))) = tokio::time::timeout(timing.stall, stream.next()).await {
            let Some(sample) = sampler.sample(&frame) else { continue };
            let event = StatsEvent {
                container_id: container_id.clone(),
                sample,
            };
            if tx.send((generation, event)).is_err() {
                return;
            }
        }
        // Not surfaced on the status line: the loop recovers on its own, and a
        // stopped or restarted container keeps its stream open (zeroed or
        // reset frames that the sampler drops) rather than failing it.
        tokio::time::sleep(timing.reconnect).await;
    }
}

impl Drop for StatsManager {
    fn drop(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.handle.abort();
        }
    }
}

#[cfg(test)]
mod tests;
