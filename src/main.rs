use crate::app::App;
use clap::Parser;

#[derive(Parser)]
#[command(name = "git-local-review")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "A Terminal User Interface (TUI) for reviewing Git changes with local SQLite state storage."
)]
struct Cli {}

pub mod app;
pub mod database;
pub mod event;
pub mod event_handler;
pub mod logging;
pub mod models;
pub mod services;
#[cfg(test)]
pub mod test_utils;
pub mod time_provider;
pub mod ui;
pub mod views;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let app = App::new().await?;
    // Support for command line arguments like `--version`
    let _cli = Cli::parse();

    crate::logging::setup_logging();
    log::info!("Starting application");

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
