use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn log_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("rkpack").join("logs")
}

pub fn init_logging() -> Result<WorkerGuard> {
    let log_dir = log_dir();
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("ログディレクトリの作成に失敗: {}", log_dir.display()))?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let file_name = format!("rkpack_{}.log", timestamp);

    let file_appender = tracing_appender::rolling::never(&log_dir, &file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_level(false)
        .without_time();

    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(file_layer)
        .with(stdout_layer)
        .init();

    tracing::info!("ログファイル: {}", log_dir.join(&file_name).display());

    Ok(guard)
}
