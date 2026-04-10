use std::sync::Mutex;

use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use tracing_subscriber::{
    filter::{self, LevelFilter},
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use crate::config::AccessLogConfig;

/// Initialize the tracing subscriber with an stdout layer and an optional
/// access log file layer.
///
/// - The stdout layer respects `RUST_LOG` (default `info`) and excludes the
///   `access` target so access events never leak to stdout.
/// - The file layer (when `access_log` config is present) captures only the
///   `access` target in JSON format, writing to a size-rotated file.
pub fn init(access_log: Option<&AccessLogConfig>) {
    let stdout_layer = fmt::layer().with_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
            .add_directive("access=off".parse().unwrap()),
    );

    let access_layer = access_log.map(|cfg| {
        let file_rotate = FileRotate::new(
            &cfg.path,
            AppendCount::new(cfg.max_files),
            ContentLimit::Bytes(cfg.max_size),
            Compression::None,
            None,
        );
        fmt::layer()
            .json()
            .with_ansi(false)
            .with_target(false)
            .with_level(false)
            .with_writer(Mutex::new(file_rotate))
            .with_filter(
                filter::Targets::new().with_target("access", LevelFilter::INFO),
            )
    });

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(access_layer)
        .init();
}
