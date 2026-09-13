//! GET /api/preview.mjpg — live preview MJPEG (multipart/x-mixed-replace),
//! kompatibel Safari/iPad via tag `<img>`. Frame diambil ±5 fps dari kamera
//! server (gphoto2/mock); mode browser memakai getUserMedia di klien (503).

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::camera::CameraError;
use crate::state::AppState;

const BOUNDARY: &str = "photobooth";
const FRAME_INTERVAL_MS: u64 = 200; // ±5 fps
const RETRY_DELAY_MS: u64 = 1000;

pub async fn preview_mjpeg(State(st): State<AppState>) -> Response {
    if st.camera.name() == "browser" {
        return CameraError::Unavailable(
            "mode browser: preview langsung dari kamera klien (getUserMedia)".into(),
        )
        .into_response();
    }

    let cam = st.camera.clone();
    let stream = async_stream::stream! {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(FRAME_INTERVAL_MS));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let c = cam.clone();
            match tokio::task::spawn_blocking(move || c.preview()).await {
                Ok(Ok(jpeg)) => {
                    let mut part = format!(
                        "--{BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                        jpeg.len()
                    )
                    .into_bytes();
                    part.extend_from_slice(&jpeg);
                    part.extend_from_slice(b"\r\n");
                    yield Ok::<axum::body::Bytes, std::io::Error>(part.into());
                }
                // error transien (kamera sibuk/lepas) → tunggu lalu coba lagi,
                // jangan matikan <img> di sisi klien
                Ok(Err(e)) => {
                    tracing::warn!("preview gagal: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                }
                // join error (shutdown) → akhiri stream
                Err(e) => {
                    yield Err(std::io::Error::other(e));
                }
            }
        }
    };

    (
        [
            (header::CONTENT_TYPE, format!("multipart/x-mixed-replace; boundary={BOUNDARY}")),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}
