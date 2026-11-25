use bollard::models::{ClusterVolume, Network, Volume};
use rustydocker::docker::model::{sort_networks, sort_volumes};

fn network(name: &str, id: &str) -> Network {
    Network {
        name: Some(name.to_string()),
        id: Some(id.to_string()),
        ..Default::default()
    }
}

fn volume(name: &str) -> Volume {
    volume_on(name, "local")
}

fn volume_on(name: &str, driver: &str) -> Volume {
    Volume {
        name: name.to_string(),
        driver: driver.to_string(),
        ..Default::default()
    }
}

fn cluster_volume(name: &str, id: &str) -> Volume {
    Volume {
        cluster_volume: Some(ClusterVolume {
            id: Some(id.to_string()),
            ..Default::default()
        }),
        ..volume_on(name, "csi")
    }
}

fn network_names(networks: &[Network]) -> Vec<&str> {
    networks.iter().map(|n| n.name.as_deref().unwrap_or("")).collect()
}

#[test]
fn networks_sort_the_same_whatever_order_docker_returns() {
    // The two consecutive orders reported in issue #67.
    let mut first = vec![
        network("host", "1"),
        network("bridge", "2"),
        network("demo", "3"),
        network("none", "4"),
    ];
    let mut second = vec![
        network("demo", "3"),
        network("none", "4"),
        network("host", "1"),
        network("bridge", "2"),
    ];
    sort_networks(&mut first);
    sort_networks(&mut second);

    assert_eq!(network_names(&first), ["bridge", "demo", "host", "none"]);
    assert_eq!(first, second);
}

#[test]
fn networks_with_the_same_name_keep_a_fixed_order() {
    let mut first = vec![network("demo", "bbb"), network("demo", "aaa")];
    let mut second = vec![network("demo", "aaa"), network("demo", "bbb")];
    sort_networks(&mut first);
    sort_networks(&mut second);

    assert_eq!(first, second);
    assert_eq!(first[0].id.as_deref(), Some("aaa"));
}

#[test]
fn volumes_sort_by_name() {
    let mut volumes = vec![volume("pgdata"), volume("cache"), volume("logs")];
    sort_volumes(&mut volumes);

    let names: Vec<&str> = volumes.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["cache", "logs", "pgdata"]);
}

#[test]
fn volumes_with_the_same_name_keep_a_fixed_order() {
    // Two plugin drivers can both report "data", in either order.
    let mut first = vec![volume_on("data", "nfs"), volume_on("data", "efs")];
    let mut second = vec![volume_on("data", "efs"), volume_on("data", "nfs")];
    sort_volumes(&mut first);
    sort_volumes(&mut second);
    assert_eq!(first, second);
    assert_eq!(first[0].driver, "efs");

    let mut first = vec![cluster_volume("data", "z1"), cluster_volume("data", "a1")];
    let mut second = vec![cluster_volume("data", "a1"), cluster_volume("data", "z1")];
    sort_volumes(&mut first);
    sort_volumes(&mut second);
    assert_eq!(first, second);
}
