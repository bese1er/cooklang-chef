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
    handlers::{check_path, clean_path, mj_ok, ok_status},
    locale::UserLocale
};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct RunRequest {
    content: String,
}

pub async fn load_editor(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
) -> Response {
    if state.disable_open_editor {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if let Err(e) = check_path(&path) {
        return e.into_response();
    }
    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);
    let recipe = ok_status!(tokio::fs::read_to_string(&entry.path()).await, NOT_FOUND);
    let src_path = clean_path(entry.path(), &state.base_path);

    let tmpl = mj_ok!(state.templates.get_template("editor.html"));

    let ctx = context! {
        t,
        path => path,
        recipe => recipe,
        href => format!("/r/{}", src_path.with_extension("")),
        src_path
    };
    let content = mj_ok!(tmpl.render(ctx));
    Html(content).into_response()
}

pub async fn update_recipe(
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

    let entry = match state.recipe_index.get(&path).await {
        Ok(entry) => entry,
        Err(_) => return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response(),
    };

    tracing::info!("Saving '{}'", entry.path());

    if let Err(err) = tokio::fs::write(&entry.path(), &request.content).await {
        tracing::error!("Cannot save recipe {}: {err}", &entry.path());
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }
    
    let _ = state.updates_sender.send(Update::Modified {
        path: entry.path().into()
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
}
