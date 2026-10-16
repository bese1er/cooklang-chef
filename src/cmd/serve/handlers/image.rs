use axum::{
    extract::{Multipart, Path, Query, State}, 
    http::StatusCode, 
    response::{Html, IntoResponse, Response},
};
use camino::{Utf8Path, Utf8PathBuf};
use cooklang::Recipe;
use minijinja::context;

use crate::cmd::serve::{
    S, 
    async_index::Update, 
    handlers::{check_path, mj_ok, ok_status}, 
    locale::UserLocale
};
use std::io;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct IndexParams {
    sect: Option<u16>,
    step: Option<u16>,
}

#[derive(Deserialize)]
pub struct IndexParamsWithUp {
    sect: Option<u16>,
    step: Option<u16>,
    to_up: bool,
}

pub async fn upload_image(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    Query(params): Query<IndexParams>,
    mut multipart: Multipart,
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

    if let Err(err) = check_path(&path) {
        return (err, mj_ok!(err_html())).into_response();
    }

    let entry = match state.recipe_index.get(&path).await {
        Ok(entry) => entry,
        Err(_) => return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response(),
    };

    let mut image_data: Option<bytes::Bytes> = None;
    let mut image_extension: Option<String> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
            }
        };

        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "image" => {
                let filename = field.file_name().unwrap_or("image").to_string();

                let extension = match std::path::Path::new(&filename).extension().and_then(|ext| ext.to_str()) {
                    Some(ext) => ext.to_ascii_lowercase(),
                    None => {
                        return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
                    }
                };

                if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
                    return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
                }

                let data = match field.bytes().await {
                    Ok(data) => data,
                    Err(_) => {
                        return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
                    }
                };

                image_extension = Some(extension);
                image_data = Some(data);
            }

            _ => {}
        }
    }

    let (Some(data), Some(extension)) =
        (image_data, image_extension)
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let filename = match (sect.is_none(), step.is_none()) {

        (true, true) => format!("{}.{extension}", entry.name()),

        (false, false) => format!("{}.{}.{}.{extension}", entry.name(), sect.map(|v| v.to_string()).unwrap_or_default(), step.map(|v| v.to_string()).unwrap_or_default()),

        _ => {
            return (StatusCode::BAD_REQUEST, mj_ok!(err_html())).into_response();
        }
    };

    let Some(image_dir) = entry.path().parent() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    };

    // Construire le chemin final.
    let destination = image_dir.join(&filename);

    tracing::info!("Saving image : {:?}", destination);

    // Écrire les données sur le disque.
    if let Err(err) = tokio::fs::write(&destination, &data).await {
        tracing::error!("Cannot save image {:?} : {}", destination, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response();
    }

    let _ = state.updates_sender.send(Update::Modified {
        path: entry.path().to_path_buf(),
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
}

pub async fn move_image(
    UserLocale(t): UserLocale,
    Path(path): Path<String>,
    State(state): State<S>,
    Query(params): Query<IndexParamsWithUp>,
) -> Response {
    let sect = params.sect;
    let step = params.step;
    let to_up = params.to_up;
    
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
    let content = ok_status!(tokio::fs::read_to_string(&entry.path()).await, NOT_FOUND);
    let Some(res) = tokio::task::block_in_place(|| {
        state
            .parser
            .parse(&content)
            .into_output()
    }) else {
        return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response()
    };

    let Some(image) = entry.image(sect, step) else {
        return (StatusCode::NOT_FOUND, mj_ok!(err_html())).into_response()
    };

    let Some((dest_sect, dest_step)) = get_destination(&res, sect, step, to_up) else {
        return (StatusCode::BAD_REQUEST, mj_ok!(toast_html("editor.error", "red"))).into_response()
    };

    let dest_path = match entry.image(dest_sect, dest_step) {
        Some(i) => i.path.to_path_buf(),
        _ => change_image_path(&image.path, dest_sect, dest_step)
    };

    tracing::info!("Moving image: {:?} to {:?}", image.path, dest_path);

    if let Err(e) = swap_or_move(&image.path, &dest_path).await {
        tracing::error!("Cannot move image: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, mj_ok!(err_html())).into_response()
    }

    let _ = state.updates_sender.send(Update::Modified {
        path: entry.path().to_path_buf(),
    });
    mj_ok!(toast_html("editor.success", "green")).into_response()
    
}

fn get_destination(
    res: &Recipe,
    sect: Option<u16>,
    step: Option<u16>,
    to_up: bool,
) -> Option<(Option<u16>, Option<u16>)> {
    match (sect, step, to_up) {
        (None, None, true) => None,
        (None, None, false) => Some((Some(0), Some(0))),
        (Some(0), Some(0), true) => Some((None, None)),
        (Some(s), Some(st), false) => {
            let section = res.sections.get(s as usize)?;
            let st_idx = st as usize;
            if st_idx + 1 < section.content.len() {
                Some((Some(s), Some(st.checked_add(1)?)))
            } else if (s as usize) < res.sections.len() - 1 {
                Some((Some(s.checked_add(1)?), Some(0)))
            } else {
                None
            }
        }
        (Some(s), Some(_), true)
            if (s as usize) >= res.sections.len() =>
        {
            let last_s = res.sections.len().checked_sub(1)?;
            let section = res.sections.get(last_s)?;
            let last_st = section.content.len().checked_sub(1)?;
            Some((
                Some(u16::try_from(last_s).ok()?),
                Some(u16::try_from(last_st).ok()?),
            ))
        }
        (Some(s), Some(st), true)
            if (st as usize) >= res.sections.get(s as usize)?.content.len() =>
        {
            let section = res.sections.get(s as usize)?;
            let last_st = section.content.len().checked_sub(1)?;

            Some((
                Some(s),
                Some(u16::try_from(last_st).ok()?),
            ))
        }
        (Some(s), Some(0), true) => {
            let previous_s = (s as usize).checked_sub(1)?;
            let section = res.sections.get(previous_s)?;
            let last_st = section.content.len().checked_sub(1)?;
            Some((
                Some(u16::try_from(previous_s).ok()?),
                Some(u16::try_from(last_st).ok()?),
            ))
        }
        (Some(s), Some(st), true) => {
            Some((Some(s), Some(st.checked_sub(1)?)))
        }
        _ => None,
    }
}

fn change_image_path(
    image_path: &Utf8PathBuf,
    dest_sect: Option<u16>,
    dest_step: Option<u16>,
) -> Utf8PathBuf {
    let parent = image_path.parent().unwrap_or(Utf8Path::new(""));
    let extension = image_path.extension();
    let stem = image_path.file_stem().unwrap_or("");

    let prefix = stem
        .rsplit_once('.')
        .and_then(|(before_step, _)| before_step.rsplit_once('.'))
        .map(|(before_sect, _)| before_sect)
        .unwrap_or(stem);

    let filename = match (dest_sect, dest_step) {
        (Some(sect), Some(step)) => format!("{prefix}.{sect}.{step}"),
        (None, None) => prefix.to_string(),
        _ => stem.to_string(),
    };

    let filename = match extension {
        Some(ext) => format!("{filename}.{ext}"),
        None => filename,
    };

    parent.join(filename)
}

async fn swap_or_move(
    source: &Utf8Path,
    destination: &Utf8Path,
) -> io::Result<()> {
    if !tokio::fs::try_exists(destination).await? {
        return tokio::fs::rename(source, destination).await;
    }

    let temp = destination.with_extension("tmp");
    if tokio::fs::try_exists(&temp).await? {
        tokio::fs::remove_file(&temp).await?;
    }
    tokio::fs::rename(destination, &temp).await?;

    if let Err(e) = tokio::fs::rename(source, destination).await {
        let _ = tokio::fs::rename(&temp, destination).await;
        return Err(e);
    }
    if let Err(e) = tokio::fs::rename(&temp, source).await {
        let _ = tokio::fs::rename(destination, source).await;
        let _ = tokio::fs::rename(&temp, destination).await;
        return Err(e);
    }

    Ok(())
}