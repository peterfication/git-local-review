use crate::app::App;
use crate::event::{AppEvent, Event};

pub struct EventProcessor;

impl EventProcessor {
    pub async fn process_event(app: &mut App, event: Event) -> color_eyre::Result<()> {
        match event {
            Event::Tick => app.tick(),
            #[allow(clippy::single_match)]
            Event::Crossterm(event) => match event {
                crossterm::event::Event::Key(key_event) => app.handle_key_events(key_event)?,
                _ => {}
            },
            Event::App(app_event) => match app_event {
                AppEvent::Quit => app.quit(),
                AppEvent::ReviewCreateOpen => app.review_create_open(),
                AppEvent::ReviewCreateClose => app.review_create_close(),
                AppEvent::ReviewCreateSubmit => app.review_create_submit().await?,
            },
        }
        Ok(())
    }
}
