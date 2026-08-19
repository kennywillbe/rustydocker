use crate::docker::client::DockerClient;
use crate::docker::model::container_state;
use bollard::models::ContainerSummary;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    Line {
        container_id: String,
        container_name: String,
        line: String,
    },
    StreamError {
        container_id: String,
        message: String,
    },
}

/// Owns one reconnecting Docker log task for every running container.
/// Events reach the UI through one channel, preserving arrival order.
pub struct LogManager {
    tasks: HashMap<String, JoinHandle<()>>,
    tx: mpsc::UnboundedSender<LogEvent>,
    rx: mpsc::UnboundedReceiver<LogEvent>,
}

impl Default for LogManager {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tasks: HashMap::new(),
            tx,
            rx,
        }
    }
}

impl LogManager {
    pub fn sync(&mut self, docker: &DockerClient, containers: &[ContainerSummary], tail: &str) {
        let desired: HashSet<String> = containers
            .iter()
            .filter(|container| container_state(container) == Some("running"))
            .filter_map(|container| container.id.clone())
            .collect();

        self.tasks.retain(|id, task| {
            if desired.contains(id) && !task.is_finished() {
                true
            } else {
                task.abort();
                false
            }
        });

        for container in containers {
            if container_state(container) != Some("running") {
                continue;
            }
            let Some(id) = container.id.clone() else { continue };
            if self.tasks.contains_key(&id) {
                continue;
            }
            let name = container
                .names
                .as_ref()
                .and_then(|names| names.first())
                .map(|name| name.trim_start_matches('/').to_string())
                .unwrap_or_else(|| id.chars().take(12).collect());
            let docker = docker.clone();
            let tx = self.tx.clone();
            let tail = tail.to_string();
            let task_id = id.clone();
            self.tasks.insert(
                id,
                tokio::spawn(async move {
                    let mut since = i32::try_from(chrono::Utc::now().timestamp()).unwrap_or(i32::MAX);
                    let mut seen_at_cursor = HashSet::new();
                    if let Ok(lines) = docker.container_logs_batch(&task_id, &tail).await {
                        for line in lines {
                            update_cursor(&line, &mut since, &mut seen_at_cursor);
                            let _ = tx.send(LogEvent::Line {
                                container_id: task_id.clone(),
                                container_name: name.clone(),
                                line,
                            });
                        }
                    }

                    loop {
                        let mut stream = docker.container_logs_follow_since(&task_id, since);
                        while let Some(result) = stream.next().await {
                            match result {
                                Ok(output) => {
                                    let line = output.to_string();
                                    if is_duplicate_at_cursor(&line, since, &seen_at_cursor) {
                                        continue;
                                    }
                                    update_cursor(&line, &mut since, &mut seen_at_cursor);
                                    let _ = tx.send(LogEvent::Line {
                                        container_id: task_id.clone(),
                                        container_name: name.clone(),
                                        line,
                                    });
                                }
                                Err(error) => {
                                    let _ = tx.send(LogEvent::StreamError {
                                        container_id: task_id.clone(),
                                        message: error.to_string(),
                                    });
                                    break;
                                }
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }),
            );
        }
    }

    pub async fn recv(&mut self) -> Option<LogEvent> {
        self.rx.recv().await
    }
}

fn log_timestamp(line: &str) -> Option<i32> {
    let timestamp = line.split_whitespace().next()?;
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
    i32::try_from(parsed.timestamp()).ok()
}

fn is_duplicate_at_cursor(line: &str, cursor: i32, seen: &HashSet<String>) -> bool {
    log_timestamp(line) == Some(cursor) && seen.contains(line)
}

fn update_cursor(line: &str, cursor: &mut i32, seen: &mut HashSet<String>) {
    let Some(timestamp) = log_timestamp(line) else { return };
    if timestamp > *cursor {
        *cursor = timestamp;
        seen.clear();
    }
    if timestamp == *cursor {
        seen.insert(line.to_string());
    }
}

impl Drop for LogManager {
    fn drop(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_cursor_deduplicates_same_timestamped_entry() {
        let mut cursor = 0;
        let mut seen = HashSet::new();
        let first = "2026-08-19T10:15:30.123456789Z first";
        update_cursor(first, &mut cursor, &mut seen);
        assert_eq!(
            cursor,
            i32::try_from(
                chrono::DateTime::parse_from_rfc3339("2026-08-19T10:15:30.123456789Z")
                    .unwrap()
                    .timestamp()
            )
            .unwrap()
        );
        assert!(is_duplicate_at_cursor(first, cursor, &seen));
        assert!(!is_duplicate_at_cursor(
            "2026-08-19T10:15:30.223456789Z second",
            cursor,
            &seen
        ));
    }

    #[test]
    fn reconnect_cursor_advances_and_forgets_old_second() {
        let mut cursor = 0;
        let mut seen = HashSet::new();
        update_cursor("2026-08-19T10:15:30Z first", &mut cursor, &mut seen);
        update_cursor("2026-08-19T10:15:31Z next", &mut cursor, &mut seen);
        assert_eq!(seen.len(), 1);
        assert!(seen.contains("2026-08-19T10:15:31Z next"));
    }
}
