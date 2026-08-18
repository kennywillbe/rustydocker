use rustydocker::docker::compose::ComposeInvocation;

#[test]
fn default_invocation_uses_compose_discovery() {
    let invocation = ComposeInvocation::default();

    assert_eq!(invocation.args(&["up", "-d"]), ["compose", "up", "-d"]);
}

#[test]
fn invocation_preserves_file_order_and_project_name() {
    let invocation = ComposeInvocation::new(
        vec!["compose.yml".to_string(), "compose.dev.yml".to_string()],
        Some("demo".to_string()),
        None,
    );

    assert_eq!(
        invocation.args(&["restart"]),
        [
            "compose",
            "-f",
            "compose.yml",
            "-f",
            "compose.dev.yml",
            "-p",
            "demo",
            "restart",
        ]
    );
}

#[test]
fn invocation_forwards_configured_docker_host() {
    let invocation = ComposeInvocation::new(Vec::new(), None, Some("tcp://docker.example:2375".to_string()));
    let command = invocation.command(&["down"]);
    let docker_host = command
        .as_std()
        .get_envs()
        .find(|(key, _)| *key == "DOCKER_HOST")
        .and_then(|(_, value)| value)
        .and_then(|value| value.to_str());

    assert_eq!(docker_host, Some("tcp://docker.example:2375"));
}
