use clap::Parser;

/// MeshCore MQTT Broker — Ed25519 authenticated, topic-authorized MQTT broker.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to the broker configuration file (TOML).
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let config = gomesh_broker::config::BrokerConfig::load(&args.config)?;
    log::info!("Loaded configuration from {}", args.config);

    let _plugin = gomesh_broker::hooks::MeshcorePlugin::new(config);

    // TODO: Initialize rmqtt broker instance and register plugin hooks
    // plugin.register();

    log::info!("gomesh-broker started");

    // TODO: Run the rmqtt broker event loop
    // For now, just hold the process open
    tokio::signal::ctrl_c().await?;
    log::info!("Shutting down");

    Ok(())
}
