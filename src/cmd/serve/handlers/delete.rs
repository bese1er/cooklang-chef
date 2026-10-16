use axum::{
    extract::{Path, Query, State}, 
    http::StatusCode, 
    response::{Html, IntoResponse, Response},
};
use minijinja::context;

use crate::cmd::serve::{
    S, 
    async_index::Update, 
    handlers::{check_path, mj_ok, ok_status}, 
    locale::UserLocale
};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct IndexParams {
    sect: Option<u16>,
    step: Option<u16>,
}

pub async fn delete_image(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    Query(params): Query<IndexParams>,
) -> Response {
    let sect = params.sect;
    let step = params.step;
    
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

    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);

    let Some(image) = entry.image(sect, step) else {
        return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response()
    };
    
    if let Err(err) = tokio::fs::remove_file(&image.path).await {
        tracing::error!("Impossible de supprimer l'image {:?} : {}", image.path, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }

    tracing::info!("Image supprimée avec succès : {:?}", image.path);
    let _ = state.updates_sender.send(Update::Modified {
        path: entry.path().to_path_buf(),
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
    
}