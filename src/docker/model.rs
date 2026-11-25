use bollard::models::{ContainerSummary, Network, Volume};

pub fn container_state(container: &ContainerSummary) -> Option<&str> {
    container.state.as_ref().map(AsRef::as_ref)
}

/// The Engine API returns networks in no stable order, so the sidebar would
/// reshuffle on every refresh. Names can repeat across drivers, hence the id.
pub fn sort_networks(networks: &mut [Network]) {
    networks.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
}

/// A local and a Swarm cluster volume, or volumes from two plugin drivers, can
/// share a name, and drivers are listed in parallel, so the name alone can
/// still let equal entries swap between refreshes.
pub fn sort_volumes(volumes: &mut [Volume]) {
    let cluster_id = |volume: &Volume| volume.cluster_volume.as_ref().and_then(|cluster| cluster.id.clone());
    volumes.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.driver.cmp(&b.driver))
            .then_with(|| cluster_id(a).cmp(&cluster_id(b)))
    });
}
