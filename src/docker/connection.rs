use crate::config::AppConfig;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionSpec {
    Local,
    Unix { socket: String },
    Http { host: String },
    Tls { host: String, cert_path: PathBuf },
    Ssh { host: String, key_path: Option<String> },
}

impl ConnectionSpec {
    pub fn host(&self) -> Option<&str> {
        match self {
            Self::Local => None,
            Self::Unix { socket } => Some(socket),
            Self::Http { host } | Self::Tls { host, .. } | Self::Ssh { host, .. } => Some(host),
        }
    }

    pub fn configure_cli(&self, command: &mut Command, context: Option<&str>) {
        if let Some(context) = context {
            command
                .env("DOCKER_CONTEXT", context)
                .env_remove("DOCKER_HOST")
                .env_remove("DOCKER_TLS_VERIFY")
                .env_remove("DOCKER_CERT_PATH");
            return;
        }
        if let Some(host) = self.host() {
            command.env("DOCKER_HOST", host);
        }
        if let Self::Tls { cert_path, .. } = self {
            command.env("DOCKER_TLS_VERIFY", "1").env("DOCKER_CERT_PATH", cert_path);
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedConnection {
    pub spec: ConnectionSpec,
    pub context: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContextInspect {
    endpoints: ContextEndpoints,
    #[serde(default)]
    tls_material: HashMap<String, serde_json::Value>,
    #[serde(default)]
    storage: ContextStorage,
}

#[derive(Debug, Deserialize)]
struct ContextEndpoints {
    docker: ContextDockerEndpoint,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContextDockerEndpoint {
    host: String,
    #[serde(default)]
    skip_tls_verify: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContextStorage {
    #[serde(rename = "TLSPath")]
    tls_path: Option<PathBuf>,
}

fn tls_requested(env: &impl Fn(&str) -> Option<String>, config: &AppConfig) -> bool {
    env("DOCKER_TLS_VERIFY").is_some_and(|value| !value.is_empty() && value != "0") || config.docker_tls_verify
}

fn connection_from_host(
    host: String,
    cert_path: Option<PathBuf>,
    verify_tls: bool,
    ssh_key_path: Option<String>,
) -> Result<ConnectionSpec> {
    if host.starts_with("unix://") || host.starts_with("npipe://") {
        return Ok(ConnectionSpec::Unix { socket: host });
    }
    if host.starts_with("ssh://") {
        return Ok(ConnectionSpec::Ssh {
            host,
            key_path: ssh_key_path,
        });
    }
    if host.starts_with("https://") || verify_tls || cert_path.is_some() {
        let cert_path = cert_path.context(
            "TLS is enabled but no certificate directory was found; set DOCKER_CERT_PATH or docker_cert_path",
        )?;
        validate_cert_path(&cert_path)?;
        return Ok(ConnectionSpec::Tls { host, cert_path });
    }
    if host.starts_with("tcp://") || host.starts_with("http://") {
        return Ok(ConnectionSpec::Http { host });
    }
    bail!("Unsupported Docker endpoint: {host}")
}

fn validate_cert_path(path: &Path) -> Result<()> {
    for name in ["ca.pem", "cert.pem", "key.pem"] {
        let file = path.join(name);
        if !file.is_file() {
            bail!("Docker TLS file is missing: {}", file.display());
        }
    }
    Ok(())
}

fn inspect_context(name: &str) -> Result<ContextInspect> {
    let output = Command::new("docker")
        .args(["context", "inspect", name])
        .output()
        .with_context(|| format!("failed to inspect Docker context '{name}'"))?;
    if !output.status.success() {
        bail!(
            "failed to inspect Docker context '{name}': {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let contexts: Vec<ContextInspect> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("invalid Docker context metadata for '{name}'"))?;
    contexts
        .into_iter()
        .next()
        .context("Docker context inspect returned no context")
}

pub fn resolve(cli_context: Option<&str>, config: &AppConfig) -> Result<ResolvedConnection> {
    let environment_context = std::env::var("DOCKER_CONTEXT").ok();
    let environment_host = std::env::var("DOCKER_HOST").ok();
    let configured_endpoint = config.docker_context.is_some() || config.docker_host.is_some();
    let selected_context =
        if cli_context.is_none() && environment_context.is_none() && environment_host.is_none() && !configured_endpoint
        {
            current_context().ok()
        } else {
            None
        };
    resolve_with(
        cli_context.or(selected_context.as_deref()),
        config,
        |name| std::env::var(name).ok(),
        inspect_context,
    )
}

fn current_context() -> Result<String> {
    let output = Command::new("docker")
        .args(["context", "show"])
        .output()
        .context("failed to query the current Docker context")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let name = String::from_utf8(output.stdout)?.trim().to_string();
    if name.is_empty() {
        bail!("Docker did not report a current context");
    }
    Ok(name)
}

fn resolve_with<E, I>(cli_context: Option<&str>, config: &AppConfig, env: E, inspect: I) -> Result<ResolvedConnection>
where
    E: Fn(&str) -> Option<String>,
    I: Fn(&str) -> Result<ContextInspect>,
{
    let context = cli_context.map(str::to_owned).or_else(|| env("DOCKER_CONTEXT"));

    if let Some(name) = context {
        let metadata = inspect(&name)?;
        let has_tls_material = metadata.tls_material.get("docker").is_some_and(|value| {
            value.as_array().is_some_and(|files| !files.is_empty())
                || value.as_object().is_some_and(|files| !files.is_empty())
        });
        let cert_path = has_tls_material.then_some(metadata.storage.tls_path).flatten();
        let verify_tls = !metadata.endpoints.docker.skip_tls_verify && cert_path.is_some();
        let spec = connection_from_host(
            metadata.endpoints.docker.host,
            cert_path,
            verify_tls,
            config.docker_ssh_key_path.clone(),
        )?;
        return Ok(ResolvedConnection {
            spec,
            context: Some(name),
        });
    }

    if let Some(host) = env("DOCKER_HOST") {
        let cert_path = env("DOCKER_CERT_PATH")
            .map(PathBuf::from)
            .or_else(|| config.docker_cert_path.clone());
        let spec = connection_from_host(
            host,
            cert_path,
            tls_requested(&env, config),
            config.docker_ssh_key_path.clone(),
        )?;
        return Ok(ResolvedConnection { spec, context: None });
    }

    if let Some(name) = &config.docker_context {
        let metadata = inspect(name)?;
        let has_tls_material = metadata.tls_material.get("docker").is_some_and(|value| {
            value.as_array().is_some_and(|files| !files.is_empty())
                || value.as_object().is_some_and(|files| !files.is_empty())
        });
        let cert_path = has_tls_material.then_some(metadata.storage.tls_path).flatten();
        let verify_tls = !metadata.endpoints.docker.skip_tls_verify && cert_path.is_some();
        let spec = connection_from_host(
            metadata.endpoints.docker.host,
            cert_path,
            verify_tls,
            config.docker_ssh_key_path.clone(),
        )?;
        return Ok(ResolvedConnection {
            spec,
            context: Some(name.clone()),
        });
    }

    let host = config.docker_host.clone();
    let Some(host) = host else {
        return Ok(ResolvedConnection {
            spec: ConnectionSpec::Local,
            context: None,
        });
    };
    let cert_path = env("DOCKER_CERT_PATH")
        .map(PathBuf::from)
        .or_else(|| config.docker_cert_path.clone());
    let spec = connection_from_host(
        host,
        cert_path,
        tls_requested(&env, config),
        config.docker_ssh_key_path.clone(),
    )?;
    Ok(ResolvedConnection { spec, context: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn env(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let values: HashMap<String, String> = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        move |key| values.get(key).cloned()
    }

    fn unix_context(host: &str) -> ContextInspect {
        ContextInspect {
            endpoints: ContextEndpoints {
                docker: ContextDockerEndpoint {
                    host: host.to_string(),
                    skip_tls_verify: false,
                },
            },
            tls_material: HashMap::new(),
            storage: ContextStorage::default(),
        }
    }

    #[test]
    fn cli_context_wins_over_environment_and_host() {
        let config = AppConfig::default();
        let resolved = resolve_with(
            Some("cli"),
            &config,
            env(&[("DOCKER_CONTEXT", "environment"), ("DOCKER_HOST", "tcp://ignored:2375")]),
            |name| {
                assert_eq!(name, "cli");
                Ok(unix_context("unix:///tmp/docker.sock"))
            },
        )
        .unwrap();
        assert_eq!(resolved.context.as_deref(), Some("cli"));
        assert_eq!(
            resolved.spec,
            ConnectionSpec::Unix {
                socket: "unix:///tmp/docker.sock".to_string()
            }
        );
    }

    #[test]
    fn docker_context_wins_over_docker_host() {
        let resolved = resolve_with(
            None,
            &AppConfig::default(),
            env(&[("DOCKER_CONTEXT", "remote"), ("DOCKER_HOST", "tcp://ignored:2375")]),
            |_| Ok(unix_context("unix:///tmp/context.sock")),
        )
        .unwrap();
        assert_eq!(resolved.context.as_deref(), Some("remote"));
    }

    #[test]
    fn plain_tcp_host_remains_explicit_http() {
        let resolved = resolve_with(
            None,
            &AppConfig::default(),
            env(&[("DOCKER_HOST", "tcp://daemon:2375")]),
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(
            resolved.spec,
            ConnectionSpec::Http {
                host: "tcp://daemon:2375".to_string()
            }
        );
    }

    #[test]
    fn plain_tcp_context_does_not_infer_tls_from_empty_storage_directory() {
        let resolved = resolve_with(None, &AppConfig::default(), env(&[("DOCKER_CONTEXT", "plain")]), |_| {
            Ok(ContextInspect {
                endpoints: ContextEndpoints {
                    docker: ContextDockerEndpoint {
                        host: "tcp://daemon:2375".to_string(),
                        skip_tls_verify: false,
                    },
                },
                tls_material: HashMap::new(),
                storage: ContextStorage {
                    tls_path: Some(PathBuf::from("/tmp/empty-context-tls")),
                },
            })
        })
        .unwrap();
        assert!(matches!(resolved.spec, ConnectionSpec::Http { .. }));
    }

    #[test]
    fn ssh_host_carries_key_path() {
        let config = AppConfig {
            docker_ssh_key_path: Some("/tmp/id_ed25519".to_string()),
            ..AppConfig::default()
        };
        let resolved = resolve_with(
            None,
            &config,
            env(&[("DOCKER_HOST", "ssh://me@example.test")]),
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(
            resolved.spec,
            ConnectionSpec::Ssh {
                host: "ssh://me@example.test".to_string(),
                key_path: Some("/tmp/id_ed25519".to_string())
            }
        );
    }

    #[test]
    fn environment_host_wins_over_configured_context() {
        let config = AppConfig {
            docker_context: Some("configured".to_string()),
            ..AppConfig::default()
        };
        let resolved = resolve_with(None, &config, env(&[("DOCKER_HOST", "tcp://environment:2375")]), |_| {
            panic!("configured context must not be inspected")
        })
        .unwrap();
        assert_eq!(
            resolved.spec,
            ConnectionSpec::Http {
                host: "tcp://environment:2375".to_string()
            }
        );
    }

    #[test]
    fn tls_requires_complete_certificate_bundle() {
        let directory = std::env::temp_dir().join(format!(
            "rustydocker-tls-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("ca.pem"), "ca").unwrap();
        let error =
            connection_from_host("tcp://secure:2376".to_string(), Some(directory.clone()), true, None).unwrap_err();
        assert!(error.to_string().contains("cert.pem"));

        std::fs::write(directory.join("cert.pem"), "cert").unwrap();
        std::fs::write(directory.join("key.pem"), "key").unwrap();
        assert!(matches!(
            connection_from_host("tcp://secure:2376".to_string(), Some(directory.clone()), true, None,).unwrap(),
            ConnectionSpec::Tls { .. }
        ));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
