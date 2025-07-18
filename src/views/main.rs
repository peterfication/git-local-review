use crate::{
    app::App,
    event::AppEvent,
    models::review::Review,
    services::review_service::ReviewsLoadingState,
    views::{ViewHandler, ViewType},
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

pub struct MainView {
    selected_review_index: Option<usize>,
    reviews: Vec<Review>,
    reviews_loading_state: ReviewsLoadingState,
}

impl Default for MainView {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewHandler for MainView {
    fn view_type(&self) -> ViewType {
        ViewType::Main
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => app.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                app.events.send(AppEvent::Quit)
            }
            KeyCode::Char('n') => self.create_review(app),
            KeyCode::Char('j') | KeyCode::Down => self.select_next_review(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous_review(),
            KeyCode::Char('d') => self.delete_selected_review(app),
            _ => {}
        }
        Ok(())
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let header =
            Paragraph::new("Git Local Review - Press 'n' to create a new review, 'q' to quit")
                .block(Block::bordered().title("git-local-review"))
                .fg(Color::Cyan);
        header.render(chunks[0], buf);

        let reviews: Vec<ListItem> = match &self.reviews_loading_state {
            ReviewsLoadingState::Init => self.render_reviews_init(),
            ReviewsLoadingState::Loading => self.render_reviews_loading(),
            ReviewsLoadingState::Loaded => self.render_reviews_loaded(),
            ReviewsLoadingState::Error(error) => self.render_reviews_error(error),
        };

        let reviews_list = List::new(reviews)
            .block(Block::bordered().title("Reviews"))
            .style(Style::default().fg(Color::White));

        reviews_list.render(chunks[1], buf);
    }

    fn handle_app_events(&mut self, _app: &App, event: &AppEvent) {
        match event {
            AppEvent::ReviewsLoad => {
                self.reviews_loading_state = ReviewsLoadingState::Loading;
            }
            AppEvent::ReviewsLoaded(reviews) => {
                self.reviews = reviews.clone();
                self.reviews_loading_state = ReviewsLoadingState::Loaded;
                self.update_selection_after_reviews_change();
            }
            AppEvent::ReviewsLoadingError(error) => {
                self.reviews_loading_state = ReviewsLoadingState::Error(error.clone());
            }
            _ => {
                // Ignore other events
            }
        }
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MainView {
    pub fn new() -> Self {
        Self {
            selected_review_index: None,
            reviews: Vec::new(),
            reviews_loading_state: ReviewsLoadingState::Init,
        }
    }

    /// Update selection after reviews list changes (e.g., after deletion)
    pub fn update_selection_after_reviews_change(&mut self) {
        if self.reviews.is_empty() {
            self.selected_review_index = None;
        } else if let Some(index) = self.selected_review_index {
            if index >= self.reviews.len() {
                // If selected index is out of bounds, select the last item
                self.selected_review_index = Some(self.reviews.len() - 1);
            }
        } else {
            // If no selection and we have reviews, select first
            self.selected_review_index = Some(0);
        }
    }

    /// Open the review creation view
    pub fn create_review(&mut self, app: &mut App) {
        app.events.send(AppEvent::ReviewCreateOpen);
    }

    /// Move selection up (decrease index)
    pub fn select_previous_review(&mut self) {
        if self.reviews.is_empty() {
            return;
        }

        match self.selected_review_index {
            None => self.selected_review_index = Some(0),
            Some(0) => {} // Already at top
            Some(index) => self.selected_review_index = Some(index - 1),
        }
    }

    /// Move selection down (increase index)
    pub fn select_next_review(&mut self) {
        if self.reviews.is_empty() {
            return;
        }

        match self.selected_review_index {
            None => self.selected_review_index = Some(0),
            Some(index) if index >= self.reviews.len() - 1 => {} // Already at bottom
            Some(index) => self.selected_review_index = Some(index + 1),
        }
    }

    /// Delete the currently selected review
    pub fn delete_selected_review(&self, app: &mut App) {
        if let Some(index) = self.selected_review_index {
            if index < self.reviews.len() {
                let review_id = self.reviews[index].id.clone();
                app.events.send(AppEvent::ReviewDeleteConfirm(review_id));
            }
        }
    }

    fn render_reviews_init(&self) -> Vec<ListItem> {
        vec![ListItem::new("Initializing...").style(Style::default().fg(Color::Gray))]
    }

    fn render_reviews_loading(&self) -> Vec<ListItem> {
        vec![ListItem::new("Loading reviews...").style(Style::default().fg(Color::Yellow))]
    }

    fn render_reviews_loaded(&self) -> Vec<ListItem> {
        if self.reviews.is_empty() {
            vec![
                ListItem::new("No reviews found - Press 'n' to create a new review")
                    .style(Style::default().fg(Color::Yellow)),
            ]
        } else {
            self.reviews
                .iter()
                .enumerate()
                .map(|(index, review)| {
                    let is_selected = Some(index) == self.selected_review_index;
                    self.render_review_list_item(review, is_selected)
                })
                .collect()
        }
    }

    fn render_review_list_item(&self, review: &Review, is_selected: bool) -> ListItem {
        let style = if is_selected {
            Style::default().bg(Color::Blue).fg(Color::Black)
        } else {
            Style::default()
        };
        let prefix = if is_selected { ">" } else { " " };
        let content = format!(
            "{} {} ({})",
            prefix,
            review.title,
            review.created_at.format("%Y-%m-%d %H:%M")
        );
        ListItem::new(content).style(style)
    }

    fn render_reviews_error(&self, error: &str) -> Vec<ListItem> {
        vec![
            ListItem::new(format!("Error loading reviews: {error}"))
                .style(Style::default().fg(Color::Red)),
        ]
    }

    /// Get the selected review index (for testing)
    #[cfg(test)]
    pub fn selected_review_index(&self) -> Option<usize> {
        self.selected_review_index
    }

    /// Get the reviews loading state (for testing)
    #[cfg(test)]
    pub fn reviews_loading_state(&self) -> &ReviewsLoadingState {
        &self.reviews_loading_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::event::{AppEvent, Event};
    use crate::models::review::Review;
    use crate::services::review_service::ReviewCreateData;
    use crate::test_utils::{fixed_time, render_app_to_terminal_backend};
    use crate::time_provider::MockTimeProvider;
    use insta::assert_snapshot;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use sqlx::SqlitePool;

    async fn create_test_app_with_reviews() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        // Create some test reviews with fixed timestamps
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::hours(1);

        let time_provider1 = MockTimeProvider::new(time1);
        let time_provider2 = MockTimeProvider::new(time2);

        let review1 = Review::new_with_time_provider("Review 1".to_string(), &time_provider1);
        let review2 = Review::new_with_time_provider("Review 2".to_string(), &time_provider2);
        review1.save(&pool).await.unwrap();
        review2.save(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![Box::new(MainView::new())],
        }
    }

    #[tokio::test]
    async fn test_main_view_handle_quit_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();
        assert!(app.running);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // The view handler only sends events, it doesn't process them immediately
        // The app remains running until the event is processed by EventProcessor
        assert!(app.running);

        // Verify that a Quit event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_esc_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();
        assert!(app.running);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);

        // Verify that a Quit event was sent (Esc also triggers quit)
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_ctrl_c() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();
        assert!(app.running);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);

        // Verify that a Quit event was sent (Ctrl+C also triggers quit)
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_create_review_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Verify that a ReviewCreateOpen event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewCreateOpen)));
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_unknown_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();
        let initial_running = app.running;
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Unknown keys should not change app state or send events
        assert_eq!(app.running, initial_running);
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_main_view_handle_navigation_j_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Populate the view with reviews first
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        view.reviews = reviews;

        let key_event = KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should select first review (index 0)
        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_handle_navigation_k_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Populate the view with reviews first
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        view.reviews = reviews;

        // Start with second review selected
        view.selected_review_index = Some(1);

        let key_event = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should move to first review
        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_handle_navigation_down_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Populate the view with reviews first
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        view.reviews = reviews;

        // Start with first review selected
        view.selected_review_index = Some(0);

        let key_event = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should move to second review
        assert_eq!(view.selected_review_index, Some(1));
    }

    #[tokio::test]
    async fn test_main_view_handle_navigation_up_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Populate the view with reviews first
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        view.reviews = reviews;

        // Start with second review selected
        view.selected_review_index = Some(1);

        let key_event = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should move to first review
        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_init() {
        let mut app = create_test_app_with_reviews().await;
        // Create a MainView with Init state
        let mut main_view = MainView::new();
        main_view.reviews_loading_state = ReviewsLoadingState::Init;
        app.view_stack = vec![Box::new(main_view)];
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loading() {
        let mut app = create_test_app_with_reviews().await;
        // Create a MainView with Loading state
        let mut main_view = MainView::new();
        main_view.reviews_loading_state = ReviewsLoadingState::Loading;
        app.view_stack = vec![Box::new(main_view)];
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_with_reviews() {
        let mut app = create_test_app_with_reviews().await;
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        app.handle_app_events(&AppEvent::ReviewsLoaded(reviews));
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_no_reviews() {
        let mut app = create_test_app_with_reviews().await;
        // Create a MainView with Loaded state but no reviews
        let mut main_view = MainView::new();
        main_view.reviews_loading_state = ReviewsLoadingState::Loaded;
        app.view_stack = vec![Box::new(main_view)];
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_error() {
        let mut app = create_test_app_with_reviews().await;
        // Create a MainView with Error state
        let mut main_view = MainView::new();
        main_view.reviews_loading_state = ReviewsLoadingState::Error("Test error".to_string());
        app.view_stack = vec![Box::new(main_view)];
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_with_selected_review() {
        let mut app = create_test_app_with_reviews().await;
        // Create a MainView with first review selected
        let mut main_view = MainView::new();
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        main_view.reviews = reviews;
        main_view.reviews_loading_state = ReviewsLoadingState::Loaded;
        main_view.selected_review_index = Some(0);
        app.view_stack = vec![Box::new(main_view)];

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_handle_delete_key_with_selection() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Populate the view with reviews first
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        view.reviews = reviews;

        // Select first review
        view.selected_review_index = Some(0);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should have sent a ReviewDeleteConfirm event with the review ID
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(
            event,
            Event::App(AppEvent::ReviewDeleteConfirm(_))
        ));
    }

    #[tokio::test]
    async fn test_main_view_handle_delete_key_no_selection() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // No selection
        // view.selected_review_index is None by default
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should not have sent any events since no selection
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_main_view_handle_delete_key_empty_reviews() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        // Empty reviews list
        view.reviews = vec![];
        view.selected_review_index = Some(0); // Invalid selection
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should not have sent any events since reviews list is empty
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_main_view_handle_app_events_reviews_loaded() {
        let app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        view.selected_review_index = None;

        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();
        let reviews = vec![review];

        view.handle_app_events(&app, &AppEvent::ReviewsLoaded(reviews));

        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_handle_app_events_review_delete() {
        let app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        view.selected_review_index = None;

        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();

        view.handle_app_events(&app, &AppEvent::ReviewDelete("some_id".to_string()));

        // Selection should not change until ReviewsLoaded is received
        assert_eq!(view.selected_review_index, None);
    }

    #[tokio::test]
    async fn test_main_view_handle_app_events_review_create_submit() {
        let app = create_test_app_with_reviews().await;
        let mut view = MainView::new();

        view.selected_review_index = None;

        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();

        view.handle_app_events(
            &app,
            &AppEvent::ReviewCreateSubmit(ReviewCreateData {
                title: "New Review".to_string(),
            }),
        );

        // Selection should not change until ReviewsLoaded is received
        assert_eq!(view.selected_review_index, None);
    }

    #[tokio::test]
    async fn test_main_view_update_selection_after_reviews_change_empty() {
        let mut view = MainView::new();
        view.selected_review_index = Some(0);

        view.reviews = vec![];
        view.update_selection_after_reviews_change();

        // Should clear selection for empty reviews
        assert_eq!(view.selected_review_index, None);
    }

    #[tokio::test]
    async fn test_main_view_update_selection_after_reviews_change_out_of_bounds() {
        let mut view = MainView::new();

        // Set selection to last item (index 1)
        view.selected_review_index = Some(1);

        // Create a review and a smaller reviews list (only 1 item)
        view.reviews = vec![Review::new("Test Review".to_string())];
        view.update_selection_after_reviews_change();

        // Should adjust selection to last valid index (0)
        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_update_selection_after_reviews_change_valid_selection() {
        let mut view = MainView::new();

        // Set selection to first item
        view.selected_review_index = Some(0);

        // Create a reviews list for testing
        view.reviews = vec![Review::new("Test Review".to_string())];
        view.update_selection_after_reviews_change();

        // Should preserve valid selection
        assert_eq!(view.selected_review_index, Some(0));
    }

    #[tokio::test]
    async fn test_main_view_update_selection_after_reviews_change_no_selection() {
        let mut view = MainView::new();

        // No selection initially
        assert_eq!(view.selected_review_index, None);

        // Create a reviews list for testing
        view.reviews = vec![Review::new("Test Review".to_string())];
        view.update_selection_after_reviews_change();

        // Should select first review
        assert_eq!(view.selected_review_index, Some(0));
    }
}
