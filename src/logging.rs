use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::rolling;
use tracing_appender::non_blocking::WorkerGuard;

pub fn init_logging() -> WorkerGuard {
    let file_appender = rolling::daily("logs", "manifoldr.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .with_file(true)
        .pretty();

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .json();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}
