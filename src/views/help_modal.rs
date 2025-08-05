#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    views::{ViewHandler, ViewType, centered_rectangle},
};

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: String,
    pub description: String,
    pub key_event: KeyEvent,
}

pub struct HelpModalView {
    pub keybindings: Arc<[KeyBinding]>,
    pub list_state: ListState,
}

impl HelpModalView {
    pub fn new(keybindings: Arc<[KeyBinding]>) -> Self {
        let mut list_state = ListState::default();
        if !keybindings.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            keybindings,
            list_state,
        }
    }

    fn select_next(&mut self) {
        if self.keybindings.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let next = if selected >= self.keybindings.len() - 1 {
            0
        } else {
            selected + 1
        };
        self.list_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        if self.keybindings.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let previous = if selected == 0 {
            self.keybindings.len() - 1
        } else {
            selected - 1
        };
        self.list_state.select(Some(previous));
    }

    fn get_selected_key_event(&self) -> Option<KeyEvent> {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.keybindings.len() {
                return Some(self.keybindings[selected].key_event);
            }
        }
        None
    }
}

impl ViewHandler for HelpModalView {
    fn view_type(&self) -> ViewType {
        ViewType::HelpModal
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Enter => {
                // Send the selected key event - the event handler will close the modal
                if let Some(selected_key_event) = self.get_selected_key_event() {
                    app.events
                        .send(AppEvent::HelpKeySelected(Arc::new(selected_key_event)));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rectangle(70, 80, area);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title("Help - Key Bindings")
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);

        let items: Vec<ListItem> = self
            .keybindings
            .iter()
            .map(|binding| {
                ListItem::new(format!("{:<20} {}", binding.key, binding.description))
                    .style(Style::default().fg(Color::White))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("► ");

        let mut list_state = self.list_state.clone();
        ratatui::widgets::StatefulWidget::render(list, chunks[0], buf, &mut list_state);

        let help_text = ratatui::widgets::Paragraph::new(
            "Use ↑/↓ or j/k to navigate, Enter to execute, Esc to close",
        )
        .style(Style::default().fg(Color::Gray));
        help_text.render(chunks[1], buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!(
            "HelpModalView(selected: {:?}, keybindings: {})",
            self.list_state.selected(),
            self.keybindings.len()
        )
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        // Help modal doesn't have its own actionable keybindings,
        // it displays keybindings for other views
        Arc::new([])
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
        event::{Event, EventHandler},
        test_utils::render_app_to_terminal_backend,
        views::{ConfirmationDialogView, MainView, ReviewCreateView},
    };

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        }
    }

    #[test]
    fn test_help_modal_view_new() {
        let keybindings = vec![KeyBinding {
            key: "q".to_string(),
            description: "Quit".to_string(),
            key_event: KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            },
        }];
        let view = HelpModalView::new(keybindings.into());
        assert_eq!(view.keybindings.len(), 1);
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[test]
    fn test_help_modal_view_for_main_view() {
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let view = HelpModalView::new(Arc::clone(&keybindings));
        assert_eq!(view.keybindings.len(), 5);
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[test]
    fn test_help_modal_view_for_review_create_view() {
        let review_create_view = ReviewCreateView::default();
        let keybindings = review_create_view.get_keybindings();
        let view = HelpModalView::new(Arc::clone(&keybindings));
        assert_eq!(view.keybindings.len(), 4);
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[test]
    fn test_help_modal_view_for_confirmation_dialog() {
        let confirmation_view =
            ConfirmationDialogView::new("Test".to_string(), AppEvent::Quit, AppEvent::ViewClose);
        let keybindings = confirmation_view.get_keybindings();
        let view = HelpModalView::new(Arc::clone(&keybindings));
        assert_eq!(view.keybindings.len(), 2);
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[tokio::test]
    async fn test_help_modal_view_handle_esc_key() {
        let mut app = create_test_app().await;
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let mut view = HelpModalView::new(keybindings);
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
    async fn test_help_modal_view_navigation() {
        let mut app = create_test_app().await;
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let mut view = HelpModalView::new(keybindings);

        // Should start with first item selected
        assert_eq!(view.list_state.selected(), Some(0));

        // Test down navigation
        let key_event = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.list_state.selected(), Some(1));

        // Test j navigation
        let key_event = KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.list_state.selected(), Some(2));

        // Test up navigation
        let key_event = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.list_state.selected(), Some(1));

        // Test k navigation
        let key_event = KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.list_state.selected(), Some(0));
    }

    #[tokio::test]
    async fn test_help_modal_view_navigation_wraparound() {
        let _app = create_test_app().await;
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let mut view = HelpModalView::new(keybindings);

        // Navigate to last item
        while view.list_state.selected() != Some(view.keybindings.len() - 1) {
            view.select_next();
        }

        // Going down from last should wrap to first
        view.select_next();
        assert_eq!(view.list_state.selected(), Some(0));

        // Going up from first should wrap to last
        view.select_previous();
        assert_eq!(view.list_state.selected(), Some(view.keybindings.len() - 1));
    }

    #[tokio::test]
    async fn test_help_modal_view_enter_sends_key_event() {
        let mut app = create_test_app().await;
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let mut view = HelpModalView::new(keybindings);

        // Select the first keybinding (should be 'q')
        assert_eq!(view.list_state.selected(), Some(0));

        let key_event = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send HelpKeySelected
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::HelpKeySelected(_))));
    }

    #[tokio::test]
    async fn test_help_modal_view_render() {
        let main_view = MainView::new();
        let keybindings = main_view.get_keybindings();
        let view = HelpModalView::new(Arc::clone(&keybindings));
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
