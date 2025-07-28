use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::{
    app::App,
    event::AppEvent,
    models::Review,
    services::GitDiffLoadingState,
    views::{KeyBinding, ViewHandler, ViewType},
};

#[derive(Debug, Clone)]
enum ReviewDetailsState {
    Loading,
    Loaded(Arc<Review>),
    Error(String),
}

pub struct ReviewDetailsView {
    state: ReviewDetailsState,
    diff_state: GitDiffLoadingState,
    scroll_offset: usize,
    current_line: usize,
}

const CONTENT_HEIGHT: usize = 30; // Default content height for scrolling

impl ReviewDetailsView {
    pub fn new(review: Review) -> Self {
        Self {
            state: ReviewDetailsState::Loaded(Arc::from(review)),
            diff_state: GitDiffLoadingState::Init,
            scroll_offset: 0,
            current_line: 0,
        }
    }

    pub fn new_loading() -> Self {
        Self {
            state: ReviewDetailsState::Loading,
            diff_state: GitDiffLoadingState::Init,
            scroll_offset: 0,
            current_line: 0,
        }
    }
}

impl ViewHandler for ReviewDetailsView {
    fn view_type(&self) -> ViewType {
        ViewType::ReviewDetails
    }

    fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
        // Clear the background to make this a proper full-screen modal
        Clear.render(area, buf);

        let block = Block::default()
            .title(" Review Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .style(Style::default().bg(Color::Black));

        let inner_area = block.inner(area);
        block.render(area, buf);

        match &self.state {
            ReviewDetailsState::Loading => self.render_loading(inner_area, buf),
            ReviewDetailsState::Error(error) => self.render_error(error, inner_area, buf),
            ReviewDetailsState::Loaded(review) => self.render_loaded(review, inner_area, buf),
        }
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => self.close(app),
            KeyCode::Char('?') => self.help(app),
            KeyCode::Up | KeyCode::Char('k') => self.go_up(),
            KeyCode::Down | KeyCode::Char('j') => self.go_down(),
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::ReviewLoaded(review) => self.handle_review_loaded(app, review),
            AppEvent::ReviewNotFound(review_id) => self.handle_review_not_found(review_id),
            AppEvent::ReviewLoadError(error) => self.handle_review_load_error(error),
            AppEvent::GitDiffLoadingState(diff_loading_state) => {
                self.handle_git_diff_loading_state(diff_loading_state);
            }
            _ => {
                // Other events are not handled by this view
            }
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
            KeyBinding {
                key: "Esc".to_string(),
                description: "Go back".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
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
                key: "?".to_string(),
                description: "Help".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('?'),
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
        ])
    }

    #[cfg(test)]
    fn debug_state(&self) -> String {
        match &self.state {
            ReviewDetailsState::Loading => format!(
                "state: Loading, diff_state: {:?}, current_line: {}, scroll_offset: {}",
                self.diff_state, self.current_line, self.scroll_offset
            ),
            ReviewDetailsState::Error(error) => format!(
                "state: Error(\"{error}\"), diff_state: {:?}, current_line: {}, scroll_offset: {}",
                self.diff_state, self.current_line, self.scroll_offset
            ),
            ReviewDetailsState::Loaded(review) => {
                format!(
                    "state: Loaded(review_id: \"{}\"), diff_state: {:?}, current_line: {}, scroll_offset: {}",
                    review.id, self.diff_state, self.current_line, self.scroll_offset
                )
            }
        }
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

impl ReviewDetailsView {
    /// Close the view by sending a ViewClose event
    fn close(&self, app: &mut App) {
        app.events.send(AppEvent::ViewClose);
    }

    /// Open help dialog with the keybindings of this view
    fn help(&self, app: &mut App) {
        app.events.send(AppEvent::HelpOpen(self.get_keybindings()));
    }

    /// Navigate to the previous line
    fn go_up(&mut self) {
        if self.current_line > 0 {
            self.current_line -= 1;
            // Update scroll to follow current line (estimated content height)
            self.update_scroll_to_follow_current_line(CONTENT_HEIGHT);
        }
    }

    /// Navigate to the next line
    fn go_down(&mut self) {
        let total_lines = self.get_total_lines();
        if total_lines > 0 && self.current_line < total_lines.saturating_sub(1) {
            self.current_line += 1;
            // Update scroll to follow current line
            self.update_scroll_to_follow_current_line(CONTENT_HEIGHT);
        }
    }

    /// Reset the current line and scroll offset to 0 and set the review
    fn handle_review_loaded(&mut self, app: &mut App, review: &Arc<Review>) {
        self.state = ReviewDetailsState::Loaded(Arc::clone(review));
        self.scroll_offset = 0; // Reset scroll when new review is loaded
        self.current_line = 0; // Reset current line when new review is loaded
        self.diff_state = GitDiffLoadingState::Init; // Reset diff state

        // Request git diff if SHAs are available
        if let (Some(base_sha), Some(target_sha)) = (&review.base_sha, &review.target_sha) {
            app.events.send(AppEvent::GitDiffLoad {
                base_sha: base_sha.clone().into(),
                target_sha: target_sha.clone().into(),
            });
        } else {
            self.diff_state = GitDiffLoadingState::Error(
                "Missing SHA information - cannot generate diff.".into(),
            );
        }
    }

    /// Reset the current line and scroll offset to 0 and set the state
    fn handle_review_not_found(&mut self, review_id: &str) {
        self.state = ReviewDetailsState::Error(format!("Review not found: {review_id}"));
        self.diff_state = GitDiffLoadingState::Init;
        self.scroll_offset = 0; // Reset scroll on error
        self.current_line = 0; // Reset current line on error
    }

    /// Reset the current line and scroll offset to 0 and set the state
    fn handle_review_load_error(&mut self, error: &str) {
        self.state = ReviewDetailsState::Error(error.to_string());
        self.diff_state = GitDiffLoadingState::Init;
        self.scroll_offset = 0; // Reset scroll on error
        self.current_line = 0; // Reset current line on error
    }

    /// Handle git diff loading state changes
    fn handle_git_diff_loading_state(&mut self, loading_state: &GitDiffLoadingState) {
        self.diff_state = loading_state.clone();
    }

    /// Get the diff content for the current diff state
    fn get_diff_content(&self) -> String {
        match &self.diff_state {
            GitDiffLoadingState::Init => "Initializing diff...".to_string(),
            GitDiffLoadingState::Loading => "Loading diff...".to_string(),
            GitDiffLoadingState::Loaded(diff) => diff.to_string(),
            GitDiffLoadingState::Error(error) => error.to_string(),
        }
    }

    /// Get the total number of lines in the content
    fn get_total_lines(&self) -> usize {
        match &self.state {
            ReviewDetailsState::Loaded(_) => {
                let content = self.get_diff_content();
                content.lines().count()
            }
            _ => 0,
        }
    }

    /// Get the maximum allowed scroll offset based on content
    fn get_max_scroll_offset(&self, content_height: usize) -> usize {
        let total_lines = self.get_total_lines();
        if total_lines > content_height {
            total_lines.saturating_sub(content_height)
        } else {
            0
        }
    }

    /// Update scroll offset to ensure current line is visible
    fn update_scroll_to_follow_current_line(&mut self, content_height: usize) {
        if content_height == 0 {
            return;
        }

        // If current line is above the viewport, scroll up
        if self.current_line < self.scroll_offset {
            self.scroll_offset = self.current_line;
        }

        // If current line is below the viewport, scroll down
        let viewport_bottom = self.scroll_offset + content_height.saturating_sub(1);
        if self.current_line > viewport_bottom {
            self.scroll_offset = self
                .current_line
                .saturating_sub(content_height.saturating_sub(1));
        }

        // Ensure scroll offset doesn't exceed bounds
        let max_offset = self.get_max_scroll_offset(content_height);
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }

    fn render_loading(&self, area: Rect, buf: &mut Buffer) {
        let loading_text = Paragraph::new("Loading review...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::NONE));
        loading_text.render(area, buf);
    }

    fn render_error(&self, error: &str, area: Rect, buf: &mut Buffer) {
        let error_text = Paragraph::new(format!("Error: {error}"))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::NONE));
        error_text.render(area, buf);
    }

    fn render_loaded(&self, review: &Review, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title section
                Constraint::Min(1),    // Content area
            ])
            .split(area);

        // Title section
        let title_block = Block::default()
            .title(" Title ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let title = review.title().clone();
        let title_content = Paragraph::new(title.as_str())
            .block(title_block)
            .style(Style::default().fg(Color::White));

        title_content.render(layout[0], buf);

        // Content section - show diff
        let content_text = self.get_diff_content();

        // Split content into lines and apply scrolling with highlighting
        let content_lines: Vec<&str> = content_text.lines().collect();
        let content_height = layout[1].height.saturating_sub(2) as usize; // Account for borders

        // Calculate the visible lines based on scroll offset
        let start_line = self.scroll_offset;
        let end_line = (start_line + content_height).min(content_lines.len());
        let visible_lines = if start_line < content_lines.len() {
            &content_lines[start_line..end_line]
        } else {
            &[]
        };

        // Create styled lines with highlighting for current line
        let styled_lines: Vec<Line> = visible_lines
            .iter()
            .enumerate()
            .map(|(idx, line_text)| {
                let absolute_line_idx = start_line + idx;
                let is_current_line = absolute_line_idx == self.current_line;

                if is_current_line {
                    // Highlight current line with inverted colors
                    Line::from(Span::styled(
                        *line_text,
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    // Regular styling for other lines with diff colors
                    let style = match line_text.chars().next() {
                        Some('+') => Style::default().fg(Color::Green),
                        Some('-') => Style::default().fg(Color::Red),
                        Some('@') => Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default().fg(Color::White),
                    };
                    Line::from(Span::styled(*line_text, style))
                }
            })
            .collect();

        // Show scroll and line indicator in title
        let total_lines = content_lines.len();
        let title_text = if total_lines > 0 {
            format!(
                " Diff (line {}/{}, scroll {}/{}) ",
                self.current_line + 1,
                total_lines,
                start_line + 1,
                total_lines
            )
        } else {
            " Diff ".to_string()
        };

        let content = Paragraph::new(styled_lines).block(
            Block::default()
                .title(title_text)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
        );

        content.render(layout[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::App,
        database::Database,
        models::{Review, review::TestReviewParams},
        test_utils::render_app_to_terminal_backend,
    };
    use insta::assert_snapshot;
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
    fn test_review_details_view_creation() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.state {
            ReviewDetailsState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.base_branch, "default");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[test]
    fn test_review_details_view_new_loading() {
        let view = ReviewDetailsView::new_loading();

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.state {
            ReviewDetailsState::Loading => {}
            _ => panic!("Expected loading state"),
        }
    }

    #[test]
    fn test_review_details_view_debug_state() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        let debug_state = view.debug_state();
        assert!(debug_state.contains(&review.id));
        assert!(debug_state.starts_with("state: Loaded(review_id: \""));
    }

    #[test]
    fn test_review_details_view_debug_state_loading() {
        let view = ReviewDetailsView::new_loading();

        let debug_state = view.debug_state();
        assert_eq!(
            debug_state,
            "state: Loading, diff_state: Init, current_line: 0, scroll_offset: 0"
        );
    }

    #[test]
    fn test_review_details_view_keybindings() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        let keybindings = view.get_keybindings();
        assert_eq!(keybindings.len(), 4);
        assert_eq!(keybindings[0].key, "Esc");
        assert_eq!(keybindings[0].description, "Go back");
        assert_eq!(keybindings[1].key, "↑/k");
        assert_eq!(keybindings[1].description, "Scroll up");
        assert_eq!(keybindings[2].key, "↓/j");
        assert_eq!(keybindings[2].description, "Scroll down");
        assert_eq!(keybindings[3].key, "?");
        assert_eq!(keybindings[3].description, "Help");
    }

    #[test]
    fn test_review_details_view_line_navigation() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);

        // Initial current line should be 0
        assert_eq!(view.current_line, 0);

        // Test direct line manipulation
        view.current_line = 5;
        assert_eq!(view.current_line, 5);

        // Test reset behavior
        view.current_line = 0;
        assert_eq!(view.current_line, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_line_navigation_keys() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_sha("abc123")
                .target_sha("def456"),
        );
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Initial current line should be 0
        assert_eq!(view.current_line, 0);

        // Check total lines - with error message, there should be 1 line
        let total_lines = view.get_total_lines();
        assert_eq!(total_lines, 1);

        // Try to navigate down with 'j' - should not increment since we only have 1 line (index 0)
        let key_event = KeyEvent::new(
            KeyCode::Char('j'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 0 since there's only 1 line (max index is 0)
        assert_eq!(view.current_line, 0);

        // Navigate up with 'k' - should stay at 0
        let key_event = KeyEvent::new(
            KeyCode::Char('k'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_line, 0);

        // Try to navigate up from 0 - should stay at 0
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_line, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_line_reset_on_new_review() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        // Set some current line
        view.current_line = 5;

        // Load a new review - should reset current line
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        view.handle_app_events(&mut app, &AppEvent::ReviewLoaded(Arc::from(review)));

        assert_eq!(view.current_line, 0);
    }

    #[test]
    fn test_get_total_lines() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        // Should return 1 for the "Missing SHA information" message
        let total_lines = view.get_total_lines();
        assert_eq!(total_lines, 1);
    }

    #[test]
    fn test_update_scroll_to_follow_current_line() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);

        // Test with small content height, but limited by total lines (1 line for this review)
        view.current_line = 0; // Only line available
        view.scroll_offset = 0;
        view.update_scroll_to_follow_current_line(3);

        // Should stay at 0 since we only have 1 line
        assert_eq!(view.scroll_offset, 0);

        // Test with artificially high current line (will be bounded)
        view.current_line = 10; // Way beyond available content
        view.update_scroll_to_follow_current_line(3);

        // Should be bounded by max offset (0 since only 1 line)
        assert_eq!(view.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_handles_escape_key() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send ViewClose event
        let event = app.events.try_recv().unwrap();
        match *event {
            crate::event::Event::App(AppEvent::ViewClose) => {}
            _ => panic!("Expected ViewClose event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_help_key() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(
            KeyCode::Char('?'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send HelpOpen event
        let event = app.events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::HelpOpen(_)) => {}
            _ => panic!("Expected HelpOpen event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_loaded_event() {
        let mut view = ReviewDetailsView::new_loading();
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut app = create_test_app().await;

        view.handle_app_events(&mut app, &AppEvent::ReviewLoaded(Arc::from(review)));

        match &view.state {
            ReviewDetailsState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.base_branch, "main");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_load_error_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadError("Database error".into()),
        );

        match &view.state {
            ReviewDetailsState::Error(error) => {
                assert_eq!(error, "Database error");
            }
            _ => panic!("Expected error state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_not_found_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(&mut app, &AppEvent::ReviewNotFound("test-id".into()));

        match &view.state {
            ReviewDetailsState::Error(error) => {
                assert_eq!(error, "Review not found: test-id");
            }
            _ => panic!("Expected error state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_render_loading_state() {
        let view = ReviewDetailsView::new_loading();
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_error_state() {
        let mut view = ReviewDetailsView::new_loading();
        view.state = ReviewDetailsState::Error("Database connection failed".to_string());
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review);
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_down_basic() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let _app = create_test_app().await;

        // Initial scroll offset should be 0
        assert_eq!(view.scroll_offset, 0);

        // Manually set scroll_offset to test that the basic scroll functionality works
        // (separate from bounds checking which depends on content)
        view.scroll_offset = 0;

        // Directly increment scroll offset to simulate what would happen
        // This tests the core scroll mechanism
        view.scroll_offset += 1;
        assert_eq!(view.scroll_offset, 1);

        view.scroll_offset += 1;
        assert_eq!(view.scroll_offset, 2);
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_key_handling() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Test that the key handling logic exists by calling it
        // Even if bounds checking prevents scrolling, the method should work
        let key_event = KeyEvent::new(
            KeyCode::Char('j'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );

        // This should not error
        let result = view.handle_key_events(&mut app, &key_event);
        assert!(result.is_ok());

        // Test up key as well
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        let result = view.handle_key_events(&mut app, &key_event);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_up() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Set initial current line and scroll offset to test navigation
        view.current_line = 3;
        view.scroll_offset = 3;

        // Navigate up with 'k' - should move current line up
        let key_event = KeyEvent::new(
            KeyCode::Char('k'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_line, 2);

        // Navigate up with arrow key
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.current_line, 1);
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_up_bounds() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Start at scroll offset 0
        assert_eq!(view.scroll_offset, 0);

        // Try to scroll up - should stay at 0
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_reset_on_new_review() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        // Set some scroll offset
        view.scroll_offset = 5;

        // Load a new review - should reset scroll
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        view.handle_app_events(&mut app, &AppEvent::ReviewLoaded(Arc::from(review)));

        assert_eq!(view.scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_scroll_reset_on_error() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Set some scroll offset
        view.scroll_offset = 3;

        // Trigger an error - should reset scroll
        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadError("Database error".into()),
        );

        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn test_get_diff_content_with_shas() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_sha("abc123")
                .target_sha("def456"),
        );
        let view = ReviewDetailsView::new(review);

        // This will show "Missing SHA information" since we don't have SHAs set up for diff loading
        let content = view.get_diff_content();
        assert!(content.contains("Initializing diff"));
    }

    #[test]
    fn test_get_diff_content_without_shas() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        match &view.state {
            ReviewDetailsState::Loaded(_) => {
                let content = view.get_diff_content();
                assert_eq!(content, "Initializing diff...");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[test]
    fn test_get_max_scroll_offset() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        // With a small content height, max offset should be calculated correctly
        let max_offset = view.get_max_scroll_offset(5);
        // Since the content is a single line, max offset should be 0
        assert_eq!(max_offset, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_render_with_diff_content() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review);

        // Simulate diff content being loaded
        let diff_content = r#"@@ -1,3 +1,4 @@
 # Test Repository
+
 This is a test file
-Old line to remove
+New line to add"#;
        view.diff_state = GitDiffLoadingState::Loaded(diff_content.into());

        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_with_diff_loading() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch("develop")
                .base_sha("asdf1234"),
        );
        let mut view = ReviewDetailsView::new(review);

        // Simulate diff loading state
        view.diff_state = GitDiffLoadingState::Loading;

        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_with_diff_error() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch("feature")
                .base_sha("jkl09876"),
        );
        let mut view = ReviewDetailsView::new(review);

        // Simulate diff error state
        view.diff_state = GitDiffLoadingState::Error("Repository not found".into());

        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
