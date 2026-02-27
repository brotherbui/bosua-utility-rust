use tokio_util::sync::CancellationToken;

/// Handles graceful shutdown by intercepting SIGINT (Ctrl+C) and SIGTERM signals.
///
/// Uses a `CancellationToken` to broadcast shutdown to all listeners.
/// Spawn `listen()` as a background task, then pass `token()` clones to
/// any subsystem that needs to react to shutdown.
pub struct SignalHandler {
    token: CancellationToken,
}

impl SignalHandler {
    /// Create a new signal handler with a fresh cancellation token.
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Returns a clone of the cancellation token for sharing with subsystems.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Listens for SIGINT or SIGTERM and cancels the token when received.
    ///
    /// This method blocks until a signal arrives, then cancels the token
    /// so all holders can begin graceful shutdown.
    pub async fn listen(&self) {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received SIGINT, shutting down...");
            }
            _ = Self::sigterm() => {
                tracing::info!("Received SIGTERM, shutting down...");
            }
        }
        self.token.cancel();
    }

    #[cfg(unix)]
    async fn sigterm() {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    }

    #[cfg(not(unix))]
    async fn sigterm() {
        // On non-Unix platforms, SIGTERM doesn't exist — wait forever.
        std::future::pending::<()>().await;
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_uncancelled_token() {
        let handler = SignalHandler::new();
        assert!(!handler.token().is_cancelled());
    }

    #[test]
    fn test_token_clones_share_state() {
        let handler = SignalHandler::new();
        let t1 = handler.token();
        let t2 = handler.token();

        assert!(!t1.is_cancelled());
        assert!(!t2.is_cancelled());

        // Cancelling the original token propagates to clones
        handler.token.cancel();
        assert!(t1.is_cancelled());
        assert!(t2.is_cancelled());
    }

    #[test]
    fn test_default_impl() {
        let handler = SignalHandler::default();
        assert!(!handler.token().is_cancelled());
    }
}
