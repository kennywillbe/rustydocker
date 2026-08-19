use rustydocker::docker::compose::{load_effective_project, ComposeInvocation};

#[tokio::test]
async fn docker_compose_renders_merged_effective_model() {
    let available = std::process::Command::new("docker")
        .args(["compose", "version"])
        .output()
        .is_ok_and(|output| output.status.success());
    if !available {
        eprintln!("skipping: Docker Compose CLI is unavailable");
        return;
    }

    let root = env!("CARGO_MANIFEST_DIR");
    let invocation = ComposeInvocation::new(
        vec![
            format!("{root}/tests/fixtures/compose/base.yml"),
            format!("{root}/tests/fixtures/compose/override.yml"),
        ],
        Some("fixture".to_string()),
        None,
    )
    .with_profiles(vec!["debug".to_string()]);

    let project = load_effective_project(&invocation).await.unwrap();
    assert_eq!(project.name, "fixture");
    assert_eq!(project.services.len(), 3);
    let api = project.services.iter().find(|service| service.name == "api").unwrap();
    assert_eq!(api.image.as_deref(), Some("alpine:3.21"));
    assert!(api.has_build);
    assert_eq!(api.depends_on, ["db"]);
    let worker = project
        .services
        .iter()
        .find(|service| service.name == "worker")
        .unwrap();
    assert_eq!(worker.profiles, ["debug"]);
}
