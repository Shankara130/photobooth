//! GET /api/qr?u=<url> — render QR code PNG untuk link unduhan.

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;

use super::AppError;
use crate::state::AppState;

const MAX_URL_LEN: usize = 512;

pub async fn qr(
    State(_st): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let url = params
        .get("u")
        .ok_or_else(|| AppError::BadRequest("parameter 'u' wajib diisi".into()))?;
    if url.len() > MAX_URL_LEN || !url.starts_with("http") {
        return Err(AppError::BadRequest(format!(
            "parameter 'u' harus URL http(s) maksimal {MAX_URL_LEN} karakter"
        )));
    }

    let code = qrcode::QrCode::new(url.as_bytes())
        .map_err(|e| AppError::BadRequest(format!("URL tidak bisa dienkode: {e}")))?;
    let img = code
        .render::<image::Luma<u8>>()
        .min_dimensions(320, 320)
        .build();

    let mut png = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(format!("encode PNG gagal: {e}")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        png,
    )
        .into_response())
}
