use rustydocker::ui::graph::build_graph_lines;

#[test]
fn test_simple_dependency_graph() {
    let services = vec![
        ("web".to_string(), vec!["api".to_string()]),
        ("api".to_string(), vec!["db".to_string()]),
        ("db".to_string(), vec![]),
    ];
    let lines = build_graph_lines(&services, Some("api"));
    let text: String = lines
        .iter()
        .map(|l| l.iter().map(|(s, _)| s.as_str()).collect::<String>())
        .collect();
    assert!(text.contains("web"));
    assert!(text.contains("api"));
    assert!(text.contains("db"));
}

#[test]
fn test_graph_no_deps() {
    let services = vec![("app".to_string(), vec![])];
    let lines = build_graph_lines(&services, None);
    let text: String = lines
        .iter()
        .map(|l| l.iter().map(|(s, _)| s.as_str()).collect::<String>())
        .collect();
    assert!(text.contains("app"));
}
