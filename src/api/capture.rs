//! POST /api/capture — ambil foto dari kamera server (gphoto2/mock), simpan,
//! balas metadata.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::camera::{CameraError, CameraResult};
use crate::state::AppState;
use crate::store::{self, MediaMeta};

pub async fn capture(State(st): State<AppState>) -> CameraResult<(StatusCode, Json<MediaMeta>)> {
    let cam = st.camera.clone();
    let jpeg = tokio::task::spawn_blocking(move || cam.capture())
        .await
        .map_err(|e| CameraError::Unavailable(format!("task kamera gagal: {e}")))??;
    let meta = store::save_media(&st.data_dir, "photos", "photo", &jpeg).await?;
    Ok((StatusCode::CREATED, Json(meta)))
}
