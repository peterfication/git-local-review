use crate::database::Database;
use crate::event::{AppEvent, EventHandler};
use crate::event_handler::EventProcessor;
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

        Ok(Self {
            running: true,
            events: EventHandler::new(),
            database,
            view_stack: vec![Box::new(MainView::new())],
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        // Trigger initial reviews load
        self.events.send(AppEvent::ReviewsLoad);

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

    /// Notify all views in the view stack about an app event.
    /// Similar to handle_key_events, this ensures all views can respond to app events they care about.
    pub fn handle_app_events(&mut self, event: &AppEvent) {
        // We need to iterate through the view stack and handle each view separately
        // to avoid borrowing issues
        for i in 0..self.view_stack.len() {
            // Extract the view temporarily to avoid borrowing conflicts
            let mut view = self.view_stack.remove(i);
            view.handle_app_events(self, event);
            self.view_stack.insert(i, view);
        }
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
    use crate::event::{AppEvent, Event};
    use crate::models::review::Review;
    use crate::views::{ViewType, main::MainView, review_create::ReviewCreateView};
    use sqlx::SqlitePool;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![Box::new(MainView::new())],
        }
    }

    #[tokio::test]
    async fn test_app_new() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        let app = App {
            running: true,
            events: EventHandler::new(),
            database,
            view_stack: vec![Box::new(MainView::new())],
        };

        assert!(app.running);
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
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        app.handle_key_events(key_event).unwrap();

        // MainView should have received the key event and sent a Quit event
        assert!(app.running); // App doesn't quit until event is processed by EventProcessor
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_handle_key_events_with_review_create_view() {
        let mut app = create_test_app().await;

        // Add a review create view to the stack
        app.push_view(Box::new(ReviewCreateView::default()));
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        app.handle_key_events(key_event).unwrap();

        // The ReviewCreateView (top of stack) should have received the key event and sent a ReviewCreateClose event
        // The view stack should remain the same since we only sent the event to the view
        // The actual view closing would happen through the event system
        assert_eq!(app.view_stack.len(), 2);
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewCreateClose)));
    }

    #[tokio::test]
    async fn test_handle_key_events_only_top_view_responds() {
        let mut app = create_test_app().await;

        // Create a ReviewCreateView with some initial content to track changes
        let review_create_view = ReviewCreateView {
            title_input: "test".to_string(),
        };

        // Add it to the stack
        app.push_view(Box::new(review_create_view));
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
        assert!(!app.events.has_pending_events());

        // Verify initial state
        assert_eq!(
            app.view_stack.last().unwrap().debug_state(),
            "ReviewCreateView(title_input: \"test\")"
        );

        // Send a character key that would trigger different behaviors in different views
        // 'n' would trigger ReviewCreateOpen in MainView, but should be handled as text input by ReviewCreateView
        let key_event = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        app.handle_key_events(key_event).unwrap();

        // Only the ReviewCreateView (top of stack) should have received the key event
        // It should have processed 'n' as text input, changing the title_input
        assert_eq!(app.view_stack.len(), 2);
        assert!(!app.events.has_pending_events()); // No events sent for regular character input

        // Verify that the ReviewCreateView's title_input has been updated to include 'n'
        assert_eq!(
            app.view_stack.last().unwrap().debug_state(),
            "ReviewCreateView(title_input: \"testn\")"
        );
    }

    #[tokio::test]
    async fn test_tick() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let app = App {
            running: true,
            events: EventHandler::new(),
            database: Database::from_pool(pool),
            view_stack: vec![Box::new(MainView::new())],
        };

        // Tick should not change anything
        app.tick();
        // Since tick() is a no-op, there's nothing to assert
    }

    #[tokio::test]
    async fn test_handle_app_events() {
        let mut app = create_test_app().await;

        // Create a review to have data for testing
        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();
        let reviews = vec![review];

        // Verify MainView initially has no selection
        if let Some(main_view) = app.view_stack.get_mut(0) {
            if let Some(main_view) = main_view.as_any_mut().downcast_mut::<MainView>() {
                assert_eq!(main_view.selected_review_index(), None);
            }
        }

        // Call handle_app_events with ReviewsLoaded event
        app.handle_app_events(&AppEvent::ReviewsLoaded(reviews));

        // Verify MainView now has the first review selected
        if let Some(main_view) = app.view_stack.get_mut(0) {
            if let Some(main_view) = main_view.as_any_mut().downcast_mut::<MainView>() {
                assert_eq!(main_view.selected_review_index(), Some(0));
            }
        }
    }

    #[tokio::test]
    async fn test_handle_app_events_with_multiple_views() {
        let mut app = create_test_app().await;

        // Create a review to have data for testing
        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();
        let reviews = vec![review];

        // Add a ReviewCreateView to the stack
        app.push_view(Box::new(ReviewCreateView::default()));
        assert_eq!(app.view_stack.len(), 2);

        // Verify MainView initially has no selection
        if let Some(main_view) = app.view_stack.get_mut(0) {
            if let Some(main_view) = main_view.as_any_mut().downcast_mut::<MainView>() {
                assert_eq!(main_view.selected_review_index(), None);
            }
        }

        // Call handle_app_events with ReviewsLoaded event
        app.handle_app_events(&AppEvent::ReviewsLoaded(reviews));

        // Verify MainView now has the first review selected (all views should have received the event)
        if let Some(main_view) = app.view_stack.get_mut(0) {
            if let Some(main_view) = main_view.as_any_mut().downcast_mut::<MainView>() {
                assert_eq!(main_view.selected_review_index(), Some(0));
            }
        }

        // View stack should remain unchanged
        assert_eq!(app.view_stack.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_app_events_preserves_view_stack_order() {
        let mut app = create_test_app().await;

        // Add multiple views to the stack
        app.push_view(Box::new(ReviewCreateView::default()));
        let confirmation_dialog = crate::views::confirmation_dialog::ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ReviewCreateClose,
        );
        app.push_view(Box::new(confirmation_dialog));

        // Verify initial order: MainView -> ReviewCreateView -> ConfirmationDialogView
        assert_eq!(app.view_stack.len(), 3);
        assert_eq!(app.view_stack[0].view_type(), ViewType::Main);
        assert_eq!(app.view_stack[1].view_type(), ViewType::ReviewCreate);
        assert_eq!(app.view_stack[2].view_type(), ViewType::ConfirmationDialog);

        // Call handle_app_events
        app.handle_app_events(&AppEvent::ReviewsLoaded(vec![]));

        // Verify view stack order is preserved
        assert_eq!(app.view_stack.len(), 3);
        assert_eq!(app.view_stack[0].view_type(), ViewType::Main);
        assert_eq!(app.view_stack[1].view_type(), ViewType::ReviewCreate);
        assert_eq!(app.view_stack[2].view_type(), ViewType::ConfirmationDialog);
    }
}
