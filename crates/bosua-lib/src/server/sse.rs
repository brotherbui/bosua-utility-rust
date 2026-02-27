//! Server-Sent Events (SSE) endpoint for real-time notifications.
//!
//! Streams notifications from the `SseHandler`'s broadcast channel to
//! connected HTTP clients. Each notification is serialized as a JSON
//! SSE `data` frame with the event type set to the notification trigger.

use crate::notifications::{Notification, SseHandler};
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Shared application state holding the SSE broadcast sender.
#[derive(Clone)]
pub struct SseState {
    pub sse_handler: Arc<SseHandler>,
}

/// SSE endpoint handler.
///
/// Subscribes to the `SseHandler`'s broadcast channel and streams each
/// notification as a JSON-encoded SSE event. The event name is derived
/// from the notification trigger (e.g. `"DownloadComplete"`).
///
/// A keep-alive comment is sent every 15 seconds to prevent proxies and
/// load balancers from closing idle connections.
pub async fn notifications_sse_handler(
    State(state): State<SseState>,
) -> impl IntoResponse {
    let rx = state.sse_handler.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(notification) => Some(notification_to_event(notification)),
        Err(_) => None, // lagged — skip missed messages
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Convert a `Notification` into an SSE `Event`.
fn notification_to_event(notification: Notification) -> Result<Event, Infallible> {
    let event_name = format!("{:?}", notification.trigger);
    let data = serde_json::to_string(&notification).unwrap_or_default();
    Ok(Event::default().event(event_name).data(data))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::NotificationTrigger;
    use chrono::Utc;
    use std::collections::HashMap;

    fn sample_notification() -> Notification {
        Notification {
            id: "sse-test-1".into(),
            trigger: NotificationTrigger::DownloadComplete,
            title: "Download done".into(),
            message: "file.zip finished".into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn notification_to_event_produces_valid_event() {
        let n = sample_notification();
        let event = notification_to_event(n).unwrap();
        // Event was created without error — axum Event doesn't expose
        // fields publicly, so we just verify it doesn't panic.
        let _ = event;
    }

    #[test]
    fn sse_state_is_clone() {
        let handler = Arc::new(SseHandler::new(16));
        let state = SseState {
            sse_handler: handler.clone(),
        };
        let _cloned = state.clone();
    }
}
