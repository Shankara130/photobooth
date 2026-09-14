//! GET /api/events — Server-Sent Events: pemicu shutter kamera → browser.
//! Payload trigger: `{"type":"trigger","mode":"...","photo":MediaMeta|null}`.

use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

pub async fn events(State(st): State<AppState>) -> axum::response::Response {
    let mut rx = st.events.subscribe();
    let stream = async_stream::stream! {
        // Sapaan awal — bukti koneksi hidup sebelum trigger pertama.
        yield Ok::<Event, Infallible>(
            Event::default().event("hello").data(r#"{"type":"hello"}"#),
        );
        loop {
            match rx.recv().await {
                Ok(msg) => yield Ok(Event::default().event("trigger").data(msg)),
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("subscriber SSE ketinggalan {n} event");
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}
