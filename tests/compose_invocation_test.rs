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

#[test]
fn invocation_forwards_context_and_profiles() {
    let invocation = ComposeInvocation::new(
        vec!["compose.yml".to_string()],
        Some("demo".to_string()),
        Some("tcp://ignored:2375".to_string()),
    )
    .with_context(Some("production".to_string()))
    .with_profiles(vec!["debug".to_string(), "metrics".to_string()]);

    assert_eq!(
        invocation.args(&["config"]),
        [
            "compose",
            "-f",
            "compose.yml",
            "-p",
            "demo",
            "--profile",
            "debug",
            "--profile",
            "metrics",
            "config",
        ]
    );
    let command = invocation.command(&["config"]);
    let envs: std::collections::HashMap<_, _> = command.as_std().get_envs().collect();
    assert_eq!(
        envs.get(std::ffi::OsStr::new("DOCKER_CONTEXT"))
            .and_then(|value| *value),
        Some(std::ffi::OsStr::new("production"))
    );
    assert_eq!(envs.get(std::ffi::OsStr::new("DOCKER_HOST")), Some(&None));
}
