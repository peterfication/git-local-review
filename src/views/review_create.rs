use crate::{
    app::App,
    event::AppEvent,
    services::review_service::ReviewCreateData,
    views::{ViewHandler, ViewType},
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

#[derive(Default)]
pub struct ReviewCreateView {
    pub title_input: String,
}

impl ViewHandler for ReviewCreateView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewCreate
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.title_input.clear();
                app.events.send(AppEvent::ReviewCreateClose);
            }
            KeyCode::Enter => {
                app.events
                    .send(AppEvent::ReviewCreateSubmit(ReviewCreateData {
                        title: self.title_input.clone(),
                    }));
                self.title_input.clear();
            }
            KeyCode::Char(char) => {
                self.title_input.push(char);
            }
            KeyCode::Backspace => {
                self.title_input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rect(60, 40, area);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Create New Review")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);

        let title_label = Paragraph::new("Title:");
        title_label.render(chunks[0], buf);

        let title_input = Paragraph::new(self.title_input.as_str())
            .block(Block::bordered())
            .style(Style::default().fg(Color::Yellow));
        title_input.render(chunks[1], buf);

        let help = Paragraph::new("Press Enter to create, Esc to cancel")
            .style(Style::default().fg(Color::Gray));
        help.render(chunks[2], buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!("ReviewCreateView(title_input: \"{}\")", self.title_input)
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::event::{AppEvent, Event};
    use crate::models::review::Review;
    use crate::services::ReviewsLoadingState;
    use crate::test_utils::render_app_to_terminal_backend;
    use insta::assert_snapshot;
    use sqlx::SqlitePool;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        let database = Database::from_pool(pool);
        let reviews = vec![];

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            reviews,
            reviews_loading_state: ReviewsLoadingState::Loaded,
            view_stack: vec![],
        }
    }

    #[test]
    fn test_review_create_view_default() {
        let view = ReviewCreateView::default();
        assert_eq!(view.title_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_char_input() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        let key_event = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert_eq!(view.title_input, "a");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_multiple_chars() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        let chars = ['H', 'e', 'l', 'l', 'o'];
        for c in chars {
            let key_event = KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            };
            view.handle_key_events(&mut app, key_event).unwrap();
        }

        assert_eq!(view.title_input, "Hello");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_backspace() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            title_input: "Hello".to_string(),
        };

        let key_event = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert_eq!(view.title_input, "Hell");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_backspace_empty() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        let key_event = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert_eq!(view.title_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_esc() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            title_input: "Some input".to_string(),
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Title input should be cleared
        assert_eq!(view.title_input, "");

        // Verify that a ReviewCreateClose event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewCreateClose)));
    }

    #[tokio::test]
    async fn test_review_create_view_handle_enter() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            title_input: "Test Review".to_string(),
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Title input should be cleared after submit
        assert_eq!(view.title_input, "");

        // Verify that a ReviewCreateSubmit event was sent with the correct title
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewCreateSubmit(data)) = event {
            assert_eq!(data.title, "Test Review");
        } else {
            panic!("Expected ReviewCreateSubmit event");
        }
    }

    #[tokio::test]
    async fn test_review_create_view_handle_enter_empty() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Should still work with empty input
        assert_eq!(view.title_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_unknown_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();
        let initial_input = "Test".to_string();
        view.title_input = initial_input.clone();

        let key_event = KeyEvent {
            code: KeyCode::F(1),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Unknown keys should not change input
        assert_eq!(view.title_input, initial_input);
    }

    #[tokio::test]
    async fn test_review_create_view_render_default() {
        let view = ReviewCreateView::default();
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_create_view_render_with_title() {
        let view = ReviewCreateView {
            title_input: "My New Review".to_string(),
        };
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
