use crate::app::App;

pub mod app;
pub mod database;
pub mod event;
pub mod logging;
pub mod models;
pub mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::logging::setup_logging();
    log::info!("Starting application");

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().await?.run(terminal).await;
    ratatui::restore();
    result
}
