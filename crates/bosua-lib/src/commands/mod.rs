pub mod araxis;
pub mod aria2_cmd;
pub mod aws;
pub mod bitcoin;
pub mod checksum;
pub mod cloud;
pub mod cloudflare;
pub mod config_cmd;
pub mod cron;
pub mod crx;
pub mod daemon;
pub mod detect;
pub mod download;
pub mod fshare;
pub mod fshare_scan;
pub mod gcloud;
pub mod gcp;
pub mod gdrive;
pub mod gdrive_sync;
pub mod info;
pub mod ip;
pub mod latex2pdf;
pub mod latex2typst;
#[cfg(feature = "macos")]
pub mod mac;
pub mod md2pdf;
pub mod medium;
pub mod memprofile;
pub mod ocr;
pub mod onflix;
pub mod play;
pub mod proxy;
pub mod registry_cmd;
pub mod scraper_api;
pub mod search;
pub mod serve;
pub mod sms_activate;
pub mod tailscale;
pub mod testmail;
pub mod version;
pub mod vmf;
pub mod winrar;

use clap::ArgMatches;

use crate::cli::{CommandBuilder, CommandCategory, CommandRegistry};
use crate::commands::registry_cmd::ServiceRegistry;
use crate::errors::{BosuaError, Result};

/// Register the full 40+ command set for the macOS variant.
///
/// Matches Go's `app.go` `RegisterCommands()` which calls:
/// AddVersionCmd, AddCommonCmd (config/cert/obf), AddRegistryCmd, AddMemProfileCmd,
/// AddIpCmd, AddTestmailCmd, AddSmsActivateCmd, AddSearchCmd, AddScraperApiCmd,
/// AddProxyCmd, AddPlayCmd, AddMacCmd, AddChecksumCmd, AddDetectCmd, AddAraxisCmd,
/// AddAria2Cmd, AddBitcoinCmd, AddCrxCmd, AddDownloadCmd, AddFshareCmd,
/// AddFshareScanCmd, AddGcpCmd, AddCloudCmd, AddGdriveCmd, AddGcloudCmd,
/// AddHttpCmd, AddVmfCmd, AddTailscaleCmd, AddMediumCmd, addAwsCmd,
/// AddMd2PdfCmd, AddLatex2PdfCmd, AddLatex2TypstCmd, AddCloudflareCmd,
/// AddWinrarCmd, AddOCRCmd, AddOnflixCmd, AddGdriveSyncCmd
#[cfg(feature = "macos")]
pub fn register_macos_commands(registry: &mut CommandRegistry) {
    // version
    registry
        .register(version::version_meta())
        .expect("failed to register version command");

    // AddCommonCmd() registers: config (aliases: i, c, conf, info), cert, obf
    registry
        .register(config_cmd::config_meta())
        .expect("failed to register config command");
    registry
        .register(
            CommandBuilder::new("cert")
                .category(CommandCategory::Network)
                .description("Print certificate chains of domain")
                .build(),
        )
        .expect("failed to register cert command");
    registry
        .register(
            CommandBuilder::new("obf")
                .category(CommandCategory::Utility)
                .description("Obfuscate text input for security")
                .build(),
        )
        .expect("failed to register obf command");

    // Utility commands
    registry
        .register(registry_cmd::registry_meta())
        .expect("failed to register registry command");
    registry
        .register(memprofile::memprofile_meta())
        .expect("failed to register memprofile command");
    registry
        .register(ip::ip_meta())
        .expect("failed to register ip command");
    registry
        .register(testmail::testmail_meta())
        .expect("failed to register testmail command");
    registry
        .register(sms_activate::sms_activate_meta())
        .expect("failed to register smsactivate command");
    registry
        .register(search::search_meta())
        .expect("failed to register search command");
    registry
        .register(scraper_api::scraper_api_meta())
        .expect("failed to register scraperapi command");
    registry
        .register(proxy::proxy_meta())
        .expect("failed to register proxy command");
    registry
        .register(play::play_meta())
        .expect("failed to register play command");
    registry
        .register(mac::mac_meta())
        .expect("failed to register macos command");
    registry
        .register(checksum::checksum_meta())
        .expect("failed to register luhn command");
    registry
        .register(detect::detect_meta())
        .expect("failed to register detect command");
    registry
        .register(araxis::araxis_meta())
        .expect("failed to register araxis command");
    registry
        .register(aria2_cmd::aria2_meta())
        .expect("failed to register aria2 command");
    registry
        .register(bitcoin::bitcoin_meta())
        .expect("failed to register bitcoin command");
    registry
        .register(crx::crx_meta())
        .expect("failed to register crx command");
    registry
        .register(download::download_meta())
        .expect("failed to register download command");
    registry
        .register(fshare::fshare_meta())
        .expect("failed to register fshare command");
    registry
        .register(fshare_scan::fshare_scan_meta())
        .expect("failed to register fshare-scan command");
    registry
        .register(gcp::gcp_meta())
        .expect("failed to register gcp command");
    registry
        .register(cloud::cloud_meta())
        .expect("failed to register cloud command");
    registry
        .register(gdrive::gdrive_meta())
        .expect("failed to register gdrive command");
    registry
        .register(gcloud::gcloud_meta())
        .expect("failed to register gcloud command");
    registry
        .register(serve::serve_meta())
        .expect("failed to register serve command");
    registry
        .register(vmf::vmf_meta())
        .expect("failed to register vmf command");
    registry
        .register(tailscale::tailscale_meta())
        .expect("failed to register tailscale command");
    registry
        .register(medium::medium_meta())
        .expect("failed to register medium command");
    registry
        .register(aws::aws_meta())
        .expect("failed to register aws command");
    registry
        .register(md2pdf::md2pdf_meta())
        .expect("failed to register md2pdf command");
    registry
        .register(latex2pdf::latex2pdf_meta())
        .expect("failed to register latex2pdf command");
    registry
        .register(latex2typst::latex2typst_meta())
        .expect("failed to register latex2typst command");
    registry
        .register(cloudflare::cloudflare_meta())
        .expect("failed to register cloudflare command");
    registry
        .register(winrar::winrar_meta())
        .expect("failed to register winrar command");
    registry
        .register(ocr::ocr_meta())
        .expect("failed to register ocr command");
    registry
        .register(onflix::onflix_meta())
        .expect("failed to register onflix command");
    registry
        .register(gdrive_sync::gdrive_sync_meta())
        .expect("failed to register gdrive-sync command");
}

/// Register the reduced command set for the Linux server variant.
///
/// Matches Go's `app_server.go` `RegisterCommands()`:
/// AddVersionCmd, AddInfoCmd, AddCronCmd, AddDaemonCmd, AddDownloadCmd,
/// AddFshareCmd, AddFshareScanCmd, AddGcpCmd, AddGdriveCmd, AddGdriveSyncCmd,
/// AddHttpCmd, AddOnflixCmd
#[cfg(feature = "linux")]
pub fn register_linux_commands(registry: &mut CommandRegistry) {
    registry
        .register(version::version_meta())
        .expect("failed to register version command");
    // AddInfoCmd registers "config" with aliases "conf", "info"
    registry
        .register(info::info_meta())
        .expect("failed to register config/info command");
    registry
        .register(cron::cron_meta())
        .expect("failed to register cron command");
    registry
        .register(daemon::daemon_meta())
        .expect("failed to register daemon command");
    registry
        .register(download::download_meta())
        .expect("failed to register download command");
    registry
        .register(fshare::fshare_meta())
        .expect("failed to register fshare command");
    registry
        .register(fshare_scan::fshare_scan_meta())
        .expect("failed to register fshare-scan command");
    registry
        .register(gcp::gcp_meta())
        .expect("failed to register gcp command");
    registry
        .register(gdrive::gdrive_meta())
        .expect("failed to register gdrive command");
    registry
        .register(gdrive_sync::gdrive_sync_meta())
        .expect("failed to register gdrive-sync command");
    registry
        .register(serve::serve_meta())
        .expect("failed to register serve command");
    registry
        .register(onflix::onflix_meta())
        .expect("failed to register onflix command");
}

/// Register the reduced command set for the GCP server variant.
///
/// Matches Go's `app_server_gcp.go` `RegisterCommands()`:
/// AddVersionCmd, AddInfoCmd, AddCronCmd, AddDaemonCmd, AddDownloadCmd,
/// AddFshareCmd, AddGdriveCmd, AddGdriveSyncCmd, AddGcpCmd, AddHttpCmd,
/// AddOnflixCmd
#[cfg(feature = "gcp")]
pub fn register_gcp_commands(registry: &mut CommandRegistry) {
    registry
        .register(version::version_meta())
        .expect("failed to register version command");
    // AddInfoCmd registers "config" with aliases "conf", "info"
    registry
        .register(info::info_meta())
        .expect("failed to register config/info command");
    registry
        .register(cron::cron_meta())
        .expect("failed to register cron command");
    registry
        .register(daemon::daemon_meta())
        .expect("failed to register daemon command");
    registry
        .register(download::download_meta())
        .expect("failed to register download command");
    registry
        .register(fshare::fshare_meta())
        .expect("failed to register fshare command");
    registry
        .register(gdrive::gdrive_meta())
        .expect("failed to register gdrive command");
    registry
        .register(gdrive_sync::gdrive_sync_meta())
        .expect("failed to register gdrive-sync command");
    registry
        .register(gcp::gcp_meta())
        .expect("failed to register gcp command");
    registry
        .register(serve::serve_meta())
        .expect("failed to register serve command");
    registry
        .register(onflix::onflix_meta())
        .expect("failed to register onflix command");
}

/// Dispatch a parsed command to its handler with the appropriate service dependencies.
///
/// Currently calls the existing stub handlers which are synchronous and take only `&ArgMatches`.
/// As handlers are updated in later tasks, this function will pass service dependencies from
/// the `ServiceRegistry`.
pub async fn dispatch_command(
    name: &str,
    matches: &ArgMatches,
    services: &ServiceRegistry,
) -> Result<()> {
    match name {
        "version" => version::handle_version(matches),
        "config" => config_cmd::handle_config(matches, &services.config_manager).await?,
        "registry" => registry_cmd::handle_registry(matches),
        "memprofile" => memprofile::handle_memprofile(matches)?,
        "ip" => ip::handle_ip(matches, &services.http_client).await?,
        "testmail" => testmail::handle_testmail(matches, &services.http_client).await?,
        "smsactivate" => sms_activate::handle_sms_activate(matches, &services.http_client).await?,
        "search" => {
            search::handle_search(matches, services.search_engine().await?.as_ref()).await?
        }
        "scraperapi" => scraper_api::handle_scraper_api(matches, &services.http_client).await?,
        "proxy" => proxy::handle_proxy(matches).await?,
        "play" => play::handle_play(matches, services).await?,
        #[cfg(feature = "macos")]
        "macos" => mac::handle_mac(matches).await?,
        "luhn" => checksum::handle_checksum(matches)?,
        "detect" => detect::handle_detect(matches)?,
        "araxis" => araxis::handle_araxis(matches).await?,
        "aria2" => {
            aria2_cmd::handle_aria2(matches, services.aria2().await?.as_ref()).await?
        }
        "bitcoin" => bitcoin::handle_bitcoin(matches, &services.http_client).await?,
        "crx" => crx::handle_crx(matches, &services.http_client).await?,
        "download" => {
            download::handle_download(
                matches,
                services.download_manager().await?.as_ref(),
                services.fshare().await.ok().map(|f| f.as_ref()),
            )
            .await?
        }
        "fshare" => {
            fshare::handle_fshare(
                matches,
                services.fshare().await?.as_ref(),
                services.download_manager().await.ok().map(|dm| dm.as_ref()),
            )
            .await?
        }
        "fshare-scan" => fshare_scan::handle_fshare_scan(matches, services.fshare().await?.as_ref()).await?,
        "gcp" => {
            let config = services.config_manager.get_config().await;
            gcp::handle_gcp(matches, &config, &services.http_client).await?
        }
        "cloud" => cloud::handle_cloud(matches),
        "gdrive" => gdrive::handle_gdrive(matches, services.gdrive().await?.as_ref()).await?,
        "gcloud" => {
            let config = services.config_manager.get_config().await;
            gcloud::handle_gcloud(matches, &config).await?
        }
        "serve" => serve::handle_serve(matches).await?,
        "vmf" => vmf::handle_vmf(matches)?,
        "tailscale" => tailscale::handle_tailscale(matches, services.tailscale().await?.as_ref()).await?,
        "medium" => {
            let config = services.config_manager.get_config().await;
            medium::handle_medium(matches, &config, &services.http_client).await?
        }
        "aws" => aws::handle_aws(matches, &services.config_manager).await?,
        "md2pdf" => md2pdf::handle_md2pdf(matches).await?,
        "latex2pdf" => latex2pdf::handle_latex2pdf(matches).await?,
        "latex2typst" => latex2typst::handle_latex2typst(matches).await?,
        "cloudflare" => cloudflare::handle_cloudflare(matches, services.cloudflare().await?.as_ref()).await?,
        "winrar" => winrar::handle_winrar(matches).await?,
        "ocr" => {
            let config = services.config_manager.get_config().await;
            ocr::handle_ocr(matches, &config, &services.http_client).await?
        }
        "onflix" => onflix::handle_onflix(matches).await?,
        "gdrive-sync" => {
            gdrive_sync::handle_gdrive_sync(
                matches,
                services.gdrive().await?.as_ref(),
                &services.config_manager,
            )
            .await?
        }
        "daemon" => daemon::handle_daemon(matches, services.daemon_manager().await.as_ref())?,
        "cron" => {
            let mut cron_mgr = services.cron_manager().await.lock().await;
            cron::handle_cron(matches, &mut cron_mgr)?
        }
        "info" => info::handle_info(matches),
        _ => return Err(BosuaError::Command(format!("Unknown command: {}", name))),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::create_root_command;

    /// Expected macOS command names matching Go's `app.go` `RegisterCommands()`.
    /// Go macOS registers: version, config (from AddCommonCmd), cert, obf, registry,
    /// memprofile, ip, testmail, smsactivate, search, scraperapi, proxy, play, macos,
    /// luhn, detect, araxis, aria2, bitcoin, crx, download, fshare, fshare-scan, gcp,
    /// cloud, gdrive, gcloud, serve, vmf, tailscale, medium, aws, md2pdf, latex2pdf,
    /// latex2typst, cloudflare, winrar, ocr, onflix, gdrive-sync
    #[allow(dead_code)]
    const EXPECTED_MACOS_COMMANDS: &[&str] = &[
        "version",
        "config",
        "cert",
        "obf",
        "registry",
        "memprofile",
        "ip",
        "testmail",
        "smsactivate",
        "search",
        "scraperapi",
        "proxy",
        "play",
        "macos",
        "luhn",
        "detect",
        "araxis",
        "aria2",
        "bitcoin",
        "crx",
        "download",
        "fshare",
        "fshare-scan",
        "gcp",
        "cloud",
        "gdrive",
        "gcloud",
        "serve",
        "vmf",
        "tailscale",
        "medium",
        "aws",
        "md2pdf",
        "latex2pdf",
        "latex2typst",
        "cloudflare",
        "winrar",
        "ocr",
        "onflix",
        "gdrive-sync",
    ];

    /// Expected Linux command names matching Go's `app_server.go` `RegisterCommands()`.
    #[allow(dead_code)]
    const EXPECTED_LINUX_COMMANDS: &[&str] = &[
        "version",
        "config", // AddInfoCmd registers "config" with alias "info"
        "cron",
        "daemon",
        "download",
        "fshare",
        "fshare-scan",
        "gcp",
        "gdrive",
        "gdrive-sync",
        "serve", // Go: "serve" with aliases "http", "server"
        "onflix",
    ];

    /// Expected GCP command names matching Go's `app_server_gcp.go` `RegisterCommands()`.
    #[allow(dead_code)]
    const EXPECTED_GCP_COMMANDS: &[&str] = &[
        "version",
        "config", // AddInfoCmd registers "config" with alias "info"
        "cron",
        "daemon",
        "download",
        "fshare",
        "gdrive",
        "gdrive-sync",
        "gcp",
        "serve", // Go: "serve" with aliases "http", "server"
        "onflix",
    ];

    #[cfg(feature = "macos")]
    #[test]
    fn test_register_macos_commands() {
        let mut registry = CommandRegistry::new(create_root_command());
        register_macos_commands(&mut registry);

        let registered: Vec<String> = registry.command_names();
        for expected in EXPECTED_MACOS_COMMANDS {
            assert!(
                registered.contains(&expected.to_string()),
                "macOS variant missing command: '{}'. Registered: {:?}",
                expected,
                registered,
            );
        }
        assert_eq!(
            registered.len(),
            EXPECTED_MACOS_COMMANDS.len(),
            "macOS variant should have {} commands, got {}. Registered: {:?}",
            EXPECTED_MACOS_COMMANDS.len(),
            registered.len(),
            registered,
        );
    }

    #[cfg(feature = "linux")]
    #[test]
    fn test_register_linux_commands() {
        let mut registry = CommandRegistry::new(create_root_command());
        register_linux_commands(&mut registry);

        let registered: Vec<String> = registry.command_names();
        for expected in EXPECTED_LINUX_COMMANDS {
            assert!(
                registered.contains(&expected.to_string()),
                "Linux variant missing command: '{}'. Registered: {:?}",
                expected,
                registered,
            );
        }
        assert_eq!(
            registered.len(),
            EXPECTED_LINUX_COMMANDS.len(),
            "Linux variant should have {} commands, got {}. Registered: {:?}",
            EXPECTED_LINUX_COMMANDS.len(),
            registered.len(),
            registered,
        );
    }

    #[cfg(feature = "gcp")]
    #[test]
    fn test_register_gcp_commands() {
        let mut registry = CommandRegistry::new(create_root_command());
        register_gcp_commands(&mut registry);

        let registered: Vec<String> = registry.command_names();
        for expected in EXPECTED_GCP_COMMANDS {
            assert!(
                registered.contains(&expected.to_string()),
                "GCP variant missing command: '{}'. Registered: {:?}",
                expected,
                registered,
            );
        }
        assert_eq!(
            registered.len(),
            EXPECTED_GCP_COMMANDS.len(),
            "GCP variant should have {} commands, got {}. Registered: {:?}",
            EXPECTED_GCP_COMMANDS.len(),
            registered.len(),
            registered,
        );
    }
}
