use std::sync::Arc;

use bosua_lib::cli::{create_root_command, is_verbose, CommandRegistry};
use bosua_lib::commands::register_macos_commands;
use bosua_lib::commands::registry_cmd::ServiceRegistry;
use bosua_lib::config::manager::DynamicConfigManager;
use bosua_lib::config::simplified::SimplifiedConfig;
use bosua_lib::errors::handle_fatal;
use bosua_lib::http_client::HttpClient;
use bosua_lib::signal::SignalHandler;

#[tokio::main]
async fn main() {
    // Step 0: Set timezone
    unsafe { std::env::set_var("TZ", "Asia/Bangkok") };

    // Step 1: Initialize SimplifiedConfig (singleton, from env vars)
    let _config = SimplifiedConfig::get();

    // Step 2: Set verbose mode callback (wired after arg parsing below)

    // Step 3: Initialize Logger
    bosua_lib::logger::init(false);

    // Step 4: Spawn SignalHandler
    let signal_handler = SignalHandler::new();
    let _shutdown_token = signal_handler.token();
    tokio::spawn(async move {
        signal_handler.listen().await;
    });

    // Step 5: Initialize JSON adapter
    bosua_lib::json::init();

    // Step 6: Create CommandRegistry with root command
    let mut registry = CommandRegistry::new(create_root_command());

    // Step 7: Register macOS variant commands
    register_macos_commands(&mut registry);

    // Step 8: Build and execute root command
    let root = registry.build_root();
    let matches = root.get_matches();

    // Wire verbose mode from parsed args
    let verbose = is_verbose(&matches);
    bosua_lib::logger::set_verbose(verbose);

    // Initialize DynamicConfigManager and ServiceRegistry
    let config_manager = Arc::new(
        DynamicConfigManager::initialize(None)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to initialize DynamicConfig: {}. Exiting.", e);
                std::process::exit(1);
            }),
    );
    let config = config_manager.get_config().await;
    let http_client = HttpClient::new(&config).expect("Failed to create HTTP client");
    let services = ServiceRegistry::new(config_manager, http_client);
    services.register_config_listeners().await;

    // Dispatch to subcommand handler
    match matches.subcommand() {
        Some((name, sub_matches)) => {
            tracing::debug!(command = name, "Executing command");
            if let Err(e) =
                bosua_lib::commands::dispatch_command(name, sub_matches, &services).await
            {
                bosua_lib::errors::handle_command_error(&e);
                std::process::exit(1);
            }
        }
        None => {
            // No subcommand — print help
            let mut cmd = create_root_command();
            register_macos_commands(&mut CommandRegistry::new(cmd.clone()));
            if let Err(e) = cmd.print_help().map_err(|e| {
                bosua_lib::errors::BosuaError::Application(format!("Failed to print help: {e}"))
            }) {
                handle_fatal(e);
            }
        }
    }
}
