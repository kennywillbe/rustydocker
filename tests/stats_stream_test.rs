use bollard::models::ContainerStatsResponse;
use rustydocker::docker::stats::{cpu_percent, net_counters, net_rates, StatsSampler};
use serde_json::{json, Value};

/// Three consecutive frames from a real `stream=true` response for a container
/// busy-looping one core on an 8-CPU host.
const STREAM_FRAMES: &str = include_str!("fixtures/stats/stream-frames.jsonl");

fn raw_frames() -> Vec<Value> {
    STREAM_FRAMES
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture line is JSON"))
        .collect()
}

fn frame(value: Value) -> ContainerStatsResponse {
    serde_json::from_value(value).expect("fixture matches ContainerStatsResponse")
}

fn frames() -> Vec<ContainerStatsResponse> {
    raw_frames().into_iter().map(frame).collect()
}

/// Shape of a one-shot response and of the first frame of every stream:
/// precpu is zeroed and has no system usage, so there is nothing to diff against.
fn zeroed_precpu_frame() -> ContainerStatsResponse {
    frame(json!({
        "read": "2026-11-24T22:50:00.000000000Z",
        "preread": "0001-01-01T00:00:00Z",
        "cpu_stats": {
            "cpu_usage": {"total_usage": 3097516000u64, "usage_in_kernelmode": 0, "usage_in_usermode": 0},
            "system_cpu_usage": 35430000000u64,
            "online_cpus": 8
        },
        "precpu_stats": {
            "cpu_usage": {"total_usage": 0, "usage_in_kernelmode": 0, "usage_in_usermode": 0},
            "throttling_data": {"periods": 0, "throttled_periods": 0, "throttled_time": 0}
        },
        "memory_stats": {"usage": 5255168, "limit": 8221798400u64},
        "networks": {"eth0": {"rx_bytes": 1512, "tx_bytes": 126}}
    }))
}

#[test]
fn zeroed_precpu_yields_no_cpu_sample() {
    // The old code divided lifetime totals here and reported ~70%.
    let stats = zeroed_precpu_frame();
    assert_eq!(cpu_percent(&stats), None);

    let mut sampler = StatsSampler::default();
    assert_eq!(sampler.sample(&stats), None);
}

#[test]
fn zeroed_precpu_is_dropped_even_with_a_network_baseline() {
    let mut sampler = StatsSampler::default();
    let mut earlier = raw_frames()[0].clone();
    earlier["read"] = json!("2026-11-24T22:49:59.000000000Z");
    assert_eq!(sampler.sample(&frame(earlier)), None);
    assert_eq!(sampler.sample(&zeroed_precpu_frame()), None);
}

#[test]
fn first_stream_frame_is_dropped() {
    let mut sampler = StatsSampler::default();
    assert_eq!(cpu_percent(&frames()[0]), None);
    assert_eq!(sampler.sample(&frames()[0]), None);
}

#[test]
fn consecutive_stream_frames_report_one_full_core() {
    let frames = frames();
    let mut sampler = StatsSampler::default();
    assert_eq!(sampler.sample(&frames[0]), None);

    let second = sampler.sample(&frames[1]).expect("frame 2 has a previous sample");
    // (1005872000 / 8030000000) * 8 * 100
    assert!((second.cpu_percent - 100.2114).abs() < 0.001, "{}", second.cpu_percent);

    let third = sampler.sample(&frames[2]).expect("frame 3 has a previous sample");
    // (1005958000 / 8050000000) * 8 * 100
    assert!((third.cpu_percent - 99.9710).abs() < 0.001, "{}", third.cpu_percent);
}

#[test]
fn stream_frames_keep_memory_from_the_current_frame() {
    let frames = frames();
    let mut sampler = StatsSampler::default();
    sampler.sample(&frames[0]);
    let sample = sampler.sample(&frames[1]).unwrap();
    // usage 5255168 minus inactive_file 552960
    assert!((sample.memory_mb - (5_255_168.0 - 552_960.0) / 1_048_576.0).abs() < 1e-9);
    assert!(sample.memory_limit_mb > 0.0);
}

#[test]
fn net_rate_uses_docker_read_timestamps() {
    let frames = frames();
    let previous = net_counters(&frames[0]).unwrap();
    let current = net_counters(&frames[1]).unwrap();
    let elapsed = 34.299237543 - 33.293512668;

    let (rx, tx) = net_rates(&previous, &current).unwrap();
    assert!((rx - 240.0 / elapsed).abs() < 1e-6, "{rx}");
    assert_eq!(tx, 0.0);

    let mut sampler = StatsSampler::default();
    sampler.sample(&frames[0]);
    let sample = sampler.sample(&frames[1]).unwrap();
    assert!((sample.net_rx_per_sec - 240.0 / elapsed).abs() < 1e-6);
    assert_eq!(sample.net_tx_per_sec, 0.0);

    let quiet = sampler.sample(&frames[2]).unwrap();
    assert_eq!(quiet.net_rx_per_sec, 0.0);
}

#[test]
fn net_rate_needs_time_to_pass() {
    let current = net_counters(&frames()[1]).unwrap();
    assert_eq!(net_rates(&current, &current), None);

    let mut sampler = StatsSampler::default();
    sampler.sample(&frames()[1]);
    assert_eq!(sampler.sample(&frames()[1]), None);
}

#[test]
fn backwards_network_counters_yield_no_sample() {
    let raw = raw_frames();
    let mut sampler = StatsSampler::default();
    sampler.sample(&frame(raw[0].clone()));
    assert!(sampler.sample(&frame(raw[1].clone())).is_some());

    let mut restarted = raw[2].clone();
    restarted["networks"]["eth0"]["rx_bytes"] = json!(100);
    assert_eq!(sampler.sample(&frame(restarted.clone())), None);

    // The dropped frame becomes the new baseline instead of the stale counters.
    let mut next = restarted;
    next["read"] = json!("2026-11-24T23:08:36.305793919Z");
    next["networks"]["eth0"]["rx_bytes"] = json!(300);
    let sample = sampler
        .sample(&frame(next))
        .expect("baseline moved to the restarted counters");
    assert!((sample.net_rx_per_sec - 200.0).abs() < 1e-9);
}

#[test]
fn backwards_cpu_counters_yield_no_sample() {
    let mut raw = raw_frames()[1].clone();
    raw["precpu_stats"]["cpu_usage"]["total_usage"] = json!(4_000_000_000u64);
    assert_eq!(cpu_percent(&frame(raw.clone())), None);

    raw = raw_frames()[1].clone();
    raw["precpu_stats"]["system_cpu_usage"] = json!(40_000_000_000u64);
    assert_eq!(cpu_percent(&frame(raw)), None);
}

#[test]
fn idle_container_reports_zero_not_nothing() {
    let mut raw = raw_frames()[1].clone();
    raw["cpu_stats"]["cpu_usage"]["total_usage"] = json!(2069017000u64);
    assert_eq!(cpu_percent(&frame(raw)), Some(0.0));
}

#[test]
fn missing_online_cpus_falls_back_to_per_cpu_count() {
    // Older daemons leave out online_cpus; Linux always reports num_procs as 0.
    let mut value = raw_frames()[1].clone();
    value["cpu_stats"]
        .as_object_mut()
        .expect("cpu_stats is an object")
        .remove("online_cpus");
    value["cpu_stats"]["cpu_usage"]["percpu_usage"] = json!([0, 0, 0, 0, 0, 0, 0, 0]);
    let percent = cpu_percent(&frame(value)).expect("frame 2 has a previous sample");
    assert!((percent - 100.2114).abs() < 0.001, "{percent}");
}
