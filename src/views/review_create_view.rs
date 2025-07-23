use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    services::ReviewCreateData,
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

#[derive(Default)]
pub struct ReviewCreateView {
    pub base_branch_input: String,
    pub target_branch_input: String,
    pub current_field: InputField,
}

#[derive(Default, PartialEq, Debug)]
pub enum InputField {
    #[default]
    BaseBranch,
    TargetBranch,
}

impl ViewHandler for ReviewCreateView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewCreate
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => self.close_view(app),
            KeyCode::Tab => {
                self.current_field = match self.current_field {
                    InputField::BaseBranch => InputField::TargetBranch,
                    InputField::TargetBranch => InputField::BaseBranch,
                };
            }
            KeyCode::Enter => {
                app.events
                    .send(AppEvent::ReviewCreateSubmit(Arc::new(ReviewCreateData {
                        base_branch: self.base_branch_input.clone(),
                        target_branch: self.target_branch_input.clone(),
                    })));
                self.base_branch_input.clear();
                self.target_branch_input.clear();
            }
            KeyCode::Char('?') => app.events.send(AppEvent::HelpOpen(self.get_keybindings())),
            KeyCode::Char(char) => match self.current_field {
                InputField::BaseBranch => self.base_branch_input.push(char),
                InputField::TargetBranch => self.target_branch_input.push(char),
            },
            KeyCode::Backspace => match self.current_field {
                InputField::BaseBranch => {
                    self.base_branch_input.pop();
                }
                InputField::TargetBranch => {
                    self.target_branch_input.pop();
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::ReviewCreated(_review) => {
                app.events.send(AppEvent::ViewClose);
            }
            AppEvent::ReviewCreatedError(_error) => {
                app.events.send(AppEvent::ViewClose);
            }
            _ => {}
        }
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rectangle(60, 40, area);

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
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);

        let base_branch_label = Paragraph::new("Base Branch:");
        base_branch_label.render(chunks[0], buf);

        let base_branch_style = if self.current_field == InputField::BaseBranch {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let base_branch_input = Paragraph::new(self.base_branch_input.as_str())
            .block(Block::bordered())
            .style(base_branch_style);
        base_branch_input.render(chunks[1], buf);

        let target_branch_label = Paragraph::new("Target Branch:");
        target_branch_label.render(chunks[2], buf);

        let target_branch_style = if self.current_field == InputField::TargetBranch {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let target_branch_input = Paragraph::new(self.target_branch_input.as_str())
            .block(Block::bordered())
            .style(target_branch_style);
        target_branch_input.render(chunks[3], buf);

        let help = Paragraph::new("Press Tab to switch fields, Enter to create, Esc to cancel")
            .style(Style::default().fg(Color::Gray));
        help.render(chunks[4], buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!(
            "ReviewCreateView(base_branch_input: \"{}\", target_branch_input: \"{}\", current_field: {:?})",
            self.base_branch_input, self.target_branch_input, self.current_field
        )
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "Esc".to_string(),
                description: "Cancel and close popup".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Enter".to_string(),
                description: "Create review".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Tab".to_string(),
                description: "Switch between input fields".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Tab,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "<char>".to_string(),
                description: "Enter branch names".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Backspace".to_string(),
                description: "Delete character".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
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

impl ReviewCreateView {
    fn close_view(&mut self, app: &mut App) {
        self.base_branch_input.clear();
        self.target_branch_input.clear();
        self.current_field = InputField::BaseBranch;
        app.events.send(AppEvent::ViewClose);
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
    fn test_review_create_view_default() {
        let view = ReviewCreateView::default();
        assert_eq!(view.base_branch_input, "");
        assert_eq!(view.target_branch_input, "");
        assert_eq!(view.current_field, InputField::BaseBranch);
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

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert_eq!(view.base_branch_input, "a");
        assert_eq!(view.target_branch_input, "");
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
            view.handle_key_events(&mut app, &key_event).unwrap();
        }

        assert_eq!(view.base_branch_input, "Hello");
        assert_eq!(view.target_branch_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_backspace() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            base_branch_input: "Hello".to_string(),
            target_branch_input: "".to_string(),
            current_field: InputField::BaseBranch,
        };

        let key_event = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert_eq!(view.base_branch_input, "Hell");
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

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert_eq!(view.base_branch_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_esc() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            base_branch_input: "Some input".to_string(),
            target_branch_input: "other input".to_string(),
            current_field: InputField::BaseBranch,
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Both inputs should be cleared
        assert_eq!(view.base_branch_input, "");
        assert_eq!(view.target_branch_input, "");

        // Verify that a ReviewCreateClose event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_review_create_view_handle_enter() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            base_branch_input: "main".to_string(),
            target_branch_input: "feature/test".to_string(),
            current_field: InputField::BaseBranch,
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Both inputs should be cleared after submit
        assert_eq!(view.base_branch_input, "");
        assert_eq!(view.target_branch_input, "");

        // Verify that a ReviewCreateSubmit event was sent with the correct title
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewCreateSubmit(ref data)) = *event {
            assert_eq!(data.base_branch, "main");
            assert_eq!(data.target_branch, "feature/test");
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

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should still work with empty input
        assert_eq!(view.base_branch_input, "");
        assert_eq!(view.target_branch_input, "");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_unknown_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();
        let initial_base = "Test".to_string();
        view.base_branch_input = initial_base.clone();

        let key_event = KeyEvent {
            code: KeyCode::F(1),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Unknown keys should not change input
        assert_eq!(view.base_branch_input, initial_base);
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
            base_branch_input: "main".to_string(),
            target_branch_input: "feature/new-feature".to_string(),
            current_field: InputField::BaseBranch,
        };
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
