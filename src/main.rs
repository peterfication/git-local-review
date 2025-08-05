use clap::Parser;

use crate::app::App;

#[derive(Parser)]
#[command(name = "git-local-review")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    about = "A Terminal User Interface (TUI) for reviewing Git changes with local SQLite state storage."
)]
struct Cli {
    /// Path to the Git repository to review
    #[arg(long, default_value = ".")]
    repo_path: String,
}

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
    // The App needs to be created before parsing CLI arguments so that the
    // database is being created when running with the argument "--version" for CI.
    let mut app = App::new().await?;

    // Parse command line arguments
    let cli = Cli::parse();
    app.set_repo_path(cli.repo_path);

    crate::logging::setup_logging();
    log::info!("Starting application");

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
