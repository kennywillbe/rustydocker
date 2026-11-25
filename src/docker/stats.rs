use bollard::models::ContainerStatsResponse;
use chrono::{DateTime, FixedOffset};

pub struct StatsSnapshot {
    /// 0.0 when the frame carries no previous sample; `cpu_percent()` tells
    /// that case apart from a genuinely idle container.
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_limit_mb: f64,
    pub net_rx_bytes: f64,
    pub net_tx_bytes: f64,
}

pub fn parse_stats(stats: &ContainerStatsResponse) -> StatsSnapshot {
    let memory_stats = stats.memory_stats.as_ref();
    let cache = memory_stats
        .and_then(|memory| memory.stats.as_ref())
        .and_then(|values| {
            values
                .get("total_inactive_file")
                .or_else(|| values.get("inactive_file"))
                .or_else(|| values.get("cache"))
        })
        .copied()
        .unwrap_or(0);
    let memory_usage = memory_stats.and_then(|memory| memory.usage).unwrap_or(0);
    let memory_bytes = memory_usage.saturating_sub(cache) as f64;
    let memory_limit = memory_stats.and_then(|memory| memory.limit).unwrap_or(1) as f64;
    let memory_mb = memory_bytes / 1_048_576.0;
    let memory_limit_mb = memory_limit / 1_048_576.0;

    let (net_rx, net_tx) = stats
        .networks
        .as_ref()
        .map(|nets| {
            nets.values().fold((0u64, 0u64), |(rx, tx), net| {
                (
                    rx.saturating_add(net.rx_bytes.unwrap_or(0)),
                    tx.saturating_add(net.tx_bytes.unwrap_or(0)),
                )
            })
        })
        .unwrap_or((0, 0));

    StatsSnapshot {
        cpu_percent: cpu_percent(stats).unwrap_or(0.0),
        memory_mb,
        memory_limit_mb,
        net_rx_bytes: net_rx as f64,
        net_tx_bytes: net_tx as f64,
    }
}

/// Docker CLI's formula over the frame's own `precpu_stats`. `None` when that
/// previous sample is missing (one-shot responses and the first frame of every
/// stream) or the counters went backwards, because dividing lifetime totals
/// would report the container's average since start instead of its current load.
pub fn cpu_percent(stats: &ContainerStatsResponse) -> Option<f64> {
    let cpu = stats.cpu_stats.as_ref()?;
    let precpu = stats.precpu_stats.as_ref()?;
    let total = cpu.cpu_usage.as_ref()?.total_usage?;
    let pre_total = precpu.cpu_usage.as_ref()?.total_usage?;
    let system = cpu.system_cpu_usage?;
    let pre_system = precpu.system_cpu_usage.filter(|&usage| usage > 0)?;

    let cpu_delta = total.checked_sub(pre_total)?;
    let system_delta = system.checked_sub(pre_system).filter(|&delta| delta > 0)?;
    // Same fallback as Docker CLI. num_procs is no substitute: Linux always
    // reports it as 0, which would turn every reading into a fake 0%.
    let online_cpus = cpu
        .online_cpus
        .filter(|&count| count > 0)
        .map(f64::from)
        .or_else(|| {
            let per_cpu = cpu.cpu_usage.as_ref()?.percpu_usage.as_ref()?.len();
            (per_cpu > 0).then_some(per_cpu as f64)
        })
        .unwrap_or(1.0);
    Some((cpu_delta as f64 / system_delta as f64) * online_cpus * 100.0)
}

/// Lifetime network totals at the instant Docker read them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetCounters {
    pub read: DateTime<FixedOffset>,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
}

pub fn net_counters(stats: &ContainerStatsResponse) -> Option<NetCounters> {
    let read = DateTime::parse_from_rfc3339(stats.read.as_deref()?).ok()?;
    let snapshot = parse_stats(stats);
    Some(NetCounters {
        read,
        rx_bytes: snapshot.net_rx_bytes,
        tx_bytes: snapshot.net_tx_bytes,
    })
}

/// Bytes per second between two frames. The elapsed time comes from Docker's
/// own `read` stamps so delivery jitter on our side cannot skew the rate.
pub fn net_rates(previous: &NetCounters, current: &NetCounters) -> Option<(f64, f64)> {
    let elapsed_ns = (current.read - previous.read).num_nanoseconds()?;
    if elapsed_ns <= 0 {
        return None;
    }
    let rx_delta = current.rx_bytes - previous.rx_bytes;
    let tx_delta = current.tx_bytes - previous.tx_bytes;
    if rx_delta < 0.0 || tx_delta < 0.0 {
        return None;
    }
    let elapsed = elapsed_ns as f64 / 1e9;
    Some((rx_delta / elapsed, tx_delta / elapsed))
}

/// One point for the stats graphs, with network traffic as a rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsSample {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_limit_mb: f64,
    pub net_rx_per_sec: f64,
    pub net_tx_per_sec: f64,
}

/// Turns consecutive frames of one stats stream into graph samples. Docker
/// only reports lifetime network totals, so the previous frame is kept here.
#[derive(Debug, Default)]
pub struct StatsSampler {
    previous: Option<NetCounters>,
}

impl StatsSampler {
    /// `None` means "no point to draw", never "zero": a frame without a usable
    /// baseline is skipped so a reconnect or restart cannot plot a fake dip.
    pub fn sample(&mut self, stats: &ContainerStatsResponse) -> Option<StatsSample> {
        let current = net_counters(stats);
        // Even a skipped frame becomes the baseline, otherwise a restart would
        // compare every later frame with the old, higher counters.
        let previous = std::mem::replace(&mut self.previous, current);
        let cpu_percent = cpu_percent(stats)?;
        let (net_rx_per_sec, net_tx_per_sec) = net_rates(&previous?, &current?)?;
        let snapshot = parse_stats(stats);
        Some(StatsSample {
            cpu_percent,
            memory_mb: snapshot.memory_mb,
            memory_limit_mb: snapshot.memory_limit_mb,
            net_rx_per_sec,
            net_tx_per_sec,
        })
    }
}
