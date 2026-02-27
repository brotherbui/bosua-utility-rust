//! Notification system with pluggable handlers.
//!
//! The `NotificationManager` dispatches notifications to all registered handlers.
//! Built-in handlers: `LogHandler`, `StoreHandler`, `SseHandler`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// ---------------------------------------------------------------------------
// Data models (Task 15.3)
// ---------------------------------------------------------------------------

/// A notification emitted by the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub trigger: NotificationTrigger,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The event that triggered a notification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationTrigger {
    FshareNewFiles,
    DownloadComplete,
    SystemAlert,
    GdriveSyncComplete,
    GdriveSyncFailed,
}

// ---------------------------------------------------------------------------
// Handler trait (Task 15.1)
// ---------------------------------------------------------------------------

/// Async trait implemented by notification handlers.
#[async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Human-readable name of this handler.
    fn name(&self) -> &str;

    /// Deliver a notification through this handler.
    async fn send(&self, notification: &Notification) -> crate::errors::Result<()>;
}

// ---------------------------------------------------------------------------
// NotificationManager (Task 15.1)
// ---------------------------------------------------------------------------

/// Central dispatcher that fans-out notifications to every registered handler.
pub struct NotificationManager {
    handlers: RwLock<Vec<Box<dyn NotificationHandler>>>,
    enabled: AtomicBool,
}

impl NotificationManager {
    /// Create a new, enabled `NotificationManager` wrapped in an `Arc`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            handlers: RwLock::new(Vec::new()),
            enabled: AtomicBool::new(true),
        })
    }

    /// Register a handler that will receive future notifications.
    pub async fn register_handler(&self, handler: impl NotificationHandler + 'static) {
        self.handlers.write().await.push(Box::new(handler));
    }

    /// Dispatch `notification` to every registered handler.
    ///
    /// Individual handler failures are logged but do not prevent delivery to
    /// the remaining handlers.  If the manager is disabled the call is a no-op.
    pub async fn send(&self, notification: Notification) -> crate::errors::Result<()> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.send(&notification).await {
                tracing::warn!(handler = handler.name(), "Notification handler failed: {}", e);
            }
        }
        Ok(())
    }

    /// Returns `true` when the manager is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Enable or disable notification dispatch.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Built-in handlers (Task 15.2)
// ---------------------------------------------------------------------------

/// Logs every notification via `tracing::info!`.
pub struct LogHandler;

#[async_trait]
impl NotificationHandler for LogHandler {
    fn name(&self) -> &str {
        "log"
    }

    async fn send(&self, notification: &Notification) -> crate::errors::Result<()> {
        tracing::info!(
            trigger = ?notification.trigger,
            title = %notification.title,
            "Notification: {}", notification.message
        );
        Ok(())
    }
}

/// Stores notifications in an in-memory bounded deque.
pub struct StoreHandler {
    store: Arc<RwLock<VecDeque<Notification>>>,
}

impl StoreHandler {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Get a clone of the backing store for reading.
    pub fn store(&self) -> Arc<RwLock<VecDeque<Notification>>> {
        self.store.clone()
    }
}

#[async_trait]
impl NotificationHandler for StoreHandler {
    fn name(&self) -> &str {
        "store"
    }

    async fn send(&self, notification: &Notification) -> crate::errors::Result<()> {
        self.store.write().await.push_back(notification.clone());
        Ok(())
    }
}

/// Broadcasts notifications over a `tokio::sync::broadcast` channel for SSE.
pub struct SseHandler {
    tx: broadcast::Sender<Notification>,
}

impl SseHandler {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe to the broadcast stream (e.g. for an SSE endpoint).
    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.tx.subscribe()
    }
}

#[async_trait]
impl NotificationHandler for SseHandler {
    fn name(&self) -> &str {
        "sse"
    }

    async fn send(&self, notification: &Notification) -> crate::errors::Result<()> {
        // Ignore SendError – it just means no active receivers.
        let _ = self.tx.send(notification.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // -- helpers --

    fn sample_notification(trigger: NotificationTrigger) -> Notification {
        Notification {
            id: "test-1".into(),
            trigger,
            title: "Test".into(),
            message: "hello".into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// A test handler that counts how many notifications it received.
    struct CountingHandler {
        count: Arc<AtomicUsize>,
    }

    impl CountingHandler {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let count = Arc::new(AtomicUsize::new(0));
            (Self { count: count.clone() }, count)
        }
    }

    #[async_trait]
    impl NotificationHandler for CountingHandler {
        fn name(&self) -> &str {
            "counting"
        }
        async fn send(&self, _notification: &Notification) -> crate::errors::Result<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    // -- NotificationManager tests --

    #[tokio::test]
    async fn manager_dispatches_to_all_handlers() {
        let mgr = NotificationManager::new();
        let (h1, c1) = CountingHandler::new();
        let (h2, c2) = CountingHandler::new();
        mgr.register_handler(h1).await;
        mgr.register_handler(h2).await;

        mgr.send(sample_notification(NotificationTrigger::SystemAlert))
            .await
            .unwrap();

        assert_eq!(c1.load(Ordering::SeqCst), 1);
        assert_eq!(c2.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn manager_skips_when_disabled() {
        let mgr = NotificationManager::new();
        let (h, count) = CountingHandler::new();
        mgr.register_handler(h).await;

        mgr.set_enabled(false);
        mgr.send(sample_notification(NotificationTrigger::SystemAlert))
            .await
            .unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn manager_enabled_flag_toggles() {
        let mgr = NotificationManager::new();
        assert!(mgr.is_enabled());
        mgr.set_enabled(false);
        assert!(!mgr.is_enabled());
        mgr.set_enabled(true);
        assert!(mgr.is_enabled());
    }

    // -- LogHandler test --

    #[tokio::test]
    async fn log_handler_does_not_error() {
        let handler = LogHandler;
        assert_eq!(handler.name(), "log");
        handler
            .send(&sample_notification(NotificationTrigger::DownloadComplete))
            .await
            .unwrap();
    }

    // -- StoreHandler tests --

    #[tokio::test]
    async fn store_handler_persists_notifications() {
        let handler = StoreHandler::new();
        let store = handler.store();

        handler
            .send(&sample_notification(NotificationTrigger::FshareNewFiles))
            .await
            .unwrap();
        handler
            .send(&sample_notification(NotificationTrigger::GdriveSyncComplete))
            .await
            .unwrap();

        let guard = store.read().await;
        assert_eq!(guard.len(), 2);
        assert_eq!(guard[0].trigger, NotificationTrigger::FshareNewFiles);
        assert_eq!(guard[1].trigger, NotificationTrigger::GdriveSyncComplete);
    }

    // -- SseHandler tests --

    #[tokio::test]
    async fn sse_handler_broadcasts_to_subscriber() {
        let handler = SseHandler::new(16);
        let mut rx = handler.subscribe();

        handler
            .send(&sample_notification(NotificationTrigger::GdriveSyncFailed))
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.trigger, NotificationTrigger::GdriveSyncFailed);
        assert_eq!(received.id, "test-1");
    }

    #[tokio::test]
    async fn sse_handler_no_receivers_does_not_error() {
        let handler = SseHandler::new(16);
        // No subscribers – should still succeed.
        handler
            .send(&sample_notification(NotificationTrigger::SystemAlert))
            .await
            .unwrap();
    }

    // -- Notification serde round-trip --

    #[test]
    fn notification_serde_round_trip() {
        let n = sample_notification(NotificationTrigger::DownloadComplete);
        let json = serde_json::to_string(&n).unwrap();
        let deserialized: Notification = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, n.id);
        assert_eq!(deserialized.trigger, n.trigger);
        assert_eq!(deserialized.title, n.title);
        assert_eq!(deserialized.message, n.message);
    }

    #[test]
    fn notification_trigger_serde_round_trip() {
        let triggers = [
            NotificationTrigger::FshareNewFiles,
            NotificationTrigger::DownloadComplete,
            NotificationTrigger::SystemAlert,
            NotificationTrigger::GdriveSyncComplete,
            NotificationTrigger::GdriveSyncFailed,
        ];
        for t in triggers {
            let json = serde_json::to_string(&t).unwrap();
            let back: NotificationTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(back, t);
        }
    }
}
