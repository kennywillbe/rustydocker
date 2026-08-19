use bollard::models::ContainerSummary;

pub fn container_state(container: &ContainerSummary) -> Option<&str> {
    container.state.as_ref().map(AsRef::as_ref)
}
