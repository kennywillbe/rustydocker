use bollard::container::Stats;
use rustydocker::docker::stats::parse_stats;

fn make_stats_json(
    cpu_total: u64,
    precpu_total: u64,
    system_cpu: u64,
    presystem_cpu: u64,
    online_cpus: u64,
    mem_usage: u64,
    mem_limit: u64,
    net_rx: u64,
    net_tx: u64,
) -> String {
    format!(
        r#"{{
  "read": "2024-01-01T00:00:00Z",
  "preread": "2024-01-01T00:00:00Z",
  "num_procs": 0,
  "pids_stats": {{}},
  "networks": {{
    "eth0": {{
      "rx_bytes": {net_rx},
      "rx_packets": 0,
      "rx_errors": 0,
      "rx_dropped": 0,
      "tx_bytes": {net_tx},
      "tx_packets": 0,
      "tx_errors": 0,
      "tx_dropped": 0
    }}
  }},
  "memory_stats": {{
    "usage": {mem_usage},
    "limit": {mem_limit}
  }},
  "blkio_stats": {{}},
  "cpu_stats": {{
    "cpu_usage": {{
      "total_usage": {cpu_total},
      "usage_in_usermode": 0,
      "usage_in_kernelmode": 0
    }},
    "system_cpu_usage": {system_cpu},
    "online_cpus": {online_cpus},
    "throttling_data": {{
      "periods": 0,
      "throttled_periods": 0,
      "throttled_time": 0
    }}
  }},
  "precpu_stats": {{
    "cpu_usage": {{
      "total_usage": {precpu_total},
      "usage_in_usermode": 0,
      "usage_in_kernelmode": 0
    }},
    "system_cpu_usage": {presystem_cpu},
    "throttling_data": {{
      "periods": 0,
      "throttled_periods": 0,
      "throttled_time": 0
    }}
  }},
  "storage_stats": {{}}
}}"#
    )
}

fn make_stats(
    cpu_total: u64,
    precpu_total: u64,
    system_cpu: u64,
    presystem_cpu: u64,
    online_cpus: u64,
    mem_usage: u64,
    mem_limit: u64,
    net_rx: u64,
    net_tx: u64,
) -> Stats {
    let json = make_stats_json(
        cpu_total,
        precpu_total,
        system_cpu,
        presystem_cpu,
        online_cpus,
        mem_usage,
        mem_limit,
        net_rx,
        net_tx,
    );
    serde_json::from_str(&json).expect("Failed to parse mock Stats JSON")
}

#[test]
fn test_parse_stats_cpu_percent() {
    // cpu_delta = 100, system_delta = 1000, 4 cpus => (100/1000)*4*100 = 40%
    let stats = make_stats(200, 100, 2000, 1000, 4, 0, 1, 0, 0);
    let snap = parse_stats(&stats);
    assert!((snap.cpu_percent - 40.0).abs() < 0.001);
}

#[test]
fn test_parse_stats_zero_system_delta() {
    // system_delta = 0 => cpu_percent should be 0
    let stats = make_stats(200, 100, 1000, 1000, 4, 0, 1, 0, 0);
    let snap = parse_stats(&stats);
    assert_eq!(snap.cpu_percent, 0.0);
}

#[test]
fn test_parse_stats_memory() {
    // 10 MB = 10 * 1048576 bytes
    let mem_bytes: u64 = 10 * 1_048_576;
    let mem_limit: u64 = 1024 * 1_048_576;
    let stats = make_stats(0, 0, 0, 0, 1, mem_bytes, mem_limit, 0, 0);
    let snap = parse_stats(&stats);
    assert!((snap.memory_mb - 10.0).abs() < 0.001);
    assert!((snap.memory_limit_mb - 1024.0).abs() < 0.001);
}

#[test]
fn test_parse_stats_network() {
    let stats = make_stats(0, 0, 0, 0, 1, 0, 1, 5000, 3000);
    let snap = parse_stats(&stats);
    assert_eq!(snap.net_rx_bytes, 5000.0);
    assert_eq!(snap.net_tx_bytes, 3000.0);
}

#[test]
fn test_parse_stats_no_networks() {
    let json = r#"{
  "read": "2024-01-01T00:00:00Z",
  "preread": "2024-01-01T00:00:00Z",
  "num_procs": 0,
  "pids_stats": {},
  "memory_stats": {},
  "blkio_stats": {},
  "cpu_stats": {
    "cpu_usage": {"total_usage": 0, "usage_in_usermode": 0, "usage_in_kernelmode": 0},
    "throttling_data": {"periods": 0, "throttled_periods": 0, "throttled_time": 0}
  },
  "precpu_stats": {
    "cpu_usage": {"total_usage": 0, "usage_in_usermode": 0, "usage_in_kernelmode": 0},
    "throttling_data": {"periods": 0, "throttled_periods": 0, "throttled_time": 0}
  },
  "storage_stats": {}
}"#;
    let stats: Stats = serde_json::from_str(json).unwrap();
    let snap = parse_stats(&stats);
    assert_eq!(snap.net_rx_bytes, 0.0);
    assert_eq!(snap.net_tx_bytes, 0.0);
    assert_eq!(snap.memory_mb, 0.0);
}

#[test]
fn test_parse_stats_single_cpu() {
    // cpu_delta = 50, system_delta = 500, 1 cpu => (50/500)*1*100 = 10%
    let stats = make_stats(150, 100, 1500, 1000, 1, 0, 1, 0, 0);
    let snap = parse_stats(&stats);
    assert!((snap.cpu_percent - 10.0).abs() < 0.001);
}
