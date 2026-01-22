#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

#[derive(Clone, Copy, Debug)]
pub struct ReviewRefreshOptions {
    pub can_refresh_base: bool,
    pub can_refresh_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshAction {
    Base,
    Target,
    Both,
}

impl RefreshAction {
    fn label(self) -> &'static str {
        match self {
            RefreshAction::Base => "Refresh base SHA",
            RefreshAction::Target => "Refresh target SHA",
            RefreshAction::Both => "Refresh both SHAs",
        }
    }

    fn key(self) -> char {
        match self {
            RefreshAction::Base => 'b',
            RefreshAction::Target => 't',
            RefreshAction::Both => 'a',
        }
    }
}

pub struct ReviewRefreshDialogView {
    review_id: Arc<str>,
    options: ReviewRefreshOptions,
    actions: Arc<[RefreshAction]>,
    list_state: ListState,
}

impl ReviewRefreshDialogView {
    pub fn new(review_id: Arc<str>, options: ReviewRefreshOptions) -> Self {
        let actions = Arc::new([
            RefreshAction::Base,
            RefreshAction::Target,
            RefreshAction::Both,
        ]);
        let mut list_state = ListState::default();
        if let Some(selected) = actions.iter().position(|action| match action {
            RefreshAction::Base => options.can_refresh_base,
            RefreshAction::Target => options.can_refresh_target,
            RefreshAction::Both => options.can_refresh_base && options.can_refresh_target,
        }) {
            list_state.select(Some(selected));
        }

        Self {
            review_id,
            options,
            actions,
            list_state,
        }
    }

    fn select_next(&mut self) {
        if self.actions.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let mut next = selected;
        for _ in 0..self.actions.len() {
            next = if next >= self.actions.len() - 1 {
                0
            } else {
                next + 1
            };
            if self.is_action_enabled(self.actions[next]) {
                self.list_state.select(Some(next));
                return;
            }
        }
    }

    fn select_previous(&mut self) {
        if self.actions.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let mut previous = selected;
        for _ in 0..self.actions.len() {
            previous = if previous == 0 {
                self.actions.len() - 1
            } else {
                previous - 1
            };
            if self.is_action_enabled(self.actions[previous]) {
                self.list_state.select(Some(previous));
                return;
            }
        }
    }

    fn selected_action(&self) -> Option<RefreshAction> {
        if let Some(selected) = self.list_state.selected()
            && selected < self.actions.len()
        {
            let action = self.actions[selected];
            if self.is_action_enabled(action) {
                return Some(action);
            }
        }
        None
    }

    fn is_action_enabled(&self, action: RefreshAction) -> bool {
        match action {
            RefreshAction::Base => self.options.can_refresh_base,
            RefreshAction::Target => self.options.can_refresh_target,
            RefreshAction::Both => self.options.can_refresh_base && self.options.can_refresh_target,
        }
    }

    fn trigger_action(&self, app: &mut App, action: RefreshAction) {
        let (refresh_base, refresh_target) = match action {
            RefreshAction::Base => (true, false),
            RefreshAction::Target => (false, true),
            RefreshAction::Both => (true, true),
        };
        app.events.send(AppEvent::ReviewRefresh {
            review_id: Arc::clone(&self.review_id),
            refresh_base,
            refresh_target,
        });
        app.events.send(AppEvent::ViewClose);
    }
}

impl ViewHandler for ReviewRefreshDialogView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewRefreshDialog
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('b') => {
                if self.is_action_enabled(RefreshAction::Base) {
                    self.trigger_action(app, RefreshAction::Base);
                }
            }
            KeyCode::Char('t') => {
                if self.is_action_enabled(RefreshAction::Target) {
                    self.trigger_action(app, RefreshAction::Target);
                }
            }
            KeyCode::Char('a') => {
                if self.is_action_enabled(RefreshAction::Both) {
                    self.trigger_action(app, RefreshAction::Both);
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
            }
            KeyCode::Enter => {
                if let Some(action) = self.selected_action() {
                    self.trigger_action(app, action);
                }
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
        let popup_area = centered_rectangle(70, 40, area);
        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Refresh Review")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        if self.options.can_refresh_base || self.options.can_refresh_target {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .split(inner);

            let items: Vec<ListItem> = self
                .actions
                .iter()
                .map(|action| {
                    let enabled = self.is_action_enabled(*action);
                    let label = if enabled {
                        action.label().to_string()
                    } else {
                        match action {
                            RefreshAction::Base | RefreshAction::Target => {
                                format!("{} (N/A because of up-to-date SHA)", action.label())
                            }
                            RefreshAction::Both => action.label().to_string(),
                        }
                    };
                    let key_label = if enabled {
                        action.key().to_string()
                    } else {
                        "N/A".to_string()
                    };
                    let style = if enabled {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    ListItem::new(format!("{:<6} {}", key_label, label)).style(style)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
                .highlight_symbol("► ");

            let mut list_state = self.list_state;
            ratatui::widgets::StatefulWidget::render(list, chunks[0], buf, &mut list_state);

            let help_text =
                Paragraph::new("Use ↑/↓ or j/k to navigate, Enter to select, Esc to cancel")
                    .style(Style::default().fg(Color::Gray));
            help_text.render(chunks[1], buf);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);
            Paragraph::new("No refreshable SHAs detected.\n")
                .style(Style::default().fg(Color::White))
                .render(chunks[0], buf);
            Paragraph::new("Press Esc to close.")
                .style(Style::default().fg(Color::Gray))
                .render(chunks[2], buf);
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "↑/k".to_string(),
                description: "Move selection up".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "↓/j".to_string(),
                description: "Move selection down".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Enter".to_string(),
                description: "Select action".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
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
        format!(
            "ReviewRefreshDialogView(review_id: \"{}\", selected: {:?}, can_refresh_base: {}, can_refresh_target: {})",
            self.review_id,
            self.list_state.selected(),
            self.options.can_refresh_base,
            self.options.can_refresh_target
        )
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
        let view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: true,
                can_refresh_target: true,
            },
        );
        assert_eq!(view.review_id.as_ref(), "review-1");
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_handle_base_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: true,
                can_refresh_target: false,
            },
        );
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
            Event::App(AppEvent::ReviewRefresh {
                refresh_base: true,
                refresh_target: false,
                ..
            })
        ));
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_handle_escape_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: true,
                can_refresh_target: false,
            },
        );
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
    async fn test_review_refresh_dialog_view_handle_enter_key() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: true,
                can_refresh_target: false,
            },
        );
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        let event = app.events.try_recv().unwrap();
        assert!(matches!(
            *event,
            Event::App(AppEvent::ReviewRefresh {
                refresh_base: true,
                refresh_target: false,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_handle_enter_key_no_actions() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: false,
                can_refresh_target: false,
            },
        );
        assert!(!app.events.has_pending_events());
        assert_eq!(view.list_state.selected(), None);

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_navigation_and_select() {
        let mut app = create_test_app().await;
        let mut view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: false,
                can_refresh_target: true,
            },
        );
        assert!(!app.events.has_pending_events());

        let down_event = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &down_event).unwrap();
        assert_eq!(view.list_state.selected(), Some(1));

        let enter_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &enter_event).unwrap();

        let event = app.events.try_recv().unwrap();
        assert!(matches!(
            *event,
            Event::App(AppEvent::ReviewRefresh {
                refresh_base: false,
                refresh_target: true,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_review_refresh_dialog_view_render() {
        let app = create_test_app().await;
        let view = ReviewRefreshDialogView::new(
            Arc::from("review-1"),
            ReviewRefreshOptions {
                can_refresh_base: true,
                can_refresh_target: true,
            },
        );
        let backend = render_view_to_terminal_backend(&app, |app, area, buf| {
            view.render(app, area, buf);
        });
        assert_snapshot!(backend);
    }
}
