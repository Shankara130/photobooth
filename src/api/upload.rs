//! POST /api/photos/upload — upload JPEG hasil capture klien (mode browser/iPad)
//! lewat multipart field `photo`.

use axum::extract::Multipart;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use super::AppError;
use crate::state::AppState;
use crate::store::{self, MediaMeta};

pub async fn upload(
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<MediaMeta>), AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart tidak valid: {e}")))?
    {
        if field.name() != Some("photo") {
            continue;
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("baca field gagal: {e}")))?;
        if !store::is_valid_jpeg(&bytes) {
            return Err(AppError::BadRequest(
                "bukan JPEG valid atau ukuran di luar batas (1 KiB..30 MiB)".into(),
            ));
        }
        let meta = store::save_media(&st.data_dir, "photos", "photo", &bytes).await?;
        return Ok((StatusCode::CREATED, Json(meta)));
    }
    Err(AppError::BadRequest(
        "field multipart 'photo' tidak ditemukan".into(),
    ))
}
