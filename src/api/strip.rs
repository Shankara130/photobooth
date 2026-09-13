//! POST /api/strip — susun foto-foto (id) jadi photo strip vertikal.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use super::AppError;
use crate::state::AppState;
use crate::store::{self, MediaMeta};

#[derive(Debug, Deserialize)]
pub struct StripReq {
    pub ids: Vec<String>,
}

pub async fn compose(
    State(st): State<AppState>,
    Json(req): Json<StripReq>,
) -> Result<(StatusCode, Json<MediaMeta>), AppError> {
    if req.ids.is_empty() || req.ids.len() > 8 {
        return Err(AppError::BadRequest("ids harus berisi 1..=8 foto".into()));
    }

    let mut frames = Vec::with_capacity(req.ids.len());
    for id in &req.ids {
        let id = sanitize_id(id)?;
        let path = store::photo_path(&st.data_dir, &id);
        let jpeg = tokio::fs::read(&path)
            .await
            .map_err(|_| AppError::NotFound(format!("foto {id} tidak ditemukan")))?;
        frames.push(jpeg);
    }

    let taken = chrono::Local::now();
    let jpeg = tokio::task::spawn_blocking(move || crate::strip::compose_strip(&frames, taken))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let meta = store::save_media(&st.data_dir, "strips", "strip", &jpeg).await?;
    Ok((StatusCode::CREATED, Json(meta)))
}

/// Id hanya boleh alfanumerik ASCII (uuid simple) — tolak path traversal.
fn sanitize_id(id: &str) -> Result<String, AppError> {
    if !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(id.to_string())
    } else {
        Err(AppError::BadRequest(format!("id tidak valid: {id:?}")))
    }
}
