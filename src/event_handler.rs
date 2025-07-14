use crate::app::App;
use crate::event::{AppEvent, Event};
use crate::views::review_create::ReviewCreateView;

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
                AppEvent::ReviewCreateOpen => Self::review_create_open(app),
                AppEvent::ReviewCreateClose => Self::review_create_close(app),
                AppEvent::ReviewCreateSubmit(data) => app.review_create_submit(data).await?,
            },
        }
        Ok(())
    }

    fn review_create_open(app: &mut App) {
        app.push_view(Box::new(ReviewCreateView::default()));
    }

    fn review_create_close(app: &mut App) {
        app.pop_view();
    }
}
