use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, List, ListItem, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    services::{GitService, ReviewCreateData},
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

#[derive(Default)]
pub struct ReviewCreateView {
    pub branches: Vec<String>,
    pub base_branch_index: usize,
    pub target_branch_index: usize,
    pub current_field: InputField,
    pub error_message: Option<String>,
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
            KeyCode::Up | KeyCode::Char('k') => match self.current_field {
                InputField::BaseBranch => {
                    if self.base_branch_index > 0 {
                        self.base_branch_index -= 1;
                    }
                }
                InputField::TargetBranch => {
                    if self.target_branch_index > 0 {
                        self.target_branch_index -= 1;
                    }
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.current_field {
                InputField::BaseBranch => {
                    if self.base_branch_index < self.branches.len().saturating_sub(1) {
                        self.base_branch_index += 1;
                    }
                }
                InputField::TargetBranch => {
                    if self.target_branch_index < self.branches.len().saturating_sub(1) {
                        self.target_branch_index += 1;
                    }
                }
            },
            KeyCode::Enter => {
                if self.branches.is_empty() {
                    return Ok(());
                }
                let base_branch = match self.branches.get(self.base_branch_index) {
                    Some(branch) => branch.clone(),
                    None => {
                        // This should never happen, but handle gracefully
                        self.error_message = Some("No base branch selected".to_string());
                        return Ok(());
                    }
                };
                let target_branch = match self.branches.get(self.target_branch_index) {
                    Some(branch) => branch.clone(),
                    None => {
                        // This should never happen, but handle gracefully
                        self.error_message = Some("No target branch selected".to_string());
                        return Ok(());
                    }
                };

                app.events
                    .send(AppEvent::ReviewCreateSubmit(Arc::new(ReviewCreateData {
                        base_branch,
                        target_branch,
                    })));
            }
            KeyCode::Char('?') => app.events.send(AppEvent::HelpOpen(self.get_keybindings())),
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
        let popup_area = centered_rectangle(80, 60, area);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Create New Review - Select Branches")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if let Some(error) = &self.error_message {
            let error_paragraph =
                Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));
            error_paragraph.render(inner, buf);
            return;
        }

        if self.branches.is_empty() {
            let no_branches = Paragraph::new("No Git branches found in current directory")
                .style(Style::default().fg(Color::Yellow));
            no_branches.render(inner, buf);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        // Base branch list
        let base_branch_items: Vec<ListItem> = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, branch)| {
                let style = if i == self.base_branch_index {
                    Style::default().bg(Color::Blue).fg(Color::Black)
                } else {
                    Style::default()
                };
                let text = if i == self.base_branch_index {
                    format!("> {branch}")
                } else {
                    format!("  {branch}")
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let base_branch_style = if self.current_field == InputField::BaseBranch {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let base_branch_list = List::new(base_branch_items).block(
            Block::bordered()
                .title("Base Branch")
                .border_style(base_branch_style),
        );
        base_branch_list.render(chunks[0], buf);

        // Target branch list
        let target_branch_items: Vec<ListItem> = self
            .branches
            .iter()
            .enumerate()
            .map(|(i, branch)| {
                let style = if i == self.target_branch_index {
                    Style::default().bg(Color::Blue).fg(Color::Black)
                } else {
                    Style::default()
                };
                let text = if i == self.target_branch_index {
                    format!("> {branch}")
                } else {
                    format!("  {branch}")
                };
                ListItem::new(text).style(style)
            })
            .collect();

        let target_branch_style = if self.current_field == InputField::TargetBranch {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let target_branch_list = List::new(target_branch_items).block(
            Block::bordered()
                .title("Target Branch")
                .border_style(target_branch_style),
        );
        target_branch_list.render(chunks[1], buf);

        // Help text at the bottom
        let help_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + popup_area.height - 2,
            width: popup_area.width - 2,
            height: 1,
        };
        let help = Paragraph::new("↑↓: Navigate, Tab: Switch lists, Enter: Create, Esc: Cancel")
            .style(Style::default().fg(Color::Gray));
        help.render(help_area, buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        let base_branch = self
            .branches
            .get(self.base_branch_index)
            .map(|s| s.as_str())
            .unwrap_or("none");
        let target_branch = self
            .branches
            .get(self.target_branch_index)
            .map(|s| s.as_str())
            .unwrap_or("none");
        format!(
            "ReviewCreateView(branches: {:?}, base_branch: \"{}\", target_branch: \"{}\", current_field: {:?})",
            self.branches, base_branch, target_branch, self.current_field
        )
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "↑↓ / jk".to_string(),
                description: "Navigate branch list".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Up,
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
                key: "Esc".to_string(),
                description: "Cancel and close popup".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
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
    pub fn new() -> Self {
        let mut view = Self::default();
        if let Ok(branches) = GitService::get_branches(".") {
            if !branches.is_empty() {
                view.branches = branches;
                // Set default selection to main/master if available
                if let Some(main_index) = view
                    .branches
                    .iter()
                    .position(|b| b == "main" || b == "master")
                {
                    view.base_branch_index = main_index;
                }
            }
        } else {
            view.error_message = Some("Failed to load Git branches".to_string());
        }
        view
    }

    fn close_view(&mut self, app: &mut App) {
        self.base_branch_index = 0;
        self.target_branch_index = 0;
        self.current_field = InputField::BaseBranch;
        self.error_message = None;
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
        assert_eq!(view.branches.len(), 0);
        assert_eq!(view.base_branch_index, 0);
        assert_eq!(view.target_branch_index, 0);
        assert_eq!(view.current_field, InputField::BaseBranch);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_up_down() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec![
                "main".to_string(),
                "develop".to_string(),
                "feature/test".to_string(),
            ],
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::BaseBranch,
            error_message: None,
        };

        let key_event_up = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event_up).unwrap();
        assert_eq!(view.base_branch_index, 0);

        let key_event_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event_down).unwrap();
        assert_eq!(view.base_branch_index, 1);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_tab_navigation() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec!["main".to_string(), "develop".to_string()],
            base_branch_index: 0,
            target_branch_index: 0,
            current_field: InputField::BaseBranch,
            error_message: None,
        };

        let key_event = KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_field, InputField::TargetBranch);

        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_field, InputField::BaseBranch);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_up_at_bounds() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec!["main".to_string(), "develop".to_string()],
            base_branch_index: 0,
            target_branch_index: 0,
            current_field: InputField::BaseBranch,
            error_message: None,
        };

        let key_event = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 0 (top of list)
        assert_eq!(view.base_branch_index, 0);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_down_at_bounds() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec!["main".to_string(), "develop".to_string()],
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::BaseBranch,
            error_message: None,
        };

        let key_event = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 1 (bottom of list)
        assert_eq!(view.base_branch_index, 1);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_esc() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec!["main".to_string(), "develop".to_string()],
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::TargetBranch,
            error_message: Some("Test error".to_string()),
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Indices should be reset
        assert_eq!(view.base_branch_index, 0);
        assert_eq!(view.target_branch_index, 0);
        assert_eq!(view.current_field, InputField::BaseBranch);
        assert_eq!(view.error_message, None);

        // Verify that a ViewClose event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_review_create_view_handle_enter() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec![
                "main".to_string(),
                "develop".to_string(),
                "feature/test".to_string(),
            ],
            base_branch_index: 0,
            target_branch_index: 2,
            current_field: InputField::BaseBranch,
            error_message: None,
        };
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Verify that a ReviewCreateSubmit event was sent with the correct branches
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

        // Should not create event when no branches
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_review_create_view_handle_unknown_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches: vec!["main".to_string(), "develop".to_string()],
            base_branch_index: 1,
            target_branch_index: 0,
            current_field: InputField::BaseBranch,
            error_message: None,
        };
        let initial_index = view.base_branch_index;

        let key_event = KeyEvent {
            code: KeyCode::F(1),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Unknown keys should not change indices
        assert_eq!(view.base_branch_index, initial_index);
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
    async fn test_review_create_view_render_with_branches() {
        let view = ReviewCreateView {
            branches: vec![
                "main".to_string(),
                "develop".to_string(),
                "feature/new-feature".to_string(),
            ],
            base_branch_index: 0,
            target_branch_index: 2,
            current_field: InputField::BaseBranch,
            error_message: None,
        };
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
