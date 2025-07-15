use crate::database::Database;
use crate::event::EventHandler;
use crate::event_handler::EventProcessor;
use crate::models::review::Review;
use crate::services::ReviewService;
use crate::views::{ViewHandler, main::MainView};
use ratatui::{DefaultTerminal, crossterm::event::KeyEvent};

/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// Database connection.
    pub database: Database,
    /// Reviews list.
    pub reviews: Vec<Review>,
    /// Current view stack.
    pub view_stack: Vec<Box<dyn ViewHandler>>,
}

impl Default for App {
    fn default() -> Self {
        panic!("Use App::new() instead of Default");
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub async fn new() -> color_eyre::Result<Self> {
        let database = Database::new().await?;
        let reviews = ReviewService::list_reviews(&database).await?;

        Ok(Self {
            running: true,
            events: EventHandler::new(),
            database,
            reviews,
            view_stack: vec![Box::new(MainView)],
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            let event = self.events.next().await?;
            EventProcessor::process_event(&mut self, event).await?;
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    /// Only the top view in the stack will handle the key events.
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // We need to avoid borrowing self twice, so we'll extract the view temporarily
        if !self.view_stack.is_empty() {
            let mut current_view = self.view_stack.pop().unwrap();
            let result = current_view.handle_key_events(self, key_event);
            self.view_stack.push(current_view);
            result?;
        }
        Ok(())
    }

    /// Push a view onto the view stack.
    pub fn push_view(&mut self, view: Box<dyn ViewHandler>) {
        self.view_stack.push(view);
    }

    /// Pop the current view from the view stack.
    pub fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{ViewType, main::MainView, review_create::ReviewCreateView};
    use sqlx::SqlitePool;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::models::review::Review::create_table(&pool)
            .await
            .unwrap();

        let database = Database::from_pool(pool);
        let reviews = vec![];

        App {
            running: true,
            events: EventHandler::new(),
            database,
            reviews,
            view_stack: vec![Box::new(MainView)],
        }
    }

    #[tokio::test]
    async fn test_app_new() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let database = Database::from_pool(pool);
        let reviews = ReviewService::list_reviews(&database).await.unwrap();

        let app = App {
            running: true,
            events: EventHandler::new(),
            database,
            reviews,
            view_stack: vec![Box::new(MainView)],
        };

        assert!(app.running);
        assert_eq!(app.reviews.len(), 0);
        assert_eq!(app.view_stack.len(), 1);
    }

    #[tokio::test]
    async fn test_quit() {
        let mut app = create_test_app().await;
        assert!(app.running);

        app.quit();

        assert!(!app.running);
    }

    #[tokio::test]
    async fn test_push_view() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);

        app.push_view(Box::new(ReviewCreateView::default()));

        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
    }

    #[tokio::test]
    async fn test_pop_view() {
        let mut app = create_test_app().await;

        // Initially should have MainView
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);

        // Add a second view (ReviewCreateView)
        app.push_view(Box::new(ReviewCreateView::default()));
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );

        // Pop it - should remove ReviewCreateView and leave MainView
        app.pop_view();
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_pop_view_keeps_at_least_one() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);

        // Try to pop the last view
        app.pop_view();

        // Should still have one view and it should still be MainView
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_handle_key_events() {
        let mut app = create_test_app().await;
        assert!(app.running);

        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        app.handle_key_events(key_event).unwrap();

        // MainView should have only sent an event, not processed it immediately
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_handle_key_events_with_review_create_view() {
        let mut app = create_test_app().await;

        // Add a review create view to the stack
        app.push_view(Box::new(ReviewCreateView::default()));
        assert_eq!(app.view_stack.len(), 2);

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        app.handle_key_events(key_event).unwrap();

        // The view stack should remain the same since we only sent the event to the view
        // The actual view closing would happen through the event system
        assert_eq!(app.view_stack.len(), 2);
    }

    #[tokio::test]
    async fn test_tick() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let app = App {
            running: true,
            events: EventHandler::new(),
            database: Database::from_pool(pool),
            reviews: vec![],
            view_stack: vec![Box::new(MainView)],
        };

        // Tick should not change anything
        app.tick();
        // Since tick() is a no-op, there's nothing to assert
    }
}
