use axum::{
    Json,
    extract::{Path, Query, State}, 
    http::{HeaderMap, StatusCode}, 
    response::{Html, IntoResponse, Response},
};
use cooklang::Quantity;
use minijinja::context;

use crate::cmd::serve::{
    S, 
    handlers::ok_status,
    handlers::{check_path, clean_path, mj_ok}, 
    locale::UserLocale
};

use serde::Deserialize;

pub async fn convert_popover(
    UserLocale(t): UserLocale,
    State(state): State<S>,
    headers: HeaderMap,
    Json(quantity): Json<Quantity>,
) -> Response {
    let converter = state.parser.converter();

    let triggered_by = match headers.get("HX-Trigger").map(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    if quantity.unit().is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let unit = match quantity.unit_info(converter) {
        Some(unit) => unit,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let conversions: Vec<_> = converter
        .best_units(unit.physical_quantity, None)
        .into_iter()
        .filter_map(|target| {
            let mut q = quantity.clone();
            q.convert(&target, converter).ok()?;
            Some(q)
        })
        .collect();

    let tmpl = mj_ok!(state
        .templates
        .get_template("components/convert_popover.html"));
    let html = mj_ok!(tmpl.render(context! { t, conversions, triggered_by }));
    Html(html).into_response()
}

#[derive(Deserialize)]
pub struct IndexParams {
    sect: Option<u16>,
    step: Option<u16>,
}

pub async fn image_popover(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    headers: HeaderMap,
    Query(params): Query<IndexParams>,
) -> Response {
    let triggered_by = match headers.get("HX-Trigger").map(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sect = params.sect;
    let step = params.step;

    if let Err(err) = check_path(&path) {
        return err.into_response()
    }
    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);
    let src_path = clean_path(entry.path(), &state.base_path);
    let is_new = entry.image(sect, step).is_none();

    let tmpl = mj_ok!(state.templates.get_template("components/image_popover.html"));
    let html = mj_ok!(tmpl.render(context! { t, triggered_by, is_new, sect, step, src_path }));
    Html(html).into_response()
}

pub async fn directory_popover(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    headers: HeaderMap,
) -> Response {   
    let triggered_by = match headers.get("HX-Trigger").map(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    if let Err(err) = check_path(&path) {
        return err.into_response()
    }

    let old_path = state.base_path.join(&path);

    if !old_path.exists() || !old_path.is_dir() {
        return StatusCode::BAD_REQUEST.into_response();
    }    

    let tmpl = mj_ok!(state.templates.get_template("components/it_popover.html"));
    let html = mj_ok!(tmpl.render(context! { t, triggered_by, src_path => path, type=>"d", old_name=> old_path.file_name()}));
    Html(html).into_response()
}

pub async fn recipe_popover(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    headers: HeaderMap,
) -> Response {   
    let triggered_by = match headers.get("HX-Trigger").map(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    if let Err(err) = check_path(&path) {
        return err.into_response()
    }
    let entry = ok_status!(state.recipe_index.get(&path).await, NOT_FOUND);
    let src_path = clean_path(entry.path(), &state.base_path);

    let tmpl = mj_ok!(state.templates.get_template("components/it_popover.html"));
    let html = mj_ok!(tmpl.render(context! { t, triggered_by, src_path, type=>"r", old_name=> &entry.name()}));
    Html(html).into_response()
}