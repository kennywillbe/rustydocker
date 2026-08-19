use crate::app::App;
use crate::docker::client::DockerClient;

pub async fn refresh_selected_image(app: &mut App, docker: &DockerClient) {
    let Some(image) = app.images.get(app.selected_index) else {
        app.image_inspect = None;
        app.image_history.clear();
        return;
    };
    let reference = image.repo_tags.first().map(String::as_str).unwrap_or(&image.id);
    app.image_inspect = docker.inspect_image(reference).await.ok();
    app.image_history = docker.image_history(reference).await.unwrap_or_default();
}
