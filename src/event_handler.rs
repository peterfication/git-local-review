use crate::app::App;
use crate::event::{AppEvent, Event};
use crate::services::{ReviewCreateData, ReviewService, ReviewsLoadingState};
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
                AppEvent::ReviewsLoad => Self::reviews_load(app).await?,
                AppEvent::ReviewsLoading => Self::reviews_loading(app).await?,
                AppEvent::ReviewsLoaded => Self::reviews_loaded(app),
                AppEvent::ReviewCreateOpen => Self::review_create_open(app),
                AppEvent::ReviewCreateClose => Self::review_create_close(app),
                AppEvent::ReviewCreateSubmit(data) => Self::review_create_submit(app, data).await?,
            },
        }
        Ok(())
    }

    /// Load set the loading state and send an event to start loading reviews
    async fn reviews_load(app: &mut App) -> color_eyre::Result<()> {
        // Uncomment to wait for a second for manual testing
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        app.reviews_loading_state = ReviewsLoadingState::Loading;
        app.events.send(AppEvent::ReviewsLoading);
        Ok(())
    }

    /// Load reviews from the database asynchronously
    async fn reviews_loading(app: &mut App) -> color_eyre::Result<()> {
        // Uncomment to wait for a second for manual testing
        // tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        match ReviewService::list_reviews(&app.database).await {
            Ok(reviews) => {
                app.reviews = reviews;
                app.events.send(AppEvent::ReviewsLoaded);
            }
            Err(e) => {
                app.reviews_loading_state = ReviewsLoadingState::Error(e.to_string());
            }
        }
        Ok(())
    }

    /// Mark reviews as loaded and stop loading state
    fn reviews_loaded(app: &mut App) {
        app.reviews_loading_state = ReviewsLoadingState::Loaded;
    }

    /// Open the review creation view
    fn review_create_open(app: &mut App) {
        app.push_view(Box::new(ReviewCreateView::default()));
    }

    /// Close the review creation view
    fn review_create_close(app: &mut App) {
        app.pop_view();
    }

    /// Submit the review creation form
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
    use crate::models::review::Review;
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
            reviews_loading_state: ReviewsLoadingState::Init,
            view_stack: vec![Box::new(MainView)],
        }
    }

    #[tokio::test]
    async fn test_process_reviews_load_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.reviews.len(), 0);
        assert_eq!(app.reviews_loading_state, ReviewsLoadingState::Init);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewsLoad))
            .await
            .unwrap();

        // Mark reviews as loading
        assert_eq!(app.reviews_loading_state, ReviewsLoadingState::Loading);
        // Check that the ReviewsLoading event has been triggered
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewsLoading)));
    }

    #[tokio::test]
    async fn test_process_reviews_loading_event() {
        let mut app = create_test_app().await;

        // Create and save a test review to the database
        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();

        assert_eq!(app.reviews.len(), 0);
        app.reviews_loading_state = ReviewsLoadingState::Loading; // Simulate that reviews are loading

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewsLoading))
            .await
            .unwrap();

        // Check that reviews have been loaded
        assert_eq!(app.reviews.len(), 1);
        // Check that a ReviewsLoaded event has been sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewsLoaded)));
        // Loading state should still be Loading until ReviewsLoaded is processed
        assert_eq!(app.reviews_loading_state, ReviewsLoadingState::Loading);
    }

    #[tokio::test]
    async fn test_process_reviews_loaded_event() {
        let mut app = create_test_app().await;
        app.reviews_loading_state = ReviewsLoadingState::Loading; // Simulate that reviews are loading

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewsLoaded))
            .await
            .unwrap();

        // Loading state should be Loaded after ReviewsLoaded is processed
        assert_eq!(app.reviews_loading_state, ReviewsLoadingState::Loaded);
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

        let data = crate::services::review_service::ReviewCreateData {
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

        let data = crate::services::review_service::ReviewCreateData {
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
