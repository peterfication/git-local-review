#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, List, ListItem, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    services::{GitBranchesLoadingState, ReviewCreateData},
    views::{KeyBinding, ViewHandler, ViewType, centered_rectangle},
};

#[derive(Default)]
pub struct ReviewCreateView {
    pub branches_state: GitBranchesLoadingState,
    pub base_branch_index: usize,
    pub target_branch_index: usize,
    pub current_field: InputField,
    pub base_branch_filter: String,
    pub target_branch_filter: String,
}

#[derive(Default, PartialEq, Debug)]
pub enum InputField {
    BaseBranch,
    #[default]
    TargetBranch,
}

impl ViewHandler for ReviewCreateView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewCreate
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => self.close_view(app),
            KeyCode::Tab => self.review_selection_switch(),
            KeyCode::Up => self.review_selection_up(),
            KeyCode::Down => self.review_selection_down(),
            KeyCode::Char('k') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.review_selection_up()
            }
            KeyCode::Char('j') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.review_selection_down()
            }
            KeyCode::Enter => self.submit_review(app),
            KeyCode::Backspace => self.remove_filter_character(),
            KeyCode::Char('?') if key_event.modifiers.is_empty() => {
                app.events.send(AppEvent::HelpOpen(self.get_keybindings()))
            }
            KeyCode::Char(c)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                self.add_filter_character(c)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::ReviewCreated(_review) => self.close_view(app),
            AppEvent::ReviewCreatedError(_error) => self.close_view(app),
            AppEvent::GitBranchesLoadingState(state) => {
                self.handle_git_branches_loading_state(state)
            }
            _ => (),
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

        match &self.branches_state {
            GitBranchesLoadingState::Init => {
                let loading =
                    Paragraph::new("Initializing...").style(Style::default().fg(Color::Yellow));
                loading.render(inner, buf);
                return;
            }
            GitBranchesLoadingState::Loading => {
                let loading = Paragraph::new("Loading Git branches...")
                    .style(Style::default().fg(Color::Yellow));
                loading.render(inner, buf);
                return;
            }
            GitBranchesLoadingState::Error(error) => {
                let error_paragraph =
                    Paragraph::new(error.as_ref()).style(Style::default().fg(Color::Red));
                error_paragraph.render(inner, buf);
                return;
            }
            GitBranchesLoadingState::Loaded(branches) => {
                if branches.is_empty() {
                    let no_branches = Paragraph::new("No Git branches found in current directory")
                        .style(Style::default().fg(Color::Yellow));
                    no_branches.render(inner, buf);
                    return;
                }
                // Continue with rendering the branch lists
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        // Get branches from loaded state
        let branches = match &self.branches_state {
            GitBranchesLoadingState::Loaded(branches) => branches,
            _ => return, // Should not reach here due to early returns above
        };

        // Base branch list
        let base_branches = Self::filtered_branches(branches, &self.base_branch_filter);
        let target_branches = Self::filtered_branches(branches, &self.target_branch_filter);
        let base_branch_items: Vec<ListItem> = base_branches
            .iter()
            .enumerate()
            .map(|(i, (_, branch))| {
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

        let base_branch_title = if self.base_branch_filter.is_empty() {
            "Base Branch".to_string()
        } else {
            format!("Base Branch [{}]", self.base_branch_filter)
        };
        let base_branch_list = List::new(base_branch_items).block(
            Block::bordered()
                .title(base_branch_title)
                .border_style(base_branch_style),
        );
        base_branch_list.render(chunks[0], buf);

        // Target branch list
        let target_branch_items: Vec<ListItem> = target_branches
            .iter()
            .enumerate()
            .map(|(i, (_, branch))| {
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

        let target_branch_title = if self.target_branch_filter.is_empty() {
            "Target Branch".to_string()
        } else {
            format!("Target Branch [{}]", self.target_branch_filter)
        };
        let target_branch_list = List::new(target_branch_items).block(
            Block::bordered()
                .title(target_branch_title)
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
        let help = Paragraph::new(
            "Type: Filter, Backspace: Edit, ↑↓/Ctrl+j/k: Navigate, Tab: Switch, Enter: Create",
        )
        .style(Style::default().fg(Color::Gray));
        help.render(help_area, buf);
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        match &self.branches_state {
            GitBranchesLoadingState::Init => {
                format!(
                    "ReviewCreateView(state: Init, current_field: {:?})",
                    self.current_field
                )
            }
            GitBranchesLoadingState::Loading => {
                format!(
                    "ReviewCreateView(state: Loading, current_field: {:?})",
                    self.current_field
                )
            }
            GitBranchesLoadingState::Error(error) => {
                format!(
                    "ReviewCreateView(state: Error({}), current_field: {:?})",
                    error, self.current_field
                )
            }
            GitBranchesLoadingState::Loaded(branches) => {
                let base_branch = Self::filtered_branches(branches, &self.base_branch_filter)
                    .get(self.base_branch_index)
                    .map_or("none", |(_, branch)| branch.as_str());
                let target_branch = Self::filtered_branches(branches, &self.target_branch_filter)
                    .get(self.target_branch_index)
                    .map_or("none", |(_, branch)| branch.as_str());
                format!(
                    "ReviewCreateView(branches: {:?}, base_branch: \"{}\", target_branch: \"{}\", current_field: {:?}, base_filter: {:?}, target_filter: {:?})",
                    branches.as_ref(),
                    base_branch,
                    target_branch,
                    self.current_field,
                    self.base_branch_filter,
                    self.target_branch_filter
                )
            }
        }
    }

    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "↑↓ / Ctrl+j/k".to_string(),
                description: "Navigate branch list".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Type / Backspace".to_string(),
                description: "Filter active branch list".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Tab".to_string(),
                description: "Switch between input fields".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Tab,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Enter".to_string(),
                description: "Create review".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Esc".to_string(),
                description: "Cancel and close popup".to_string(),
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
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ReviewCreateView {
    fn close_view(&mut self, app: &mut App) {
        self.base_branch_index = 0;
        self.target_branch_index = 0;
        self.current_field = InputField::TargetBranch;
        self.base_branch_filter.clear();
        self.target_branch_filter.clear();
        app.events.send(AppEvent::ViewClose);
    }

    fn submit_review(&self, app: &mut App) {
        if let GitBranchesLoadingState::Loaded(ref branches) = self.branches_state {
            if branches.is_empty() {
                log::warn!("No branches available to create a review");
                return;
            }
            let base_branches = Self::filtered_branches(branches, &self.base_branch_filter);
            let target_branches = Self::filtered_branches(branches, &self.target_branch_filter);
            let base_branch = match base_branches.get(self.base_branch_index) {
                Some((_, branch)) => (*branch).clone(),
                None => {
                    // This should never happen, but handle gracefully
                    log::error!(
                        "Base branch index {} out of bounds for branches: {:?}",
                        self.base_branch_index,
                        branches
                    );
                    return;
                }
            };
            let target_branch = match target_branches.get(self.target_branch_index) {
                Some((_, branch)) => (*branch).clone(),
                None => {
                    // This should never happen, but handle gracefully
                    log::error!(
                        "Target branch index {} out of bounds for branches: {:?}",
                        self.target_branch_index,
                        branches
                    );
                    return;
                }
            };

            app.events
                .send(AppEvent::ReviewCreateSubmit(Arc::new(ReviewCreateData {
                    base_branch,
                    target_branch,
                    base_sha: None,
                    target_sha: None,
                })));
        }
    }

    fn review_selection_switch(&mut self) {
        self.current_field = match self.current_field {
            InputField::BaseBranch => InputField::TargetBranch,
            InputField::TargetBranch => InputField::BaseBranch,
        };
    }

    fn review_selection_up(&mut self) {
        if let GitBranchesLoadingState::Loaded(ref _branches) = self.branches_state {
            match self.current_field {
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
            }
        }
    }

    fn review_selection_down(&mut self) {
        if let GitBranchesLoadingState::Loaded(ref branches) = self.branches_state {
            match self.current_field {
                InputField::BaseBranch => {
                    if self.base_branch_index
                        < Self::filtered_branches(branches, &self.base_branch_filter)
                            .len()
                            .saturating_sub(1)
                    {
                        self.base_branch_index += 1;
                    }
                }
                InputField::TargetBranch => {
                    if self.target_branch_index
                        < Self::filtered_branches(branches, &self.target_branch_filter)
                            .len()
                            .saturating_sub(1)
                    {
                        self.target_branch_index += 1;
                    }
                }
            }
        }
    }

    fn add_filter_character(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        match self.current_field {
            InputField::BaseBranch => {
                self.base_branch_filter.push(character);
                self.base_branch_index = 0;
            }
            InputField::TargetBranch => {
                self.target_branch_filter.push(character);
                self.target_branch_index = 0;
            }
        }
    }

    fn remove_filter_character(&mut self) {
        match self.current_field {
            InputField::BaseBranch => {
                self.base_branch_filter.pop();
                self.base_branch_index = 0;
            }
            InputField::TargetBranch => {
                self.target_branch_filter.pop();
                self.target_branch_index = 0;
            }
        }
    }

    fn filtered_branches<'a>(branches: &'a [String], query: &str) -> Vec<(usize, &'a String)> {
        let mut matches: Vec<_> = branches
            .iter()
            .enumerate()
            .filter_map(|(index, branch)| {
                Self::fuzzy_score(branch, query).map(|score| (score, index, branch))
            })
            .collect();
        matches.sort_by_key(|(score, index, _)| (*score, *index));
        matches
            .into_iter()
            .map(|(_, index, branch)| (index, branch))
            .collect()
    }

    /// Scores a case-insensitive fuzzy match between a branch name and a query.
    ///
    /// Returns `None` when the query characters do not occur in order. Lower tuple values rank
    /// higher: the fields represent match type, match position or span, and candidate length.
    fn fuzzy_score(candidate: &str, query: &str) -> Option<(u8, usize, usize)> {
        if query.is_empty() {
            return Some((0, 0, candidate.len()));
        }
        let candidate = candidate.to_lowercase();
        let query = query.to_lowercase();
        if candidate == query {
            return Some((0, 0, candidate.len()));
        }
        if candidate.starts_with(&query) {
            return Some((1, 0, candidate.len()));
        }
        if let Some(position) = candidate.find(&query) {
            return Some((2, position, candidate.len()));
        }

        let mut query_chars = query.chars();
        let mut wanted = query_chars.next()?;
        let mut first = None;
        for (position, character) in candidate.chars().enumerate() {
            if character == wanted {
                first.get_or_insert(position);
                match query_chars.next() {
                    Some(next) => wanted = next,
                    None => {
                        return Some((3, position - first.unwrap_or(position), candidate.len()));
                    }
                }
            }
        }
        None
    }

    fn handle_git_branches_loading_state(&mut self, state: &GitBranchesLoadingState) {
        self.branches_state = state.clone();

        // Set default selection to main/master if available and we just loaded
        if let GitBranchesLoadingState::Loaded(ref branches) = self.branches_state
            && let Some(main_index) = branches.iter().position(|b| b == "main" || b == "master")
        {
            self.base_branch_index = main_index;
        }
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
        event::{AppEvent, Event, EventHandler},
        test_utils::render_app_to_terminal_backend,
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
    fn test_review_create_view_default() {
        let view = ReviewCreateView::default();
        assert!(matches!(view.branches_state, GitBranchesLoadingState::Init));
        assert_eq!(view.base_branch_index, 0);
        assert_eq!(view.target_branch_index, 0);
        assert_eq!(view.current_field, InputField::TargetBranch);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_up_down() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec![
                    "main".to_string(),
                    "develop".to_string(),
                    "feature/test".to_string(),
                ]
                .into(),
            ),
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
        };

        let key_event_up = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event_up).unwrap();
        assert_eq!(view.target_branch_index, 0);

        let key_event_down = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event_down).unwrap();
        assert_eq!(view.target_branch_index, 1);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_ctrl_j_k() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            target_branch_index: 0,
            ..Default::default()
        };

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert_eq!(view.target_branch_index, 1);

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL),
        )
        .unwrap();
        assert_eq!(view.target_branch_index, 0);
        assert!(view.target_branch_filter.is_empty());
    }

    #[tokio::test]
    async fn test_review_create_view_handle_help() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()),
        )
        .unwrap();

        assert!(view.target_branch_filter.is_empty());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::HelpOpen(_))));
    }

    #[tokio::test]
    async fn test_review_create_view_ignores_modified_filter_characters() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        )
        .unwrap();
        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT),
        )
        .unwrap();

        assert!(view.target_branch_filter.is_empty());
    }

    #[tokio::test]
    async fn test_review_create_view_accepts_shifted_filter_characters() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView::default();

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
        )
        .unwrap();

        assert_eq!(view.target_branch_filter, "F");
    }

    #[tokio::test]
    async fn test_review_create_view_handle_tab_navigation() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            base_branch_index: 0,
            target_branch_index: 0,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
        };

        let key_event = KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_field, InputField::BaseBranch);

        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_field, InputField::TargetBranch);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_up_at_bounds() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            base_branch_index: 0,
            target_branch_index: 0,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
        };

        let key_event = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 0 (top of list)
        assert_eq!(view.target_branch_index, 0);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_down_at_bounds() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
        };

        let key_event = KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 1 (bottom of list)
        assert_eq!(view.target_branch_index, 1);
    }

    #[tokio::test]
    async fn test_review_create_view_handle_esc() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            base_branch_index: 1,
            target_branch_index: 1,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
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
        assert_eq!(view.current_field, InputField::TargetBranch);
        // State is reset to default

        // Verify that a ViewClose event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ViewClose)));
    }

    #[tokio::test]
    async fn test_review_create_view_handle_enter() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec![
                    "main".to_string(),
                    "develop".to_string(),
                    "feature/test".to_string(),
                ]
                .into(),
            ),
            base_branch_index: 0,
            target_branch_index: 2,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
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
            branches_state: GitBranchesLoadingState::Loaded(
                vec!["main".to_string(), "develop".to_string()].into(),
            ),
            base_branch_index: 1,
            target_branch_index: 0,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
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

    #[test]
    fn test_fuzzy_filtering_and_ranking() {
        let branches: Arc<[String]> = vec![
            "bugfix/feature-toggle".to_string(),
            "ft".to_string(),
            "feature/test".to_string(),
            "release/FT".to_string(),
        ]
        .into();

        let matches = ReviewCreateView::filtered_branches(&branches, "ft");
        let names: Vec<_> = matches.iter().map(|(_, branch)| branch.as_str()).collect();
        assert_eq!(
            names,
            vec!["ft", "release/FT", "feature/test", "bugfix/feature-toggle"]
        );
    }

    #[tokio::test]
    async fn test_review_create_view_filters_active_list_and_submits_match() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(
                vec![
                    "main".to_string(),
                    "develop".to_string(),
                    "feature/test".to_string(),
                ]
                .into(),
            ),
            current_field: InputField::TargetBranch,
            ..Default::default()
        };

        for character in ['f', 't'] {
            view.handle_key_events(
                &mut app,
                &KeyEvent::new(KeyCode::Char(character), KeyModifiers::empty()),
            )
            .unwrap();
        }
        assert_eq!(view.target_branch_filter, "ft");
        assert!(view.base_branch_filter.is_empty());
        assert_eq!(view.target_branch_index, 0);

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        )
        .unwrap();
        let event = app.events.try_recv().unwrap();
        let Event::App(AppEvent::ReviewCreateSubmit(data)) = &*event else {
            panic!("Expected ReviewCreateSubmit event");
        };
        assert_eq!(data.base_branch, "main");
        assert_eq!(data.target_branch, "feature/test");
    }

    #[tokio::test]
    async fn test_review_create_view_backspace_and_empty_match() {
        let mut app = create_test_app().await;
        let mut view = ReviewCreateView {
            branches_state: GitBranchesLoadingState::Loaded(vec!["main".to_string()].into()),
            target_branch_filter: "missing".to_string(),
            target_branch_index: 4,
            ..Default::default()
        };

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        )
        .unwrap();
        assert_eq!(view.target_branch_filter, "missin");
        assert_eq!(view.target_branch_index, 0);

        view.handle_key_events(
            &mut app,
            &KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        )
        .unwrap();
        assert!(!app.events.has_pending_events());
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
            branches_state: GitBranchesLoadingState::Loaded(
                vec![
                    "main".to_string(),
                    "develop".to_string(),
                    "feature/new-feature".to_string(),
                ]
                .into(),
            ),
            base_branch_index: 0,
            target_branch_index: 2,
            current_field: InputField::TargetBranch,
            base_branch_filter: String::new(),
            target_branch_filter: String::new(),
        };
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
