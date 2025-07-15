use crate::app::App;
use crate::event::{AppEvent, Event, ReviewCreateData};
use crate::services::ReviewService;
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
                AppEvent::ReviewCreateSubmit(data) => Self::review_create_submit(app, data).await?,
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

    async fn review_create_submit(app: &mut App, data: ReviewCreateData) -> color_eyre::Result<()> {
        app.reviews = ReviewService::create_review(&app.database, data).await?;
        Self::review_create_close(app);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::views::{ViewType, main::MainView};
    use sqlx::SqlitePool;

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::models::review::Review::create_table(&pool)
            .await
            .unwrap();

        let database = Database::from_pool(pool);
        let reviews = vec![];

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            reviews,
            view_stack: vec![Box::new(MainView)],
        }
    }

    #[tokio::test]
    async fn test_process_quit_event() {
        let mut app = create_test_app().await;
        assert!(app.running);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::Quit))
            .await
            .unwrap();

        assert!(!app.running);
    }

    #[tokio::test]
    async fn test_process_review_create_open_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen))
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
    }

    #[tokio::test]
    async fn test_process_review_create_close_event() {
        let mut app = create_test_app().await;

        // First open a review create view
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen))
            .await
            .unwrap();
        assert_eq!(app.view_stack.len(), 2);

        // Then close it
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateClose))
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_review_create_submit_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.reviews.len(), 0);

        // Open review create view first
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen))
            .await
            .unwrap();
        assert_eq!(app.view_stack.len(), 2);

        let data = ReviewCreateData {
            title: "Test Review".to_string(),
        };

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateSubmit(data)))
            .await
            .unwrap();

        // Should have created a review
        assert_eq!(app.reviews.len(), 1);
        assert_eq!(app.reviews[0].title, "Test Review");

        // Should have closed the view
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_review_create_submit_empty_title() {
        let mut app = create_test_app().await;
        assert_eq!(app.reviews.len(), 0);

        let data = ReviewCreateData {
            title: "".to_string(),
        };

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateSubmit(data)))
            .await
            .unwrap();

        // Should not have created a review
        assert_eq!(app.reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_process_tick_event() {
        let mut app = create_test_app().await;

        // Tick event should not change anything
        EventProcessor::process_event(&mut app, Event::Tick)
            .await
            .unwrap();

        assert!(app.running);
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_crossterm_key_event() {
        let mut app = create_test_app().await;

        let key_event = ratatui::crossterm::event::KeyEvent {
            code: ratatui::crossterm::event::KeyCode::Char('q'),
            modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::empty(),
        };

        let crossterm_event = ratatui::crossterm::event::Event::Key(key_event);

        EventProcessor::process_event(&mut app, Event::Crossterm(crossterm_event))
            .await
            .unwrap();

        // The key event should be handled by the view, which only sends events
        // The app should remain running until the event is processed
        assert!(app.running);
    }
}
