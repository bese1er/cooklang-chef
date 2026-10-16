use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Form
};
use camino::Utf8PathBuf;
use minijinja::context;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::File;

use crate::cmd::serve::{
    S,
    async_index::Update,
    handlers::{check_path, mj_ok},
    locale::UserLocale
};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RunRequest {
    name: String,
}

pub async fn create_directory(
    UserLocale(t): UserLocale,
    requested_path: Option<Path<String>>,
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

    let mut path = Utf8PathBuf::from(&state.base_path);
    if let Some(Path(p)) = &requested_path {
        match check_path(p) {
            Ok(_) => {
                path = path.join(p);
            }
            Err(err) => return (err, mj_ok!(err_html())).into_response()
        }
    }

    if let Err(err) = check_path(&request.name) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let new_path = path.join(&request.name);

    tracing::info!("Creating directory '{}'", &new_path);

    if let Err(err) = tokio::fs::create_dir_all(&new_path).await {
        tracing::error!("Cannot create directory {new_path}: {err}");
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }
    
    let _ = state.updates_sender.send(Update::Modified {
        path: new_path
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
}

pub async fn create_recipe(
    UserLocale(t): UserLocale,
    requested_path: Option<Path<String>>,
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

    let mut path = Utf8PathBuf::from(&state.base_path);
    if let Some(Path(p)) = &requested_path {
        match check_path(p) {
            Ok(_) => {
                path = path.join(p);
            }
            Err(err) => return (err, mj_ok!(err_html())).into_response()
        }
    }

    if let Err(err) = check_path(&request.name) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let new_path = path.join(&request.name).with_extension("cook");

    tracing::info!("Creating recipe '{}'", &new_path);

    let content = r#"---
time: 30 min
servings: 4
description: ...
---

..."#;

    let mut file = match tokio::fs::OpenOptions::new().write(true).create_new(true).open(&new_path).await {
        Ok(file) => file,
        Err(err) => {
            tracing::error!("Cannot create recipe {new_path}: {err}");
            return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
        }
    };

    if let Err(err) = file.write_all(content.as_bytes()).await {
        tracing::error!("Cannot write recipe {new_path}: {err}");
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }
    
    let _ = state.updates_sender.send(Update::Modified {
        path: new_path
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
}
