use bollard::container::Stats;

pub struct StatsSnapshot {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_limit_mb: f64,
    pub net_rx_bytes: f64,
    pub net_tx_bytes: f64,
}

pub fn parse_stats(stats: &Stats) -> StatsSnapshot {
    let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 - stats.precpu_stats.cpu_usage.total_usage as f64;
    let system_delta =
        stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64 - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
    let cpu_percent = if system_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    };

    use bollard::container::MemoryStatsStats;
    let cache = match &stats.memory_stats.stats {
        Some(MemoryStatsStats::V1(v1)) => v1.cache as f64,
        _ => 0.0,
    };
    let memory_bytes = (stats.memory_stats.usage.unwrap_or(0) as f64 - cache).max(0.0);
    let memory_limit = stats.memory_stats.limit.unwrap_or(1) as f64;
    let memory_mb = memory_bytes / 1_048_576.0;
    let memory_limit_mb = memory_limit / 1_048_576.0;

    let (net_rx, net_tx) = stats
        .networks
        .as_ref()
        .map(|nets| {
            nets.values()
                .fold((0u64, 0u64), |(rx, tx), net| (rx + net.rx_bytes, tx + net.tx_bytes))
        })
        .unwrap_or((0, 0));

    StatsSnapshot {
        cpu_percent,
        memory_mb,
        memory_limit_mb,
        net_rx_bytes: net_rx as f64,
        net_tx_bytes: net_tx as f64,
    }
}
