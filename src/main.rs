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

    let tls = match (config.tls_cert.clone(), config.tls_key.clone()) {
        (Some(cert), Some(key)) => Some((cert, key)),
        (None, None) => None,
        _ => miette::bail!("tls_cert and tls_key must be set together"),
    };
    let proxy_protocol = config.proxy_protocol;

    let plugin = gomesh_broker::hooks::MeshcorePlugin::new(config);

    let scx = rmqtt::context::ServerContext::new().build().await;

    // Register $SYS topic plugin for broker monitoring
    rmqtt_sys_topic::register(&scx, true, false)
        .await
        .map_err(|e| miette::miette!("Failed to register sys-topic plugin: {e}"))?;

    let register = scx.extends.hook_mgr.register();
    plugin.register(register.as_ref()).await;
    register.start().await;
    tracing::info!("Registered MeshCore hook handlers");

    let scheme = if tls.is_some() { "wss" } else { "ws" };
    tracing::info!(
        "Starting MQTT-over-{} broker on {} (proxy_protocol: {})",
        scheme.to_uppercase(),
        listen_addr,
        proxy_protocol
    );

    let mut builder = rmqtt::net::Builder::new()
        .name(scheme)
        .laddr(listen_addr)
        // Enable SO_REUSEADDR so the broker can rebind its port immediately on
        // restart while the previous process's sockets are still draining;
        // rmqtt-net leaves this off by default, causing "Address already in
        // use" flaps on redeploy/rollback.
        .reuseaddr(Some(true))
        // rmqtt-net allows anonymous clients by default, which skips the
        // ClientAuthenticate hook for CONNECTs without a username and
        // lets them connect with no identity.
        .allow_anonymous(false)
        .proxy_protocol(proxy_protocol);
    if let Some((cert, key)) = &tls {
        builder = builder.tls_cert(Some(cert)).tls_key(Some(key));
    }
    let listener = from_anyhow(builder.bind()).wrap_err("Failed to bind MQTT listener")?;

    let listener = if tls.is_some() {
        from_anyhow(listener.wss()).wrap_err("Failed to create WSS listener")?
    } else {
        from_anyhow(listener.ws()).wrap_err("Failed to create WS listener")?
    };

    from_anyhow(
        rmqtt::server::MqttServer::new(scx)
            .listener(listener)
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
