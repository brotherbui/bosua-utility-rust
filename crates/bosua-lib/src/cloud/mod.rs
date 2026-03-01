pub mod account_manager;
pub mod aws;
pub mod cloudflare;
pub mod fshare;
pub mod gcp;
pub mod gdrive;
pub mod tailscale;

use async_trait::async_trait;

use crate::errors::Result;

/// Common trait for all cloud service integrations.
///
/// Each cloud provider (GDrive, GCP, AWS, Cloudflare, Tailscale, FShare)
/// implements this trait plus its own domain-specific traits.
#[async_trait]
pub trait CloudClient: Send + Sync {
    /// Returns the display name of this cloud provider (e.g. "Google Drive", "AWS").
    fn name(&self) -> &str;

    /// Authenticates with the cloud provider.
    ///
    /// Implementations handle provider-specific auth flows (OAuth2, API keys, etc.).
    async fn authenticate(&self) -> Result<()>;
}
