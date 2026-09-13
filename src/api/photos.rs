//! GET /api/photos — daftar foto & strip terbaru (discan dari disk).

use axum::extract::State;
use axum::Json;
use serde_json::json;

use super::AppError;
use crate::state::AppState;
use crate::store;

pub async fn list_photos(State(st): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let items = store::list_media(&st.data_dir).await?;
    Ok(Json(json!({ "items": items })))
}
