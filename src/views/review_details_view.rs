use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    models::Review,
    views::{KeyBinding, ViewHandler, ViewType},
};

#[derive(Debug, Clone)]
enum ReviewDetailsState {
    Loading,
    Loaded(Arc<Review>),
    Error(String),
}

pub struct ReviewDetailsView {
    state: ReviewDetailsState,
}

impl ReviewDetailsView {
    pub fn new(review: Review) -> Self {
        Self {
            state: ReviewDetailsState::Loaded(Arc::from(review)),
        }
    }

    pub fn new_loading() -> Self {
        Self {
            state: ReviewDetailsState::Loading,
        }
    }
}

impl ViewHandler for ReviewDetailsView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewDetails
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        // Clear the background to make this a proper full-screen modal
        Clear.render(area, buf);

        let block = Block::default()
            .title(" Review Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .style(Style::default().bg(Color::Black));

        let inner_area = block.inner(area);
        block.render(area, buf);

        match &self.state {
            ReviewDetailsState::Loading => self.render_loading(inner_area, buf),
            ReviewDetailsState::Error(error) => self.render_error(error, inner_area, buf),
            ReviewDetailsState::Loaded(review) => self.render_loaded(review, inner_area, buf),
        }
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => app.events.send(AppEvent::ViewClose),
            KeyCode::Char('?') => app.events.send(AppEvent::HelpOpen(self.get_keybindings())),
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, _app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::ReviewLoaded(review) => {
                self.state = ReviewDetailsState::Loaded(Arc::clone(review));
            }
            AppEvent::ReviewNotFound(review_id) => {
                self.state = ReviewDetailsState::Error(format!("Review not found: {review_id}"));
            }
            AppEvent::ReviewLoadError(error) => {
                self.state = ReviewDetailsState::Error(error.to_string());
            }
            _ => {
                // Other events are not handled by this view
            }
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "Esc".to_string(),
                description: "Go back".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "?".to_string(),
                description: "Help".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('?'),
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
        ])
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        match &self.state {
            ReviewDetailsState::Loading => "state: Loading".to_string(),
            ReviewDetailsState::Error(error) => format!("state: Error(\"{error}\")"),
            ReviewDetailsState::Loaded(review) => {
                format!("state: Loaded(review_id: \"{}\")", review.id)
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

impl ReviewDetailsView {
    fn render_loading(&self, area: Rect, buf: &mut Buffer) {
        let loading_text = Paragraph::new("Loading review...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::NONE));
        loading_text.render(area, buf);
    }

    fn render_error(&self, error: &str, area: Rect, buf: &mut Buffer) {
        let error_text = Paragraph::new(format!("Error: {error}"))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::NONE));
        error_text.render(area, buf);
    }

    fn render_loaded(&self, review: &Review, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title section
                Constraint::Min(1),    // Content area
            ])
            .split(area);

        // Title section
        let title_block = Block::default()
            .title(" Title ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let title_content = Paragraph::new(review.title.as_str())
            .block(title_block)
            .style(Style::default().fg(Color::White));

        title_content.render(layout[0], buf);

        // Content section
        let content = Paragraph::new("<...>")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::NONE));

        let content_area = layout[1].inner(Margin {
            vertical: 1,
            horizontal: 1,
        });
        content.render(content_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::App,
        database::Database,
        models::{Review, review::TestReviewParams},
        test_utils::render_app_to_terminal_backend,
    };
    use insta::assert_snapshot;
    use sqlx::SqlitePool;

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![],
        }
    }

    #[test]
    fn test_review_details_view_creation() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.state {
            ReviewDetailsState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.title, "Test Review");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[test]
    fn test_review_details_view_new_loading() {
        let view = ReviewDetailsView::new_loading();

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.state {
            ReviewDetailsState::Loading => {}
            _ => panic!("Expected loading state"),
        }
    }

    #[test]
    fn test_review_details_view_debug_state() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        let debug_state = view.debug_state();
        assert!(debug_state.contains(&review.id));
        assert!(debug_state.starts_with("state: Loaded(review_id: \""));
    }

    #[test]
    fn test_review_details_view_debug_state_loading() {
        let view = ReviewDetailsView::new_loading();

        let debug_state = view.debug_state();
        assert_eq!(debug_state, "state: Loading");
    }

    #[test]
    fn test_review_details_view_keybindings() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        let keybindings = view.get_keybindings();
        assert_eq!(keybindings.len(), 2);
        assert_eq!(keybindings[0].key, "Esc");
        assert_eq!(keybindings[0].description, "Go back");
        assert_eq!(keybindings[1].key, "?");
        assert_eq!(keybindings[1].description, "Help");
    }

    #[tokio::test]
    async fn test_review_details_view_handles_escape_key() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send ViewClose event
        let event = app.events.try_recv().unwrap();
        match *event {
            crate::event::Event::App(AppEvent::ViewClose) => {}
            _ => panic!("Expected ViewClose event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_help_key() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(
            KeyCode::Char('?'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send HelpOpen event
        let event = app.events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::HelpOpen(_)) => {}
            _ => panic!("Expected HelpOpen event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_loaded_event() {
        let mut view = ReviewDetailsView::new_loading();
        let review = Review::test_review(TestReviewParams::new().title("Loaded Review"));
        let mut app = create_test_app().await;

        view.handle_app_events(&mut app, &AppEvent::ReviewLoaded(Arc::from(review)));

        match &view.state {
            ReviewDetailsState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.title, "Loaded Review");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_load_error_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadError("Database error".into()),
        );

        match &view.state {
            ReviewDetailsState::Error(error) => {
                assert_eq!(error, "Database error");
            }
            _ => panic!("Expected error state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_not_found_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(&mut app, &AppEvent::ReviewNotFound("test-id".into()));

        match &view.state {
            ReviewDetailsState::Error(error) => {
                assert_eq!(error, "Review not found: test-id");
            }
            _ => panic!("Expected error state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_render_loading_state() {
        let view = ReviewDetailsView::new_loading();
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_error_state() {
        let mut view = ReviewDetailsView::new_loading();
        view.state = ReviewDetailsState::Error("Database connection failed".to_string());
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state() {
        let review = Review::test_review(TestReviewParams::new().title("Sample Review Title"));
        let view = ReviewDetailsView::new(review);
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
