//! POST /api/trigger/inject — injeksi pemicu sintetis (dev/testing tanpa DSLR).
//! Mode `shot` menyertakan foto sungguhan dari backend kamera; mode lain
//! broadcast trigger tanpa foto.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::state::AppState;
use crate::trigger::TriggerMode;

pub async fn inject(State(st): State<AppState>) -> Response {
    if st.trigger_mode == TriggerMode::Off {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "TRIGGER_MODE=off — endpoint nonaktif" })),
        )
            .into_response();
    }
    let photo = if st.trigger_mode == TriggerMode::Shot && st.camera.name() != "browser" {
        let cam = st.camera.clone();
        match tokio::task::spawn_blocking(move || cam.capture()).await {
            Ok(Ok(jpeg)) => {
                crate::store::save_media(&st.data_dir, "photos", "photo", &jpeg)
                    .await
                    .ok()
            }
            Ok(Err(e)) => {
                tracing::warn!("inject capture gagal: {e}");
                None
            }
            Err(e) => {
                tracing::warn!("inject task capture gagal: {e}");
                None
            }
        }
    } else {
        None
    };
    let payload = serde_json::json!({
        "type": "trigger",
        "mode": st.trigger_mode.as_str(),
        "photo": photo,
    });
    let _ = st.events.send(payload.to_string());
    (StatusCode::OK, Json(serde_json::json!({ "ok": true, "photo": photo }))).into_response()
}
