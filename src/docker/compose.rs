use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposeInvocation {
    files: Vec<String>,
    project_name: Option<String>,
    docker_host: Option<String>,
    docker_context: Option<String>,
    profiles: Vec<String>,
    docker_cert_path: Option<PathBuf>,
}

impl ComposeInvocation {
    pub fn new(files: Vec<String>, project_name: Option<String>, docker_host: Option<String>) -> Self {
        Self {
            files,
            project_name,
            docker_host,
            docker_context: None,
            profiles: vec![],
            docker_cert_path: None,
        }
    }

    pub fn with_context(mut self, docker_context: Option<String>) -> Self {
        self.docker_context = docker_context;
        self
    }

    pub fn with_profiles(mut self, profiles: Vec<String>) -> Self {
        self.profiles = profiles;
        self
    }

    pub fn with_tls_cert_path(mut self, docker_cert_path: Option<PathBuf>) -> Self {
        self.docker_cert_path = docker_cert_path;
        self
    }

    pub fn args(&self, action: &[&str]) -> Vec<String> {
        let mut args = vec!["compose".to_string()];
        for file in &self.files {
            args.push("-f".to_string());
            args.push(file.clone());
        }
        if let Some(project_name) = &self.project_name {
            args.push("-p".to_string());
            args.push(project_name.clone());
        }
        for profile in &self.profiles {
            args.push("--profile".to_string());
            args.push(profile.clone());
        }
        args.extend(action.iter().map(|arg| (*arg).to_string()));
        args
    }

    pub fn command(&self, action: &[&str]) -> Command {
        let mut command = Command::new("docker");
        command.args(self.args(action));
        if let Some(docker_context) = &self.docker_context {
            command.env("DOCKER_CONTEXT", docker_context);
            command.env_remove("DOCKER_HOST");
            command.env_remove("DOCKER_TLS_VERIFY");
            command.env_remove("DOCKER_CERT_PATH");
        } else if let Some(docker_host) = &self.docker_host {
            command.env("DOCKER_HOST", docker_host);
            if let Some(cert_path) = &self.docker_cert_path {
                command.env("DOCKER_TLS_VERIFY", "1").env("DOCKER_CERT_PATH", cert_path);
            }
        }
        command
    }

    pub async fn run(&self, action: &[&str]) -> std::io::Result<std::process::Output> {
        self.command(action).output().await
    }
}

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
    pub profiles: Vec<String>,
    pub has_build: bool,
}

#[derive(Deserialize)]
struct ComposeFile {
    name: Option<String>,
    services: Option<HashMap<String, ServiceDef>>,
}

#[derive(Deserialize)]
struct ServiceDef {
    image: Option<String>,
    depends_on: Option<DependsOn>,
    #[serde(default)]
    profiles: Vec<String>,
    build: Option<serde_yaml::Value>,
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
                profiles: def.profiles,
                has_build: def.build.is_some(),
            }
        })
        .collect();

    services.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ComposeProject {
        name: file.name.unwrap_or_else(|| project_name.to_string()),
        path: String::new(),
        services,
    })
}

/// Ask Docker Compose to render the canonical merged model. This honors all
/// `-f`, project-name, environment interpolation, include/extends, and profile
/// options exactly as the installed Compose implementation does.
pub async fn load_effective_project(invocation: &ComposeInvocation) -> Result<ComposeProject> {
    let output = invocation.run(&["config", "--format", "json"]).await?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let mut project = parse_effective_json(&output.stdout)?;
    project.path = invocation.files.first().cloned().unwrap_or_default();
    Ok(project)
}

pub fn parse_effective_json(json: &[u8]) -> Result<ComposeProject> {
    let file: ComposeFile = serde_json::from_slice(json)?;
    let project_name = file.name.clone().unwrap_or_else(|| "compose".to_string());
    Ok(compose_file_to_project(file, &project_name))
}

fn compose_file_to_project(file: ComposeFile, project_name: &str) -> ComposeProject {
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
                profiles: def.profiles,
                has_build: def.build.is_some(),
            }
        })
        .collect();
    services.sort_by(|a, b| a.name.cmp(&b.name));
    ComposeProject {
        name: file.name.unwrap_or_else(|| project_name.to_string()),
        path: String::new(),
        services,
    }
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
