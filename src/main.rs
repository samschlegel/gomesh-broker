use clap::Parser;
use miette::Context;

/// Convert an anyhow::Result into a miette::Result.
/// Needed because anyhow::Error doesn't implement std::error::Error.
fn from_anyhow<T>(result: anyhow::Result<T>) -> miette::Result<T> {
    result.map_err(|e| miette::miette!("{e:#}"))
}

/// MeshCore MQTT Broker — Ed25519 authenticated, topic-authorized MQTT broker.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Path to the broker configuration file (TOML).
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    env_logger::init();
    let args = Args::parse();

    let config = gomesh_broker::config::BrokerConfig::load(&args.config)?;
    log::info!("Loaded configuration from {}", args.config);

    // Parse the listen address from config
    let listen_addr: std::net::SocketAddr = config
        .listen
        .parse()
        .map_err(|e| miette::miette!("Invalid listen address '{}': {e}", config.listen))?;

    let plugin = gomesh_broker::hooks::MeshcorePlugin::new(config);

    let scx = rmqtt::context::ServerContext::new().build().await;

    // Register hook handlers with the broker
    let register = scx.extends.hook_mgr.register();
    plugin.register(register.as_ref()).await;
    register.start().await;
    log::info!("Registered MeshCore hook handlers");

    log::info!("Starting MQTT broker on {}", listen_addr);

    let listener = from_anyhow(
        rmqtt::net::Builder::new()
            .name("tcp")
            .laddr(listen_addr)
            .bind(),
    )
    .wrap_err("Failed to bind MQTT listener")?;

    let tcp = from_anyhow(listener.tcp()).wrap_err("Failed to create TCP listener")?;

    from_anyhow(
        rmqtt::server::MqttServer::new(scx)
            .listener(tcp)
            .build()
            .run()
            .await,
    )
    .wrap_err("MQTT broker exited with an error")
}
