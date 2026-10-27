use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form
};
use minijinja::context;

use crate::cmd::serve::{
    S,
    async_index::Update,
    handlers::{check_path, ok_status, mj_ok},
    locale::UserLocale
};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RunRequest {
    name: String,
}

pub async fn rename_directory(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    Form(request): Form<RunRequest>
) -> Response {

    let toast_html = |text_key: &str, color: &str| -> Result<Html<String>, minijinja::Error> {
        let tmpl = state.templates.get_template("components/oob_toast.html")?;
        let html = tmpl.render(context! {
            t,
            text_key,
            color
        })?;
        Ok(Html(html))
    };

    let err_html = || toast_html("editor.error", "red");

    if state.disable_open_editor {
        return (StatusCode::UNAUTHORIZED, mj_ok!(err_html())).into_response();
    }

    if let Err(err) = check_path(&path) {
        return (err, mj_ok!(err_html())).into_response();
    }
    if let Err(err) = check_path(&request.name) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let old_path = state.base_path.join(&path);

    if !old_path.exists() || !old_path.is_dir() {
        return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
    }

    let Some(parent_path) = old_path.parent() else {
        return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response();
    };

    let new_path = parent_path.join(&request.name);

    if new_path == old_path {
        return StatusCode::BAD_REQUEST.into_response();
    }

    tracing::info!("Renaming '{}' to '{}'", &old_path, &new_path);

    if new_path.exists() {
        return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
    }

    if let Err(err) = tokio::fs::rename(&old_path, &new_path).await {
        tracing::error!("Cannot rename directory {old_path}: {err}");
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }
    
    let _ = state.updates_sender.send(Update::Renamed {
        from: old_path.into(),
        to: new_path.into()
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
}

pub async fn rename_recipe(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    Form(request): Form<RunRequest>
) -> Response {

    let toast_html = |text_key: &str, color: &str| -> Result<Html<String>, minijinja::Error> {
        let tmpl = state.templates.get_template("components/oob_toast.html")?;
        let html = tmpl.render(context! {
            t,
            text_key,
            color
        })?;
        Ok(Html(html))
    };

    let err_html = || toast_html("editor.error", "red");

    if state.disable_open_editor {
        return (StatusCode::UNAUTHORIZED, mj_ok!(err_html())).into_response();
    }

    if let Err(err) = check_path(&path) {
        return (err, mj_ok!(err_html())).into_response();
    }
    if let Err(err) = check_path(&request.name) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);
    let old_path = entry.path();

    let Some(parent_path) = old_path.parent() else {
        return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response();
    };

    let new_path = parent_path.join(&request.name).with_extension("cook");

    if new_path == old_path {
        return StatusCode::BAD_REQUEST.into_response();
    }

    tracing::info!("Renaming '{}' to '{}'", &old_path, &new_path);

    let old_images = entry.images();

    if let Err(err) = tokio::fs::rename(&old_path, &new_path).await {
        tracing::error!("Cannot rename recipe {old_path}: {err}");
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }

    let old_name = old_path.file_stem().unwrap();
    let new_name = new_path.file_stem().unwrap();

    let mut has_error = false;

    for image in old_images {
        let Some(file_name) = image.path.file_name() else {
            continue;
        };
        let Some(suffix) = file_name.strip_prefix(old_name) else {
            continue;
        };
        let new_file_name = format!("{new_name}{suffix}");
        let image_new_path = image.path.with_file_name(new_file_name);
        if let Err(err) = tokio::fs::rename(&image.path, &image_new_path).await {
            tracing::error!("Cannot rename image {} -> {}: {err}", image.path, image_new_path);
            has_error = true;
        }
    }
        
    let _ = state.updates_sender.send(Update::Renamed {
        from: old_path.into(),
        to: new_path.into(),
    });

    if has_error {
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }
    mj_ok!(toast_html("editor.success", "green")).into_response()
}

