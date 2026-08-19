use crate::app::App;
use crate::docker::client::DockerClient;

pub async fn refresh_selected(app: &mut App, docker: &DockerClient, refresh_logs: bool) {
    let Some(id) = app.selected_container_id().map(str::to_owned) else {
        app.container_env = None;
        app.container_inspect = None;
        app.container_top = None;
        return;
    };

    if refresh_logs {
        if let Ok(logs) = docker.container_logs_batch(&id, &app.log_tail_lines).await {
            app.logs.insert(id.clone(), logs);
        }
        app.log_bookmarks.clear();
    }

    match docker.inspect_container(&id).await {
        Ok(inspect) => {
            app.container_env = Some(
                inspect
                    .config
                    .as_ref()
                    .and_then(|config| config.env.as_ref())
                    .map(|env| {
                        env.iter()
                            .filter_map(|entry| {
                                entry
                                    .split_once('=')
                                    .map(|(key, value)| (key.to_string(), value.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            );
            app.container_inspect = Some(inspect);
        }
        Err(_) => {
            app.container_env = None;
            app.container_inspect = None;
        }
    }
    app.container_top = docker.container_top(&id).await.ok();
}
