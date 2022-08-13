#![windows_subsystem = "windows"]

use tracing::Level;
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

fn main() {
    LogTracer::init().expect("Failed to init logging");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    ui::qrun();
}

mod k8s;
mod ui;
