use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

pub struct ConfirmationDialogView {
    pub message: String,
    pub on_confirm_event: AppEvent,
    pub on_cancel_event: AppEvent,
}

impl ConfirmationDialogView {
    pub fn new(message: String, on_confirm_event: AppEvent, on_cancel_event: AppEvent) -> Self {
        Self {
            message,
            on_confirm_event,
            on_cancel_event,
        }
    }
}

impl ViewHandler for ConfirmationDialogView {
    fn view_type(&self) -> ViewType {
        ViewType::ConfirmationDialog
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.events.send(self.on_confirm_event.clone());
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') | KeyCode::Esc => {
                app.events.send(self.on_cancel_event.clone());
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                app.events.send(self.on_cancel_event.clone());
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char('?') => app.events.send(AppEvent::HelpOpen(self.get_keybindings())),
            _ => {}
        }
        Ok(())
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rectangle(50, 7, area);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Confirmation")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(inner);

        let message =
            Paragraph::new(self.message.as_str()).style(Style::default().fg(Color::White));
        message.render(chunks[0], buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!("ConfirmationDialogView(message: \"{}\")", self.message)
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "y / Y / Enter".to_string(),
                description: "Confirm".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('y'),
                    modifiers: KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "n / N / q / Esc / Ctrl+C".to_string(),
                description: "Cancel".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('n'),
                    modifiers: KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
        ])
    }

    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::event::{AppEvent, Event};
    use crate::test_utils::render_app_to_terminal_backend;
    use insta::assert_snapshot;
    use sqlx::SqlitePool;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
    fn test_confirmation_dialog_view_new() {
        let view = ConfirmationDialogView::new(
            "Do you want to delete this review?".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert_eq!(view.message, "Do you want to delete this review?");
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_y_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_capital_y_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('Y'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_enter_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_n_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_capital_n_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('N'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_esc_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_ctrl_c_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_ctrl_shift_c_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('C'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_handle_unknown_key() {
        let mut app = create_test_app().await;
        let mut view = ConfirmationDialogView::new(
            "Test message".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Unknown keys should not send any events
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_confirmation_dialog_view_render() {
        let view = ConfirmationDialogView::new(
            "Do you want to delete this review?".to_string(),
            AppEvent::Quit,
            AppEvent::ViewClose,
        );
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
