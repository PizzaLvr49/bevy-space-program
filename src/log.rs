use bevy::{
    log::{
        BoxedFmtLayer, BoxedLayer,
        tracing_subscriber::{self, Layer, filter::LevelFilter, fmt},
    },
    prelude::*,
};
use chrono::Local;
use flate2::{Compression, write::GzEncoder};
use std::{fs, fs::File, path::Path, sync::OnceLock};
use tracing_appender::{non_blocking, non_blocking::WorkerGuard};

static LATEST_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static DEBUG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn rotate_logs() -> std::io::Result<()> {
    fs::create_dir_all("logs")?;

    let latest = Path::new("logs/latest.log");
    if latest.exists() {
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let archive = format!("logs/{}.log.gz", timestamp);

        let mut src = File::open(latest)?;
        let mut dst = GzEncoder::new(File::create(&archive)?, Compression::default());

        std::io::copy(&mut src, &mut dst)?;
        dst.finish()?;
    }

    Ok(())
}

pub fn custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    rotate_logs().expect("Failed to rotate logs");

    let latest_file = File::create("logs/latest.log").expect("Failed to create latest.log");
    let (latest_writer, latest_guard) = non_blocking(latest_file);
    let _ = LATEST_GUARD.set(latest_guard);

    let latest_layer = tracing_subscriber::fmt::layer()
        .with_writer(latest_writer)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_filter(LevelFilter::INFO);

    let debug_file = File::create("logs/debug.log").expect("Failed to create debug.log");
    let (debug_writer, debug_guard) = non_blocking(debug_file);
    let _ = DEBUG_GUARD.set(debug_guard);

    let debug_layer = tracing_subscriber::fmt::layer()
        .with_writer(debug_writer)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_filter(LevelFilter::DEBUG);

    Some(latest_layer.and_then(debug_layer).boxed())
}

pub fn fmt_layer(_app: &mut App) -> Option<BoxedFmtLayer> {
    Some(
        fmt::layer()
            .with_ansi(true)
            .with_filter(LevelFilter::INFO)
            .boxed(),
    )
}
