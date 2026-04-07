use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ComposeProject {
    pub name: String,
    pub path: String,
    pub services: Vec<ComposeService>,
}

#[derive(Debug, Clone)]
pub struct ComposeService {
    pub name: String,
    pub image: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Deserialize)]
struct ComposeFile {
    services: Option<HashMap<String, ServiceDef>>,
}

#[derive(Deserialize)]
struct ServiceDef {
    image: Option<String>,
    depends_on: Option<DependsOn>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DependsOn {
    List(Vec<String>),
    Map(HashMap<String, serde_yaml::Value>),
}

pub fn parse_compose(yaml_content: &str, project_name: &str) -> Result<ComposeProject> {
    let file: ComposeFile = serde_yaml::from_str(yaml_content)?;
    let services_map = file.services.unwrap_or_default();

    let mut services: Vec<ComposeService> = services_map
        .into_iter()
        .map(|(name, def)| {
            let depends_on = match def.depends_on {
                Some(DependsOn::List(list)) => list,
                Some(DependsOn::Map(map)) => map.into_keys().collect(),
                None => vec![],
            };
            ComposeService {
                name,
                image: def.image,
                depends_on,
            }
        })
        .collect();

    services.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ComposeProject {
        name: project_name.to_string(),
        path: String::new(),
        services,
    })
}

pub fn find_compose_files(dir: &Path) -> Vec<String> {
    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    let mut found = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if candidates.contains(&name_str.as_ref()) {
                found.push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    found
}

pub fn load_compose_project(path: &str) -> Result<ComposeProject> {
    let content = std::fs::read_to_string(path)?;
    let dir = Path::new(path)
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mut project = parse_compose(&content, &dir)?;
    project.path = path.to_string();
    Ok(project)
}
