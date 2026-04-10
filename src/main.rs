use clap::{Parser, Subcommand};
use miette::Context;
#[cfg(feature = "generate-cert")]
use miette::IntoDiagnostic;

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
    #[arg(short, long, default_value = "config.toml", global = true)]
    config: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the MQTT broker.
    Run,
    /// Generate a self-signed TLS certificate (cert.pem + key.pem).
    #[cfg(feature = "generate-cert")]
    GenerateCert,
}

#[cfg(feature = "generate-cert")]
fn generate_self_signed_cert() -> miette::Result<()> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .into_diagnostic()
        .wrap_err("Failed to generate self-signed certificate")?;

    std::fs::write("cert.pem", cert.cert.pem())
        .into_diagnostic()
        .wrap_err("Failed to write cert.pem")?;
    std::fs::write("key.pem", cert.key_pair.serialize_pem())
        .into_diagnostic()
        .wrap_err("Failed to write key.pem")?;

    println!("Generated self-signed TLS certificate:");
    println!("  cert.pem");
    println!("  key.pem");
    Ok(())
}

async fn run(config_path: &str) -> miette::Result<()> {
    let config = gomesh_broker::config::BrokerConfig::load(config_path)?;
    gomesh_broker::logging::init(config.access_log.as_ref());
    tracing::info!("Loaded configuration from {}", config_path);

    let listen_addr: std::net::SocketAddr = config
        .listen
        .parse()
        .map_err(|e| miette::miette!("Invalid listen address '{}': {e}", config.listen))?;

    let tls_cert = config.tls_cert.clone();
    let tls_key = config.tls_key.clone();

    let plugin = gomesh_broker::hooks::MeshcorePlugin::new(config);

    let scx = rmqtt::context::ServerContext::new().build().await;

    let register = scx.extends.hook_mgr.register();
    plugin.register(register.as_ref()).await;
    register.start().await;
    tracing::info!("Registered MeshCore hook handlers");

    tracing::info!("Starting MQTT-over-WSS broker on {}", listen_addr);

    let listener = from_anyhow(
        rmqtt::net::Builder::new()
            .name("wss")
            .laddr(listen_addr)
            .tls_cert(Some(&tls_cert))
            .tls_key(Some(&tls_key))
            .bind(),
    )
    .wrap_err("Failed to bind MQTT listener")?;

    let wss = from_anyhow(listener.wss()).wrap_err("Failed to create WSS listener")?;

    from_anyhow(
        rmqtt::server::MqttServer::new(scx)
            .listener(wss)
            .build()
            .run()
            .await,
    )
    .wrap_err("MQTT broker exited with an error")
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    let args = Args::parse();

    match args.command {
        None | Some(Command::Run) => run(&args.config).await,
        #[cfg(feature = "generate-cert")]
        Some(Command::GenerateCert) => {
            gomesh_broker::logging::init(None);
            generate_self_signed_cert()
        }
    }
}
