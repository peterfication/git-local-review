#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
};

use crate::{
    app::App,
    event::AppEvent,
    models::Comment,
    services::{CommentsLoadParams, CommentsLoadingState},
    views::{KeyBinding, ViewHandler, ViewType},
};

#[derive(Debug, Clone)]
pub enum CommentTarget {
    File {
        review_id: String,
        file_path: String,
    },
    Line {
        review_id: String,
        file_path: String,
        line_number: i64,
    },
}

impl CommentTarget {
    pub fn comments_load_params(&self) -> CommentsLoadParams {
        match self {
            CommentTarget::File {
                review_id,
                file_path,
            } => CommentsLoadParams {
                review_id: Arc::from(review_id.clone()),
                file_path: Arc::from(Some(file_path.clone())),
                line_number: Arc::from(None),
            },
            CommentTarget::Line {
                review_id,
                file_path,
                line_number,
            } => CommentsLoadParams {
                review_id: Arc::from(review_id.clone()),
                file_path: Arc::from(Some(file_path.clone())),
                line_number: Arc::from(Some(*line_number)),
            },
        }
    }

    pub fn review_id(&self) -> &str {
        match self {
            CommentTarget::File { review_id, .. } => review_id,
            CommentTarget::Line { review_id, .. } => review_id,
        }
    }

    pub fn file_path(&self) -> &str {
        match self {
            CommentTarget::File { file_path, .. } => file_path,
            CommentTarget::Line { file_path, .. } => file_path,
        }
    }

    pub fn line_number(&self) -> Option<i64> {
        match self {
            CommentTarget::File { .. } => None,
            CommentTarget::Line { line_number, .. } => Some(*line_number),
        }
    }

    pub fn is_file_target(&self) -> bool {
        matches!(self, CommentTarget::File { .. })
    }

    pub fn is_line_target(&self) -> bool {
        matches!(self, CommentTarget::Line { .. })
    }

    pub fn display_title(&self) -> String {
        match self {
            CommentTarget::File { file_path, .. } => {
                format!("Comments for {file_path}")
            }
            CommentTarget::Line {
                file_path,
                line_number,
                ..
            } => {
                format!("Comments for {file_path}:{line_number}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusState {
    Input,
    CommentsList,
}

pub struct CommentsView {
    /// The target for comments (file or line)
    target: CommentTarget,
    /// Current input text for new comment
    input_text: String,
    /// Current loading state of comments
    loading_state: CommentsLoadingState,
    /// Comments list (cached from loading state)
    comments: Arc<Vec<Comment>>,
    /// Current focus state (input field or comments list)
    focus_state: FocusState,
    /// Currently selected comment index (for navigation)
    selected_comment_index: Option<usize>,
}

impl CommentsView {
    pub fn new_for_file(review_id: String, file_path: String) -> Self {
        Self {
            target: CommentTarget::File {
                review_id,
                file_path,
            },
            input_text: String::new(),
            loading_state: CommentsLoadingState::Init,
            comments: Arc::new(vec![]),
            focus_state: FocusState::Input,
            selected_comment_index: None,
        }
    }

    pub fn new_for_line(review_id: String, file_path: String, line_number: i64) -> Self {
        Self {
            target: CommentTarget::Line {
                review_id,
                file_path,
                line_number,
            },
            input_text: String::new(),
            loading_state: CommentsLoadingState::Init,
            comments: Arc::new(vec![]),
            focus_state: FocusState::Input,
            selected_comment_index: None,
        }
    }

    /// Open help dialog with the keybindings of this view
    fn help(&self, app: &mut App) {
        app.events.send(AppEvent::HelpOpen(self.get_keybindings()));
    }

    fn handle_enter(&mut self, app: &mut App) {
        if self.focus_state != FocusState::Input {
            return;
        }

        if !self.input_text.trim().is_empty() {
            // Send event to create comment
            match &self.target {
                CommentTarget::File {
                    review_id,
                    file_path,
                } => {
                    app.events.send(AppEvent::CommentCreate {
                        review_id: review_id.clone().into(),
                        file_path: file_path.clone().into(),
                        line_number: None,
                        content: self.input_text.trim().to_string().into(),
                    });
                }
                CommentTarget::Line {
                    review_id,
                    file_path,
                    line_number,
                } => {
                    app.events.send(AppEvent::CommentCreate {
                        review_id: review_id.clone().into(),
                        file_path: file_path.clone().into(),
                        line_number: Some(*line_number),
                        content: self.input_text.trim().to_string().into(),
                    });
                }
            }

            // Clear the input
            self.input_text.clear();
        }
    }

    /// Switch focus between input field and comments list
    fn handle_tab(&mut self) {
        if self.focus_state == FocusState::Input {
            self.switch_focus_to_comments();
        } else {
            self.switch_focus_to_input();
        }
    }

    fn handle_backspace(&mut self) {
        if self.focus_state != FocusState::Input {
            return;
        }

        self.input_text.pop();
    }

    fn close(&self, app: &mut App) {
        app.events.send(AppEvent::ViewClose);
    }

    fn handle_char(&mut self, char: char, app: &mut App) {
        // Only handle character input when focused on input field
        if self.focus_state == FocusState::Input {
            // Limit input length to prevent very long comments
            if self.input_text.len() < 1000 {
                self.input_text.push(char);
            }
        } else {
            match char {
                'j' => self.move_selection_down(),
                'k' => self.move_selection_up(),
                'r' => self.handle_toggle_selected_comment(app),
                'R' => self.handle_toggle_all_comments(app),
                '?' => self.help(app),
                _ => {
                    // Ignore other characters when not focused on input
                }
            }
        }
    }

    fn switch_focus_to_comments(&mut self) {
        self.focus_state = FocusState::CommentsList;
        // Select the first comment if available
        if !self.comments.is_empty() {
            self.selected_comment_index = Some(0);
        } else {
            self.selected_comment_index = None;
        }
    }

    fn switch_focus_to_input(&mut self) {
        self.focus_state = FocusState::Input;
        self.selected_comment_index = None;
    }

    fn move_selection_up(&mut self) {
        if self.focus_state != FocusState::CommentsList {
            return;
        }

        if let Some(current_index) = self.selected_comment_index {
            if current_index > 0 {
                self.selected_comment_index = Some(current_index - 1);
            }
        }
    }

    fn move_selection_down(&mut self) {
        if self.focus_state != FocusState::CommentsList {
            return;
        }

        if let Some(current_index) = self.selected_comment_index {
            if current_index < self.comments.len().saturating_sub(1) {
                self.selected_comment_index = Some(current_index + 1);
            }
        } else if !self.comments.is_empty() {
            self.selected_comment_index = Some(0);
        }
    }

    fn get_selected_comment(&self) -> Option<&Comment> {
        self.selected_comment_index
            .and_then(|index| self.comments.get(index))
    }

    fn handle_toggle_selected_comment(&self, app: &mut App) {
        if let Some(comment) = self.get_selected_comment() {
            app.events.send(AppEvent::CommentToggleResolved {
                comment_id: comment.id.clone().into(),
            });
        }
    }

    fn handle_toggle_all_comments(&self, app: &mut App) {
        app.events.send(AppEvent::CommentsToggleAllResolved {
            review_id: self.target.review_id().into(),
            file_path: self.target.file_path().into(),
            line_number: self.target.line_number(),
        });
    }
}

impl ViewHandler for CommentsView {
    fn view_type(&self) -> ViewType {
        ViewType::Comments
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        // Clear the background to make this a proper modal
        Clear.render(area, buf);

        let block = Block::default()
            .title(format!(" {} ", self.target.display_title()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .style(Style::default().bg(Color::Black));

        let inner_area = block.inner(area);
        block.render(area, buf);

        // Split into input area (top) and comments list (bottom)
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input field
                Constraint::Min(1),    // Comments list
            ])
            .split(inner_area);

        self.render_input_field(layout[0], buf);
        self.render_comments_list(layout[1], buf);
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Tab => self.handle_tab(),
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Char(c) => self.handle_char(c, app),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Enter => self.handle_enter(app),
            KeyCode::Esc => self.close(app),
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::CommentsLoadingState { params, state } => {
                self.handle_comments_loading_state(params, state);
            }
            AppEvent::CommentCreated(_comment) => {
                // Reload comments when a new comment is created
                self.request_comments_reload(app);
            }
            AppEvent::CommentCreateError(_error) => {
                // Could show error message in UI, for now just reload
                self.request_comments_reload(app);
            }
            AppEvent::CommentMarkedResolved { .. } => {
                // Reload comments when a comment is marked as resolved
                self.request_comments_reload(app);
            }
            AppEvent::CommentsMarkedAllResolved { .. } => {
                // Reload comments when all comments are marked as resolved
                self.request_comments_reload(app);
            }
            AppEvent::CommentMarkResolvedError { .. } => {
                // Could show error message in UI, for now just reload
                self.request_comments_reload(app);
            }
            AppEvent::CommentsMarkAllResolvedError { .. } => {
                // Could show error message in UI, for now just reload
                self.request_comments_reload(app);
            }
            AppEvent::CommentToggledResolved { .. } => {
                // Reload comments when a comment's resolved state is toggled
                self.request_comments_reload(app);
            }
            AppEvent::CommentsToggledAllResolved { .. } => {
                // Reload comments when all comments' resolved state is toggled
                self.request_comments_reload(app);
            }
            AppEvent::CommentToggleResolvedError { .. } => {
                // Could show error message in UI, for now just reload
                self.request_comments_reload(app);
            }
            AppEvent::CommentsToggleAllResolvedError { .. } => {
                // Could show error message in UI, for now just reload
                self.request_comments_reload(app);
            }
            _ => {
                // Other events are not handled by this view
            }
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "Tab".to_string(),
                description: "Switch focus".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Tab,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Enter".to_string(),
                description: "Add comment".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "k/↑".to_string(),
                description: "Navigate up (when in comments list)".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "j/↓".to_string(),
                description: "Navigate down (when in comments list)".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "r".to_string(),
                description: "Toggle resolved (when in comments list)".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "R".to_string(),
                description: "Toggle all resolved (when in comments list)".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('R'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Esc".to_string(),
                description: "Close comments".to_string(),
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
            "target: {:?}, input_text: {:?}, loading_state: {:?}, comments_count: {}, focus_state: {:?}, selected_comment_index: {:?}",
            self.target,
            self.input_text,
            match &self.loading_state {
                CommentsLoadingState::Init => "Init".to_string(),
                CommentsLoadingState::Loading => "Loading".to_string(),
                CommentsLoadingState::Loaded(comments) => format!("Loaded({})", comments.len()),
                CommentsLoadingState::Error(error) => format!("Error({error})"),
            },
            self.comments.len(),
            self.focus_state,
            self.selected_comment_index,
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

impl CommentsView {
    fn render_input_field(&self, area: Rect, buf: &mut Buffer) {
        let is_focused = self.focus_state == FocusState::Input;
        let border_color = if is_focused {
            Color::Green
        } else {
            Color::Gray
        };
        let title = if is_focused {
            " New Comment (focused) "
        } else {
            " New Comment "
        };

        let input_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let input_content = Paragraph::new(self.input_text.as_str())
            .block(input_block)
            .style(Style::default().fg(Color::White));

        input_content.render(area, buf);
    }

    fn render_comments_list(&self, area: Rect, buf: &mut Buffer) {
        match &self.loading_state {
            CommentsLoadingState::Init => {
                let loading_text = Paragraph::new("Initializing comments...")
                    .style(Style::default().fg(Color::Yellow))
                    .block(
                        Block::default()
                            .title(" Comments ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Gray)),
                    );
                loading_text.render(area, buf);
            }
            CommentsLoadingState::Loading => {
                let loading_text = Paragraph::new("Loading comments...")
                    .style(Style::default().fg(Color::Yellow))
                    .block(
                        Block::default()
                            .title(" Comments ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Gray)),
                    );
                loading_text.render(area, buf);
            }
            CommentsLoadingState::Error(error) => {
                let error_text = Paragraph::new(format!("Error loading comments: {error}"))
                    .style(Style::default().fg(Color::Red))
                    .block(
                        Block::default()
                            .title(" Comments ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Red)),
                    );
                error_text.render(area, buf);
            }
            CommentsLoadingState::Loaded(_) => {
                self.render_comments_list_loaded(area, buf);
            }
        }
    }

    fn render_comments_list_loaded(&self, area: Rect, buf: &mut Buffer) {
        let is_focused = self.focus_state == FocusState::CommentsList;
        let border_color = if is_focused {
            Color::Green
        } else {
            Color::Gray
        };
        let title = if is_focused {
            format!(" Comments ({}) (focused) ", self.comments.len())
        } else {
            format!(" Comments ({}) ", self.comments.len())
        };

        if self.comments.is_empty() {
            let empty_text = Paragraph::new("No comments yet. Add one above!")
                .style(Style::default().fg(Color::Gray))
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color)),
                );
            empty_text.render(area, buf);
            return;
        }

        // Create list items for comments
        let comment_items: Vec<ListItem> = self
            .comments
            .iter()
            .enumerate()
            .map(|(index, comment)| self.render_comment_item(index, comment))
            .collect();

        let comments_list = List::new(comment_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        // Create list state and set selected index if focused
        let mut list_state = ListState::default();
        if is_focused {
            list_state.select(self.selected_comment_index);
        }

        StatefulWidget::render(comments_list, area, buf, &mut list_state);
    }

    fn render_comment_item(&self, _index: usize, comment: &Comment) -> ListItem {
        // Format the comment with timestamp and content
        let timestamp = comment.created_at.format("%Y-%m-%d %H:%M:%S");

        let comment_type = if comment.is_file_comment() {
            "FILE"
        } else {
            &format!("LINE {}", comment.line_number.unwrap_or(0))
        };

        // Show resolved status
        let resolved_indicator = if comment.resolved { "[✓]" } else { "[ ]" };
        let resolved_color = if comment.resolved {
            Color::Green
        } else {
            Color::Gray
        };

        let content = vec![
            Line::from(vec![
                Span::styled(
                    format!("{resolved_indicator} [{comment_type}] "),
                    Style::default()
                        .fg(resolved_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(timestamp.to_string(), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(Span::styled(
                comment.content.clone(),
                if comment.resolved {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::White)
                },
            )),
            Line::from(""), // Empty line for spacing
        ];

        ListItem::new(content)
    }

    fn handle_comments_loading_state(
        &mut self,
        params: &CommentsLoadParams,
        state: &CommentsLoadingState,
    ) {
        if !self.target.comments_load_params().equals(params) {
            // If the params don't match our target, ignore this state
            return;
        }

        self.loading_state = state.clone();

        if let CommentsLoadingState::Loaded(comments) = state {
            self.comments = comments.clone();
            // Reset selection if comments changed
            if self.focus_state == FocusState::CommentsList {
                if !self.comments.is_empty() {
                    // Keep selection in bounds
                    if let Some(current_index) = self.selected_comment_index {
                        if current_index >= self.comments.len() {
                            self.selected_comment_index =
                                Some(self.comments.len().saturating_sub(1));
                        }
                    } else {
                        self.selected_comment_index = Some(0);
                    }
                } else {
                    self.selected_comment_index = None;
                }
            }
        }
    }

    fn request_comments_reload(&self, app: &mut App) {
        match &self.target {
            CommentTarget::File {
                review_id,
                file_path,
            } => {
                app.events.send(AppEvent::CommentsLoad(CommentsLoadParams {
                    review_id: review_id.clone().into(),
                    file_path: Arc::from(Some(file_path.clone())),
                    line_number: Arc::from(None),
                }));
            }
            CommentTarget::Line {
                review_id,
                file_path,
                line_number,
            } => {
                app.events.send(AppEvent::CommentsLoad(CommentsLoadParams {
                    review_id: review_id.clone().into(),
                    file_path: Arc::from(Some(file_path.clone())),
                    line_number: Arc::from(Some(*line_number)),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::SqlitePool;

    use crate::{app::App, database::Database, models::Comment};

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
    fn test_comments_view_creation_file() {
        let view = CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());

        assert_eq!(view.view_type(), ViewType::Comments);
        assert!(view.target.is_file_target());
        assert_eq!(view.target.review_id(), "review-123");
        assert_eq!(view.target.file_path(), "src/main.rs");
        assert_eq!(view.target.line_number(), None);
        assert_eq!(view.input_text, "");
    }

    #[test]
    fn test_comments_view_creation_line() {
        let view =
            CommentsView::new_for_line("review-123".to_string(), "src/main.rs".to_string(), 42);

        assert_eq!(view.view_type(), ViewType::Comments);
        assert!(view.target.is_line_target());
        assert_eq!(view.target.review_id(), "review-123");
        assert_eq!(view.target.file_path(), "src/main.rs");
        assert_eq!(view.target.line_number(), Some(42));
        assert_eq!(view.input_text, "");
    }

    #[test]
    fn test_comment_target_display_title() {
        let file_target = CommentTarget::File {
            review_id: "review-123".to_string(),
            file_path: "src/main.rs".to_string(),
        };
        assert_eq!(file_target.display_title(), "Comments for src/main.rs");

        let line_target = CommentTarget::Line {
            review_id: "review-123".to_string(),
            file_path: "src/main.rs".to_string(),
            line_number: 42,
        };
        assert_eq!(line_target.display_title(), "Comments for src/main.rs:42");
    }

    #[tokio::test]
    async fn test_comments_view_input_handling() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        // Test character input
        let key_event = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.input_text, "H");

        // Test more characters
        let key_event = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.input_text, "Hi");

        // Test backspace
        let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.input_text, "H");
    }

    #[tokio::test]
    async fn test_comments_view_enter_creates_comment() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        // Add some text
        view.input_text = "This is a test comment".to_string();

        // Press Enter
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Input should be cleared
        assert_eq!(view.input_text, "");

        // Should have sent CommentCreate event
        let event = app.events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentCreate {
                review_id,
                file_path,
                line_number,
                content,
            }) => {
                assert_eq!(review_id.to_string(), "review-123");
                assert_eq!(file_path.to_string(), "src/main.rs");
                assert_eq!(*line_number, None);
                assert_eq!(content.to_string(), "This is a test comment");
            }
            _ => panic!("Expected CommentCreate event"),
        }
    }

    #[tokio::test]
    async fn test_comments_view_enter_with_empty_input() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        // Input is empty
        assert_eq!(view.input_text, "");

        // Press Enter
        let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should not send any events
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_comments_view_escape_closes() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send ViewClose event
        let event = app.events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::ViewClose) => {}
            _ => panic!("Expected ViewClose event"),
        }
    }

    #[test]
    fn test_comments_view_debug_state() {
        let view =
            CommentsView::new_for_line("review-123".to_string(), "src/test.rs".to_string(), 10);

        let debug_state = view.debug_state();
        assert!(debug_state.contains("input_text: \"\""));
        assert!(debug_state.contains("loading_state: \"Init\""));
        assert!(debug_state.contains("comments_count: 0"));
    }

    #[test]
    fn test_comments_view_keybindings() {
        let view = CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());

        let keybindings = view.get_keybindings();
        assert_eq!(keybindings.len(), 7);
        assert_eq!(keybindings[0].key, "Tab");
        assert_eq!(keybindings[0].description, "Switch focus");
        assert_eq!(keybindings[1].key, "Enter");
        assert_eq!(keybindings[1].description, "Add comment");
        assert_eq!(keybindings[2].key, "k/↑");
        assert!(keybindings[2].description.contains("Navigate up"));
        assert_eq!(keybindings[3].key, "j/↓");
        assert!(keybindings[3].description.contains("Navigate down"));
        assert_eq!(keybindings[4].key, "r");
        assert!(keybindings[4].description.contains("Toggle resolved"));
        assert_eq!(keybindings[5].key, "R");
        assert!(keybindings[5].description.contains("Toggle all resolved"));
        assert_eq!(keybindings[6].key, "Esc");
        assert_eq!(keybindings[6].description, "Close comments");
    }

    #[tokio::test]
    async fn test_comments_view_handles_loading_state() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        // Test loading state
        view.handle_app_events(
            &mut app,
            &AppEvent::CommentsLoadingState {
                params: CommentsLoadParams {
                    review_id: Arc::from("review-123"),
                    file_path: Arc::from(Some("src/main.rs".to_string())),
                    line_number: Arc::from(None),
                },
                state: CommentsLoadingState::Loading,
            },
        );
        assert!(matches!(view.loading_state, CommentsLoadingState::Loading));

        // Test loaded state
        let test_comments = vec![Comment::test_comment(
            "review-123",
            "src/main.rs",
            None,
            "Test comment",
        )];
        view.handle_app_events(
            &mut app,
            &AppEvent::CommentsLoadingState {
                params: CommentsLoadParams {
                    review_id: Arc::from("review-123"),
                    file_path: Arc::from(Some("src/main.rs".to_string())),
                    line_number: Arc::from(None),
                },
                state: CommentsLoadingState::Loaded(Arc::new(test_comments.clone())),
            },
        );
        assert!(matches!(
            view.loading_state,
            CommentsLoadingState::Loaded(_)
        ));
        assert_eq!(view.comments.len(), 1);
        assert_eq!(view.comments[0].content, "Test comment");
    }
}
