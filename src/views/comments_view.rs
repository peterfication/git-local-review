use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    models::Comment,
    services::CommentsLoadingState,
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

pub struct CommentsView {
    /// The target for comments (file or line)
    target: CommentTarget,
    /// Current input text for new comment
    input_text: String,
    /// Current loading state of comments
    loading_state: CommentsLoadingState,
    /// Comments list (cached from loading state)
    comments: Arc<Vec<Comment>>,
    /// Scroll offset for comments list
    scroll_offset: usize,
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
            scroll_offset: 0,
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
            scroll_offset: 0,
        }
    }

    fn handle_enter(&mut self, app: &mut App) {
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

    fn handle_backspace(&mut self) {
        self.input_text.pop();
    }

    fn handle_char(&mut self, c: char) {
        // Limit input length to prevent very long comments
        if self.input_text.len() < 1000 {
            self.input_text.push(c);
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        if !self.comments.is_empty() {
            self.scroll_offset =
                (self.scroll_offset + 1).min(self.comments.len().saturating_sub(1));
        }
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
            KeyCode::Enter => self.handle_enter(app),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::Esc => {
                app.events.send(AppEvent::ViewClose);
            }
            KeyCode::Char(c) => self.handle_char(c),
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::CommentsLoadingState(loading_state) => {
                self.handle_comments_loading_state(loading_state);
            }
            AppEvent::CommentCreated(_comment) => {
                // Reload comments when a new comment is created
                self.request_comments_reload(app);
            }
            AppEvent::CommentCreateError(_error) => {
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
                key: "Enter".to_string(),
                description: "Add comment".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "↑/k".to_string(),
                description: "Scroll up".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Up,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "↓/j".to_string(),
                description: "Scroll down".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Down,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Esc".to_string(),
                description: "Close comments".to_string(),
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
    fn debug_state(&self) -> String {
        format!(
            "target: {:?}, input_text: {:?}, loading_state: {:?}, comments_count: {}, scroll_offset: {}",
            self.target,
            self.input_text,
            match &self.loading_state {
                CommentsLoadingState::Init => "Init".to_string(),
                CommentsLoadingState::Loading => "Loading".to_string(),
                CommentsLoadingState::Loaded(comments) => format!("Loaded({})", comments.len()),
                CommentsLoadingState::Error(error) => format!("Error({error})"),
            },
            self.comments.len(),
            self.scroll_offset
        )
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

impl CommentsView {
    fn render_input_field(&self, area: Rect, buf: &mut Buffer) {
        let input_block = Block::default()
            .title(" New Comment ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

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
        if self.comments.is_empty() {
            let empty_text = Paragraph::new("No comments yet. Add one above!")
                .style(Style::default().fg(Color::Gray))
                .block(
                    Block::default()
                        .title(" Comments ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Gray)),
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
                    .title(format!(" Comments ({}) ", self.comments.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .style(Style::default().fg(Color::White));

        comments_list.render(area, buf);
    }

    fn render_comment_item(&self, _index: usize, comment: &Comment) -> ListItem {
        // Format the comment with timestamp and content
        let timestamp = comment.created_at.format("%Y-%m-%d %H:%M:%S");

        let comment_type = if comment.is_file_comment() {
            "FILE"
        } else {
            &format!("LINE {}", comment.line_number.unwrap_or(0))
        };

        let content = vec![
            Line::from(vec![
                Span::styled(
                    format!("[{comment_type}] "),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(timestamp.to_string(), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(Span::styled(
                comment.content.clone(),
                Style::default().fg(Color::White),
            )),
            Line::from(""), // Empty line for spacing
        ];

        ListItem::new(content)
    }

    fn handle_comments_loading_state(&mut self, loading_state: &CommentsLoadingState) {
        self.loading_state = loading_state.clone();

        if let CommentsLoadingState::Loaded(comments) = loading_state {
            self.comments = comments.clone();
            self.scroll_offset = 0; // Reset scroll when comments are loaded
        }
    }

    fn request_comments_reload(&self, app: &mut App) {
        match &self.target {
            CommentTarget::File {
                review_id,
                file_path,
            } => {
                app.events.send(AppEvent::CommentsLoad {
                    review_id: review_id.clone().into(),
                    file_path: file_path.clone().into(),
                    line_number: None,
                });
            }
            CommentTarget::Line {
                review_id,
                file_path,
                line_number,
            } => {
                app.events.send(AppEvent::CommentsLoad {
                    review_id: review_id.clone().into(),
                    file_path: file_path.clone().into(),
                    line_number: Some(*line_number),
                });
            }
        }
    }

    /// Request initial loading of comments when the view is opened
    pub fn request_initial_load(&self, app: &mut App) {
        self.request_comments_reload(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::App, database::Database, models::Comment};
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
        let key_event = KeyEvent::new(
            KeyCode::Char('H'),
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.input_text, "H");

        // Test more characters
        let key_event = KeyEvent::new(
            KeyCode::Char('i'),
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.input_text, "Hi");

        // Test backspace
        let key_event = KeyEvent::new(
            KeyCode::Backspace,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
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
        let key_event = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
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
        let key_event = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should not send any events
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_comments_view_escape_closes() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
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
        assert!(debug_state.contains("scroll_offset: 0"));
    }

    #[test]
    fn test_comments_view_keybindings() {
        let view = CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());

        let keybindings = view.get_keybindings();
        assert_eq!(keybindings.len(), 4);
        assert_eq!(keybindings[0].key, "Enter");
        assert_eq!(keybindings[0].description, "Add comment");
        assert_eq!(keybindings[1].key, "↑/k");
        assert_eq!(keybindings[1].description, "Scroll up");
        assert_eq!(keybindings[2].key, "↓/j");
        assert_eq!(keybindings[2].description, "Scroll down");
        assert_eq!(keybindings[3].key, "Esc");
        assert_eq!(keybindings[3].description, "Close comments");
    }

    #[tokio::test]
    async fn test_comments_view_handles_loading_state() {
        let mut view =
            CommentsView::new_for_file("review-123".to_string(), "src/main.rs".to_string());
        let mut app = create_test_app().await;

        // Test loading state
        view.handle_app_events(
            &mut app,
            &AppEvent::CommentsLoadingState(CommentsLoadingState::Loading),
        );
        assert!(matches!(view.loading_state, CommentsLoadingState::Loading));

        // Test loaded state
        let test_comments = vec![Comment::test_file_comment(
            "review-123".to_string(),
            "src/main.rs".to_string(),
            "Test comment".to_string(),
        )];
        view.handle_app_events(
            &mut app,
            &AppEvent::CommentsLoadingState(CommentsLoadingState::Loaded(Arc::new(
                test_comments.clone(),
            ))),
        );
        assert!(matches!(
            view.loading_state,
            CommentsLoadingState::Loaded(_)
        ));
        assert_eq!(view.comments.len(), 1);
        assert_eq!(view.comments[0].content, "Test comment");
    }
}
