#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

pub struct ReviewRefreshDialogView {
    review_id: Arc<str>,
}

impl ReviewRefreshDialogView {
    pub fn new(review_id: Arc<str>) -> Self {
        Self { review_id }
    }
}

impl ViewHandler for ReviewRefreshDialogView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewRefreshDialog
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('b') => {
                app.events.send(AppEvent::ReviewRefreshBase {
                    review_id: Arc::clone(&self.review_id),
                });
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('t') => {
                app.events.send(AppEvent::ReviewRefreshTarget {
                    review_id: Arc::clone(&self.review_id),
                });
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('a') => {
                app.events.send(AppEvent::ReviewRefreshBoth {
                    review_id: Arc::clone(&self.review_id),
                });
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Esc => {
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('?') => app.events.send(AppEvent::HelpOpen(self.get_keybindings())),
            _ => {}
        }
        Ok(())
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rectangle(60, 35, area);
        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Refresh Review")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let lines = [
            Line::from("b: Refresh base SHA"),
            Line::from("t: Refresh target SHA"),
            Line::from("a: Refresh both SHAs"),
            Line::from("Esc: Cancel"),
        ];

        for (chunk, line) in chunks.iter().take(lines.len()).zip(lines.iter()) {
            Paragraph::new(line.clone())
                .style(Style::default().fg(Color::White))
                .render(*chunk, buf);
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "b".to_string(),
                description: "Refresh base SHA".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "t".to_string(),
                description: "Refresh target SHA".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('t'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "a".to_string(),
                description: "Refresh both SHAs".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Esc".to_string(),
                description: "Cancel".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
        ])
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!("ReviewRefreshDialogView(review_id: \"{}\")", self.review_id)
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_snapshot;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use sqlx::SqlitePool;

    use crate::{
        database::Database,
        event::{AppEvent, Event},
        test_utils::render_view_to_terminal_backend,
    };

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        }
    }

    #[test]
    fn test_review_refresh_dialog_view_new() {
        let view = ReviewRefreshDialogView::new(Arc::from("review-1"));
        assert_eq!(view.review_id.as_ref(), "review-1");
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_handle_base_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(Arc::from("review-1"));
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('b'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        let event = app.events.try_recv().unwrap();
        assert!(matches!(
            *event,
            Event::App(AppEvent::ReviewRefreshBase { .. })
        ));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_handle_escape_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(Arc::from("review-1"));
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_render() {
        let app = create_test_app().await;
        let view = ReviewRefreshDialogView::new(Arc::from("review-1"));
        let backend = render_view_to_terminal_backend(&app, |app, area, buf| {
            view.render(app, area, buf);
        });
        assert_snapshot!(backend);
    }
}
