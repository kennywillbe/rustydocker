use super::*;
use futures_util::stream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const FRAMES: &str = include_str!("../../../tests/fixtures/stats/stream-frames.jsonl");

const FAST: Timing = Timing {
    reconnect: Duration::from_millis(10),
    stall: Duration::from_millis(40),
};

#[derive(Clone, Copy)]
enum End {
    Close,
    Silent,
    Fail,
}

/// Replays the same frames on every connect, then ends as told. `open`
/// counts streams that are still alive, so an aborted task shows up as
/// a dropped stream rather than as missing samples.
#[derive(Clone)]
struct Scripted {
    connects: Arc<AtomicUsize>,
    open: Arc<AtomicUsize>,
    frames: Vec<ContainerStatsResponse>,
    end: End,
}

impl Scripted {
    fn new(frames: Vec<ContainerStatsResponse>, end: End) -> Self {
        Self {
            connects: Arc::new(AtomicUsize::new(0)),
            open: Arc::new(AtomicUsize::new(0)),
            frames,
            end,
        }
    }
}

struct OpenStream(Arc<AtomicUsize>);

impl Drop for OpenStream {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl StatsSource for Scripted {
    fn connect(&self) -> StatsStream<'_> {
        self.connects.fetch_add(1, Ordering::SeqCst);
        self.open.fetch_add(1, Ordering::SeqCst);
        let guard = OpenStream(self.open.clone());
        let frames = stream::iter(self.frames.clone().into_iter().map(Ok));
        let tail: StatsStream<'static> = match self.end {
            End::Close => Box::pin(stream::empty()),
            End::Silent => Box::pin(stream::pending()),
            End::Fail => Box::pin(
                stream::iter([Err(bollard::errors::Error::DockerResponseServerError {
                    status_code: 500,
                    message: "boom".to_string(),
                })])
                .chain(stream::pending()),
            ),
        };
        Box::pin(frames.chain(tail).map(move |item| {
            let _alive = &guard;
            item
        }))
    }
}

fn frames() -> Vec<ContainerStatsResponse> {
    FRAMES
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture frame"))
        .collect()
}

async fn connects_within(end: End, timing: Timing, window: Duration) -> usize {
    let source = Scripted::new(vec![], end);
    let connects = source.connects.clone();
    let (tx, _rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(pump(source, "c".to_string(), 1, tx, timing));
    tokio::time::sleep(window).await;
    task.abort();
    connects.load(Ordering::SeqCst)
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(30)).await;
}

fn ids(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

#[tokio::test]
async fn ended_stream_reconnects_after_a_pause() {
    let connects = connects_within(End::Close, FAST, Duration::from_millis(105)).await;
    // The pause keeps it from spinning: at most one connect per 10 ms.
    assert!((3..=12).contains(&connects), "{connects}");
}

#[tokio::test]
async fn silent_stream_is_dropped_after_the_stall_timeout() {
    let connects = connects_within(End::Silent, FAST, Duration::from_millis(230)).await;
    // One round is 40 ms of silence plus the 10 ms pause.
    assert!((2..=5).contains(&connects), "{connects}");

    let patient = Timing {
        stall: Duration::from_secs(5),
        ..FAST
    };
    assert_eq!(
        connects_within(End::Silent, patient, Duration::from_millis(100)).await,
        1
    );
}

#[tokio::test]
async fn pump_drops_the_first_frame_and_tags_samples() {
    let source = Scripted::new(frames(), End::Silent);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(pump(source, "c".to_string(), 7, tx, FAST));
    let mut cpu = Vec::new();
    for _ in 0..2 {
        let (generation, event) = rx.recv().await.expect("sample");
        assert_eq!((generation, event.container_id.as_str()), (7, "c"));
        cpu.push(event.sample.cpu_percent);
    }
    task.abort();
    assert!((cpu[0] - 100.2114).abs() < 0.001, "{cpu:?}");
    assert!((cpu[1] - 99.9710).abs() < 0.001, "{cpu:?}");
}

#[tokio::test]
async fn recv_skips_samples_from_an_earlier_or_forgotten_stream() {
    let mut manager = StatsManager::default();
    manager.tasks.insert(
        "c".to_string(),
        StatsTask {
            generation: 2,
            handle: tokio::spawn(async {}),
        },
    );
    let sample = StatsSample {
        cpu_percent: 0.0,
        memory_mb: 0.0,
        memory_limit_mb: 0.0,
        net_rx_per_sec: 0.0,
        net_tx_per_sec: 0.0,
    };
    let event = |cpu_percent| StatsEvent {
        container_id: "c".to_string(),
        sample: StatsSample { cpu_percent, ..sample },
    };

    manager.tx.send((1, event(1.0))).unwrap();
    manager.tx.send((2, event(2.0))).unwrap();
    assert_eq!(manager.recv().await.map(|e| e.sample.cpu_percent), Some(2.0));

    manager.tx.send((2, event(3.0))).unwrap();
    manager.forget("c");
    let after_forget = tokio::time::timeout(Duration::from_millis(50), manager.recv()).await;
    assert!(after_forget.is_err(), "a forgotten stream's sample got through");
}

#[tokio::test]
async fn failed_stream_reconnects_without_waiting_for_the_stall_timeout() {
    let patient = Timing {
        stall: Duration::from_secs(5),
        ..FAST
    };
    let connects = connects_within(End::Fail, patient, Duration::from_millis(105)).await;
    assert!((3..=12).contains(&connects), "{connects}");
}

#[tokio::test]
async fn sync_keeps_one_stream_per_running_container() {
    let source = Scripted::new(vec![], End::Silent);
    let mut manager = StatsManager::default();

    manager.sync_running(ids(&["a", "b"]), |_| source.clone());
    manager.sync_running(ids(&["a", "b"]), |_| source.clone());
    settle().await;
    assert_eq!(
        source.open.load(Ordering::SeqCst),
        2,
        "a repeated sync opened a duplicate"
    );

    manager.sync_running(ids(&["a"]), |_| source.clone());
    settle().await;
    assert_eq!(
        source.open.load(Ordering::SeqCst),
        1,
        "the stopped container's stream leaked"
    );

    manager.forget("a");
    settle().await;
    assert_eq!(source.open.load(Ordering::SeqCst), 0, "forget left the stream open");

    manager.sync_running(ids(&["a"]), |_| source.clone());
    drop(manager);
    settle().await;
    assert_eq!(source.open.load(Ordering::SeqCst), 0, "drop left a stream open");
}

#[test]
fn only_die_and_pause_reset_the_graph() {
    for action in ["die", "pause"] {
        assert!(resets_stats(action), "{action}");
    }
    // kill can be a non-fatal signal; a fatal one is followed by die.
    for action in [
        "start", "restart", "unpause", "kill", "stop", "rename", "exec_die", "oom",
    ] {
        assert!(!resets_stats(action), "{action}");
    }
}
