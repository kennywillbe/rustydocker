use rustydocker::docker::compose::parse_compose;

#[test]
fn test_parse_simple_compose() {
    let yaml = r#"
version: "3"
services:
  web:
    image: nginx
    depends_on:
      - api
  api:
    image: node:18
    depends_on:
      - db
  db:
    image: postgres:15
"#;
    let project = parse_compose(yaml, "myproject").unwrap();
    assert_eq!(project.name, "myproject");
    assert_eq!(project.services.len(), 3);

    let api = project.services.iter().find(|s| s.name == "api").unwrap();
    assert_eq!(api.depends_on, vec!["db"]);

    let web = project.services.iter().find(|s| s.name == "web").unwrap();
    assert_eq!(web.depends_on, vec!["api"]);

    let db = project.services.iter().find(|s| s.name == "db").unwrap();
    assert!(db.depends_on.is_empty());
}

#[test]
fn test_parse_compose_no_depends_on() {
    let yaml = r#"
services:
  app:
    image: myapp
"#;
    let project = parse_compose(yaml, "solo").unwrap();
    assert_eq!(project.services.len(), 1);
    assert_eq!(project.services[0].name, "app");
    assert!(project.services[0].depends_on.is_empty());
}

#[test]
fn test_parse_compose_map_depends_on() {
    let yaml = r#"
services:
  web:
    image: nginx
    depends_on:
      api:
        condition: service_healthy
"#;
    let project = parse_compose(yaml, "test").unwrap();
    let web = project.services.iter().find(|s| s.name == "web").unwrap();
    assert_eq!(web.depends_on, vec!["api"]);
}

#[test]
fn test_parse_compose_empty_services() {
    let yaml = r#"
services: {}
"#;
    let project = parse_compose(yaml, "empty").unwrap();
    assert_eq!(project.services.len(), 0);
}
