use bollard::models::ContainerStatsResponse;

pub struct StatsSnapshot {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub memory_limit_mb: f64,
    pub net_rx_bytes: f64,
    pub net_tx_bytes: f64,
}

pub fn parse_stats(stats: &ContainerStatsResponse) -> StatsSnapshot {
    let cpu_total = stats
        .cpu_stats
        .as_ref()
        .and_then(|cpu| cpu.cpu_usage.as_ref())
        .and_then(|usage| usage.total_usage)
        .unwrap_or(0);
    let precpu_total = stats
        .precpu_stats
        .as_ref()
        .and_then(|cpu| cpu.cpu_usage.as_ref())
        .and_then(|usage| usage.total_usage)
        .unwrap_or(0);
    let system_cpu = stats
        .cpu_stats
        .as_ref()
        .and_then(|cpu| cpu.system_cpu_usage)
        .unwrap_or(0);
    let presystem_cpu = stats
        .precpu_stats
        .as_ref()
        .and_then(|cpu| cpu.system_cpu_usage)
        .unwrap_or(0);
    let cpu_delta = cpu_total.saturating_sub(precpu_total) as f64;
    let system_delta = system_cpu.saturating_sub(presystem_cpu) as f64;
    let num_cpus = stats
        .cpu_stats
        .as_ref()
        .and_then(|cpu| cpu.online_cpus)
        .or(stats.num_procs)
        .unwrap_or(1) as f64;
    let cpu_percent = if system_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    };

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
        cpu_percent,
        memory_mb,
        memory_limit_mb,
        net_rx_bytes: net_rx as f64,
        net_tx_bytes: net_tx as f64,
    }
}
