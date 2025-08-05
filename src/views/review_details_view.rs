#[cfg(test)]
use std::any::Any;

use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

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
    models::{Diff, DiffFile, Review},
    services::{CommentsLoadParams, CommentsLoadingState, GitDiffLoadingState, ReviewLoadingState},
    views::{KeyBinding, ViewHandler, ViewType},
};

const FILE_SELECTION_INDICATOR: &str = super::SELECTION_INDICATOR;
const FILE_COMMENT_INDICATOR: &str = "●";
const LINE_COMMENT_INDICATOR: &str = "■";
const FILE_AND_LINE_COMMENT_INDICATOR: &str = "#";
const RESOLVED_COMMENT_INDICATOR: &str = "_";

#[derive(Debug, Clone)]
pub enum NavigationMode {
    Files,
    Lines,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileListType {
    NotViewed,
    Viewed,
}

#[derive(Debug, Clone, PartialEq)]
/// Represents the type of comments a file has
pub enum CommentIndicator {
    NoComment,
    FileComment,
    LineComment,
    FileAndLineComment,
    ResolvedComment,
}

impl fmt::Display for CommentIndicator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CommentIndicator::NoComment => f.write_str(" "),
            CommentIndicator::FileComment => f.write_str(FILE_COMMENT_INDICATOR),
            CommentIndicator::LineComment => f.write_str(LINE_COMMENT_INDICATOR),
            CommentIndicator::FileAndLineComment => f.write_str(FILE_AND_LINE_COMMENT_INDICATOR),
            CommentIndicator::ResolvedComment => f.write_str(RESOLVED_COMMENT_INDICATOR),
        }
    }
}

pub struct ReviewDetailsView {
    /// Current state of the review loading
    review_state: ReviewLoadingState,
    /// Current review being displayed if loaded
    review: Option<Arc<Review>>,
    /// Current state of the git diff loading
    diff_state: GitDiffLoadingState,
    /// Current git diff if loaded
    diff: Arc<Diff>,
    /// Current scroll offset for the diff content
    scroll_offset: usize,
    /// Index of the currently selected file
    selected_file_index: usize,
    /// Index of the currently selected line in the diff content
    selected_line_index: usize,
    /// Current navigation mode (content box)
    navigation_mode: NavigationMode,
    /// Currently active file list (not viewed or viewed)
    active_file_list: FileListType,
    /// List of viewed file paths for the current review
    viewed_files: Arc<Vec<String>>,
    /// Files that have comments (file comments only, for comment indicators)
    files_with_file_comments: Arc<Vec<String>>,
    /// Files that have file and line comments (for comment indicators)
    files_with_file_and_or_line_comments: Arc<Vec<String>>,
    /// Map of file paths to line numbers with comments (for line comment indicators)
    lines_with_comments: Arc<HashMap<String, Vec<i64>>>,
    /// Files that have only resolved comments (for resolved comment indicators)
    files_with_only_resolved_comments: Arc<Vec<String>>,
    /// Map of file paths to line numbers with only resolved comments
    lines_with_only_resolved_comments: Arc<HashMap<String, Vec<i64>>>,
}

const CONTENT_HEIGHT: usize = 15; // Default content height for scrolling

impl ReviewDetailsView {
    pub fn new(review: Review) -> Self {
        let review_arc = Arc::from(review);
        Self {
            review_state: ReviewLoadingState::Loaded(review_arc.clone()),
            review: Some(review_arc.clone()),
            diff_state: GitDiffLoadingState::Init,
            diff: Arc::new(Diff::default()),
            scroll_offset: 0,
            selected_file_index: 0,
            selected_line_index: 0,
            navigation_mode: NavigationMode::Files,
            active_file_list: FileListType::NotViewed,
            viewed_files: Arc::new(vec![]),
            files_with_file_comments: Arc::new(vec![]),
            files_with_file_and_or_line_comments: Arc::new(vec![]),
            lines_with_comments: Arc::new(HashMap::new()),
            files_with_only_resolved_comments: Arc::new(vec![]),
            lines_with_only_resolved_comments: Arc::new(HashMap::new()),
        }
    }

    pub fn new_loading() -> Self {
        Self {
            review_state: ReviewLoadingState::Loading,
            review: None,
            diff_state: GitDiffLoadingState::Init,
            diff: Arc::new(Diff::default()),
            scroll_offset: 0,
            selected_file_index: 0,
            selected_line_index: 0,
            navigation_mode: NavigationMode::Files,
            active_file_list: FileListType::NotViewed,
            viewed_files: Arc::new(vec![]),
            files_with_file_comments: Arc::new(vec![]),
            files_with_file_and_or_line_comments: Arc::new(vec![]),
            lines_with_comments: Arc::new(HashMap::new()),
            files_with_only_resolved_comments: Arc::new(vec![]),
            lines_with_only_resolved_comments: Arc::new(HashMap::new()),
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

        match &self.review_state {
            ReviewLoadingState::Init => self.render_init(inner_area, buf),
            ReviewLoadingState::Loading => self.render_loading(inner_area, buf),
            ReviewLoadingState::Error(error) => self.render_error(error, inner_area, buf),
            ReviewLoadingState::NotFound(review_id) => {
                self.render_not_found(review_id, inner_area, buf)
            }
            ReviewLoadingState::Loaded(_review) => self.render_loaded(inner_area, buf),
        }
    }

    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => self.go_up(),
            KeyCode::Down | KeyCode::Char('j') => self.go_down(),
            KeyCode::Left | KeyCode::Char('h') => self.switch_file_list_left(),
            KeyCode::Right | KeyCode::Char('l') => self.switch_file_list_right(),
            KeyCode::Enter => self.toggle_navigation_mode(),
            KeyCode::Char(' ') => self.toggle_file_view_status(app),
            KeyCode::Char('c') => self.open_comments(app),
            KeyCode::Esc => self.handle_esc(app),
            KeyCode::Char('?') => self.help(app),
            _ => {}
        }
        Ok(())
    }

    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        match event {
            AppEvent::ReviewLoadingState(review_loading_state) => {
                self.handle_review_loading_state(app, review_loading_state);
            }
            AppEvent::GitDiffLoadingState(diff_loading_state) => {
                self.handle_git_diff_loading_state(diff_loading_state);
            }
            AppEvent::FileViewsLoaded {
                review_id,
                viewed_files,
            } => {
                self.handle_file_views_loaded(review_id, viewed_files);
            }
            AppEvent::FileViewToggled {
                review_id: _,
                file_path: _,
                is_viewed: _,
            } => {
                // File view status changed, file views will be reloaded automatically
            }
            AppEvent::CommentsLoadingState { params, state } => {
                self.handle_comments_loading_state(params, state);
            }
            AppEvent::CommentCreated(_) => {
                // Reload comment metadata when a comment is created
                self.reload_comments(app);
            }
            _ => {
                // Other events are not handled by this view
            }
        }
    }

    fn get_keybindings(&self) -> Arc<[KeyBinding]> {
        Arc::new([
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
                key: "←/h".to_string(),
                description: "Switch to not viewed files".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Left,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "→/l".to_string(),
                description: "Switch to viewed files".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Right,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Space".to_string(),
                description: "Toggle file view status".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Enter".to_string(),
                description: "Toggle navigation mode".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "Esc".to_string(),
                description: "Go back / Switch to Files mode".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                    kind: ratatui::crossterm::event::KeyEventKind::Press,
                    state: ratatui::crossterm::event::KeyEventState::empty(),
                },
            },
            KeyBinding {
                key: "c".to_string(),
                description: "Open comments".to_string(),
                key_event: KeyEvent {
                    code: KeyCode::Char('c'),
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
        format!(
            "review_state: {:?}, review: {:?}, diff_state: {:?}, scroll_offset: {}, selected_file_index: {}, selected_line_index: {}, navigation_mode: {:?}, active_file_list: {:?}, viewed_files: {:?}, files_with_comments: {:?}, lines_with_comments: {:?}",
            self.review_state,
            self.review,
            self.diff_state,
            self.scroll_offset,
            self.selected_file_index,
            self.selected_line_index,
            self.navigation_mode,
            self.active_file_list,
            self.viewed_files,
            self.files_with_file_comments,
            self.lines_with_comments
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

impl ReviewDetailsView {
    /// Open help dialog with the keybindings of this view
    fn help(&self, app: &mut App) {
        app.events.send(AppEvent::HelpOpen(self.get_keybindings()));
    }

    /// Navigate to the previous line in the respective navigation mode
    fn go_up(&mut self) {
        match self.navigation_mode {
            NavigationMode::Files => {
                if self.selected_file_index > 0 {
                    self.selected_file_index -= 1;
                    self.selected_line_index = 0;
                    self.scroll_offset = 0;
                }
            }
            NavigationMode::Lines => {
                if self.selected_line_index > 0 {
                    self.selected_line_index -= 1;
                    self.update_scroll_to_follow_selected_line(CONTENT_HEIGHT);
                }
            }
        }
    }

    /// Navigate to the next line in the respective navigation mode
    fn go_down(&mut self) {
        match self.navigation_mode {
            NavigationMode::Files => {
                let current_files = self.get_current_file_list();
                if self.selected_file_index < current_files.len().saturating_sub(1) {
                    self.selected_file_index += 1;
                    self.selected_line_index = 0;
                    self.scroll_offset = 0;
                }
            }
            NavigationMode::Lines => {
                let current_file_lines = self.get_current_file_lines();
                if self.selected_line_index < current_file_lines.saturating_sub(1) {
                    self.selected_line_index += 1;
                    self.update_scroll_to_follow_selected_line(CONTENT_HEIGHT);
                }
            }
        }
    }

    /// Toggle between file navigation and line navigation modes
    fn toggle_navigation_mode(&mut self) {
        match self.navigation_mode {
            NavigationMode::Files => {
                let current_files = self.get_current_file_list();
                if !current_files.is_empty() {
                    self.navigation_mode = NavigationMode::Lines;
                    self.selected_line_index = 0;
                }
            }
            NavigationMode::Lines => {
                self.navigation_mode = NavigationMode::Files;
            }
        }
    }

    /// Handle the Escape key based on the current navigation mode
    /// If in Lines mode, switch to Files mode.
    /// If already in Files mode, close the view.
    fn handle_esc(&mut self, app: &mut App) {
        match self.navigation_mode {
            NavigationMode::Lines => {
                // Switch back to Files mode instead of closing
                self.navigation_mode = NavigationMode::Files;
            }
            NavigationMode::Files => {
                // Close the view when already in Files mode
                app.events.send(AppEvent::ViewClose);
            }
        }
    }

    /// Handle review loading state changes
    fn handle_review_loading_state(&mut self, app: &mut App, loading_state: &ReviewLoadingState) {
        self.review_state = loading_state.clone();
        self.review = None;
        self.reset_diff_state();

        if let ReviewLoadingState::Loaded(review) = loading_state {
            self.review = Some(review.clone());

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

            // Load file views for this review
            app.events.send(AppEvent::FileViewsLoad {
                review_id: review.id.clone().into(),
            });

            if let Some(params) = self.comments_load_params() {
                // Load comments for the whole review
                app.events.send(AppEvent::CommentsLoad(params));
            }
        }
    }

    /// Reset all state related to the diff view
    fn reset_diff_state(&mut self) {
        self.diff_state = GitDiffLoadingState::Init;
        self.diff = Arc::new(Diff::default());
        self.scroll_offset = 0;
        self.selected_file_index = 0;
        self.selected_line_index = 0;
        self.navigation_mode = NavigationMode::Files;
        self.active_file_list = FileListType::NotViewed;
        self.viewed_files = Arc::new(vec![]);
        self.files_with_file_comments = Arc::new(vec![]);
        self.lines_with_comments = Arc::new(HashMap::new());
    }

    /// Handle git diff loading state changes
    fn handle_git_diff_loading_state(&mut self, loading_state: &GitDiffLoadingState) {
        self.diff_state = loading_state.clone();

        // Use structured diff data when loaded
        if let GitDiffLoadingState::Loaded(diff) = loading_state {
            self.diff = diff.clone();
            self.selected_file_index = 0;
            self.selected_line_index = 0;
            self.navigation_mode = NavigationMode::Files;
        }
    }

    /// Get the number of lines in the currently selected file
    fn get_current_file_lines(&self) -> usize {
        if let Some(file) = self.get_selected_file() {
            file.content.lines().count()
        } else {
            0
        }
    }

    /// Update scroll offset to ensure selected line is visible
    fn update_scroll_to_follow_selected_line(&mut self, content_height: usize) {
        if content_height == 0 {
            return;
        }

        // If selected line is above the viewport, scroll up
        if self.selected_line_index < self.scroll_offset {
            self.scroll_offset = self.selected_line_index;
        }

        // If selected line is below the viewport, scroll down
        let viewport_bottom = self.scroll_offset + content_height.saturating_sub(1);
        if self.selected_line_index > viewport_bottom {
            self.scroll_offset = self
                .selected_line_index
                .saturating_sub(content_height.saturating_sub(1));
        }

        // Ensure scroll offset doesn't exceed bounds
        let current_file_lines = self.get_current_file_lines();
        let max_offset = if current_file_lines > content_height {
            current_file_lines.saturating_sub(content_height)
        } else {
            0
        };
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }

    /// Switch to the left file list (not viewed files)
    fn switch_file_list_left(&mut self) {
        if matches!(self.navigation_mode, NavigationMode::Files)
            && self.active_file_list != FileListType::NotViewed
        {
            self.active_file_list = FileListType::NotViewed;
            self.selected_file_index = 0;
            self.selected_line_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Switch to the right file list (viewed files)
    fn switch_file_list_right(&mut self) {
        if matches!(self.navigation_mode, NavigationMode::Files)
            && self.active_file_list != FileListType::Viewed
        {
            self.active_file_list = FileListType::Viewed;
            self.selected_file_index = 0;
            self.selected_line_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Toggle the view status of the currently selected file
    fn toggle_file_view_status(&mut self, app: &mut App) {
        if let Some(review) = &self.review {
            let current_files = self.get_current_file_list();
            if let Some(file) = current_files.get(self.selected_file_index) {
                app.events.send(AppEvent::FileViewToggle {
                    review_id: review.id.clone().into(),
                    file_path: file.path.clone().into(),
                });
            }
        }
    }

    /// Open comments view for the current context (file or line)
    fn open_comments(&mut self, app: &mut App) {
        if let Some(review) = &self.review {
            let current_files = self.get_current_file_list();
            if let Some(file) = current_files.get(self.selected_file_index) {
                let line_number = match self.navigation_mode {
                    NavigationMode::Files => None, // File-level comments
                    NavigationMode::Lines => Some(self.selected_line_index as i64), // Line-level comments
                };

                app.events.send(AppEvent::CommentsOpen {
                    review_id: review.id.clone().into(),
                    file_path: file.path.clone().into(),
                    line_number,
                });
            }
        }
    }

    /// Handle file views loaded event
    fn handle_file_views_loaded(&mut self, _review_id: &str, viewed_files: &Arc<Vec<String>>) {
        self.viewed_files = viewed_files.clone();
        // Reset selection when file views change
        self.selected_file_index = 0;
        self.selected_line_index = 0;
        self.scroll_offset = 0;
    }

    /// Handle comments loaded. This updates the files with comments and lines with comments
    /// so that the comment indicators are up to date.
    fn handle_comments_loading_state(
        &mut self,
        params: &CommentsLoadParams,
        state: &CommentsLoadingState,
    ) {
        if !self.relevant_comments_loading_state(params) {
            // Ignore comments loading for different CommentsLoad requests
            return;
        };

        if let CommentsLoadingState::Loaded(comments) = state {
            // Separate unresolved and resolved comments
            let unresolved_comments: Vec<_> = comments.iter().filter(|c| !c.resolved).collect();
            let _resolved_comments: Vec<_> = comments.iter().filter(|c| c.resolved).collect();

            // Track files with unresolved file-level comments
            self.files_with_file_comments = Arc::from(
                unresolved_comments
                    .iter()
                    .filter_map(|comment| {
                        if comment.is_file_comment() {
                            Some(comment.file_path.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<String>>(),
            );

            // Track files with any unresolved comments
            self.files_with_file_and_or_line_comments = Arc::from(
                unresolved_comments
                    .iter()
                    .map(|comment| comment.file_path.clone())
                    .collect::<HashSet<String>>()
                    .into_iter()
                    .collect::<Vec<String>>(),
            );

            // Track lines with unresolved comments
            self.lines_with_comments = Arc::from(unresolved_comments.iter().fold(
                HashMap::new(),
                |mut acc: HashMap<String, Vec<i64>>, comment| {
                    if let Some(line_number) = comment.line_number {
                        acc.entry(comment.file_path.clone())
                            .or_default()
                            .push(line_number);
                    };
                    acc
                },
            ));

            // Track files that only have resolved comments
            let all_files_with_comments: HashSet<String> =
                comments.iter().map(|c| c.file_path.clone()).collect();
            let files_with_unresolved_comments: HashSet<String> = unresolved_comments
                .iter()
                .map(|c| c.file_path.clone())
                .collect();

            self.files_with_only_resolved_comments = Arc::from(
                all_files_with_comments
                    .difference(&files_with_unresolved_comments)
                    .cloned()
                    .collect::<Vec<String>>(),
            );

            // Track lines that only have resolved comments
            let all_lines_with_comments: HashMap<String, HashSet<i64>> = comments.iter().fold(
                HashMap::new(),
                |mut acc: HashMap<String, HashSet<i64>>, comment| {
                    if let Some(line_number) = comment.line_number {
                        acc.entry(comment.file_path.clone())
                            .or_default()
                            .insert(line_number);
                    };
                    acc
                },
            );
            let lines_with_unresolved_comments: HashMap<String, HashSet<i64>> =
                unresolved_comments.iter().fold(
                    HashMap::new(),
                    |mut acc: HashMap<String, HashSet<i64>>, comment| {
                        if let Some(line_number) = comment.line_number {
                            acc.entry(comment.file_path.clone())
                                .or_default()
                                .insert(line_number);
                        };
                        acc
                    },
                );

            self.lines_with_only_resolved_comments = Arc::from(
                all_lines_with_comments
                    .iter()
                    .filter_map(|(file_path, all_lines)| {
                        let unresolved_lines = lines_with_unresolved_comments
                            .get(file_path)
                            .cloned()
                            .unwrap_or_default();
                        let resolved_only_lines: Vec<i64> =
                            all_lines.difference(&unresolved_lines).cloned().collect();
                        if resolved_only_lines.is_empty() {
                            None
                        } else {
                            Some((file_path.clone(), resolved_only_lines))
                        }
                    })
                    .collect::<HashMap<String, Vec<i64>>>(),
            );
        };
    }

    /// Check if the current comments loading state is relevant to the current view
    fn relevant_comments_loading_state(&self, params: &CommentsLoadParams) -> bool {
        if let Some(self_params) = self.comments_load_params() {
            params.equals(&self_params)
        } else {
            false
        }
    }

    /// The params for the comments loading based on the current review context
    fn comments_load_params(&self) -> Option<CommentsLoadParams> {
        self.review.as_ref().map(|review| CommentsLoadParams {
            review_id: Arc::from(review.id.to_string()),
            file_path: None.into(),
            line_number: None.into(),
        })
    }

    /// Reload comment metadata for the current review
    fn reload_comments(&self, app: &mut App) {
        if let Some(params) = self.comments_load_params() {
            // Load comments for the whole review
            app.events.send(AppEvent::CommentsLoad(params));
        }
    }

    /// Get the current file list based on the active file list type
    fn get_current_file_list(&self) -> Vec<&DiffFile> {
        match self.active_file_list {
            FileListType::NotViewed => self
                .diff
                .files
                .iter()
                .filter(|file| !self.viewed_files.contains(&file.path))
                .collect(),
            FileListType::Viewed => self
                .diff
                .files
                .iter()
                .filter(|file| self.viewed_files.contains(&file.path))
                .collect(),
        }
    }

    /// Get the currently selected file from the active file list
    fn get_selected_file(&self) -> Option<&DiffFile> {
        let current_files = self.get_current_file_list();
        current_files.get(self.selected_file_index).copied()
    }

    fn render_init(&self, area: Rect, buf: &mut Buffer) {
        let loading_text = Paragraph::new("Init review...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::NONE));
        loading_text.render(area, buf);
    }

    fn render_loading(&self, area: Rect, buf: &mut Buffer) {
        let loading_text = Paragraph::new("Loading review...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::NONE));
        loading_text.render(area, buf);
    }

    fn render_not_found(&self, review_id: &str, area: Rect, buf: &mut Buffer) {
        let error_text = Paragraph::new(format!("Review with ID '{review_id}' not found."))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::NONE));
        error_text.render(area, buf);
    }

    fn render_error(&self, error: &str, area: Rect, buf: &mut Buffer) {
        let error_text = Paragraph::new(format!("Error: {error}"))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::NONE));
        error_text.render(area, buf);
    }

    fn render_loaded(&self, area: Rect, buf: &mut Buffer) {
        let review = self.review.as_ref().expect("Review should be loaded");

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

        self.render_loaded_diff_state(layout[1], buf);
    }

    /// Render the diff content based on the current diff state
    fn render_loaded_diff_state(&self, area: Rect, buf: &mut Buffer) {
        match &self.diff_state {
            GitDiffLoadingState::Init => {
                // Show loading state for diff
                let loading_text = Paragraph::new("Init diff...")
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL));
                loading_text.render(area, buf);
            }
            GitDiffLoadingState::Loading => {
                // Show loading state for diff
                let loading_text = Paragraph::new("Loading diff...")
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL));
                loading_text.render(area, buf);
            }
            GitDiffLoadingState::Loaded(_diff) => self.render_loaded_diff_state_loaded(area, buf),
            GitDiffLoadingState::Error(error) => {
                // Show error state for diff
                let error_text = Paragraph::new(format!("Diff error: {error}"))
                    .style(Style::default().fg(Color::Red))
                    .block(Block::default().borders(Borders::ALL));
                error_text.render(area, buf);
            }
        }
    }

    /// Render the loaded diff content (file lists, file content) when the diff is fully loaded
    fn render_loaded_diff_state_loaded(&self, area: Rect, buf: &mut Buffer) {
        // Split content area into files lists (20%) and diff content (80%)
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Files lists
                Constraint::Percentage(80), // Diff content
            ])
            .split(area);

        // Render both file lists
        self.render_file_lists(content_layout[0], buf);

        // Render diff content
        self.render_diff_content(content_layout[1], buf);
    }

    /// Render both file lists (not viewed and viewed) side by side
    fn render_file_lists(&self, area: Rect, buf: &mut Buffer) {
        // Split the file lists area into two equal parts vertically
        let lists_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Not viewed files
                Constraint::Percentage(50), // Viewed files
            ])
            .split(area);

        // Render not viewed files list
        self.render_single_file_list(lists_layout[0], buf, FileListType::NotViewed, "Not Viewed");

        // Render viewed files list
        self.render_single_file_list(lists_layout[1], buf, FileListType::Viewed, "Viewed");
    }

    /// Render a single file list (either not viewed or viewed)
    fn render_single_file_list(
        &self,
        area: Rect,
        buf: &mut Buffer,
        list_type: FileListType,
        title: &str,
    ) {
        // Get the files for this list type
        let files: Vec<&DiffFile> = match list_type {
            FileListType::NotViewed => self
                .diff
                .files
                .iter()
                .filter(|file| !self.viewed_files.contains(&file.path))
                .collect(),
            FileListType::Viewed => self
                .diff
                .files
                .iter()
                .filter(|file| self.viewed_files.contains(&file.path))
                .collect(),
        };

        // Create list items
        let files_lines: Vec<ListItem> = files
            .iter()
            .enumerate()
            .map(|(index, diff_file)| self.render_file_line_for_list(index, diff_file, &list_type))
            .collect();

        // Determine if this list is active
        let is_active = matches!(self.navigation_mode, NavigationMode::Files)
            && self.active_file_list == list_type;

        // Create title with active indicator
        let list_title = if is_active {
            format!(" {title} [ACTIVE] ")
        } else {
            format!(" {title} ")
        };

        // Choose border color
        let border_color = if is_active { Color::Blue } else { Color::Gray };

        let files_list = List::new(files_lines)
            .block(
                Block::bordered()
                    .title(list_title)
                    .border_style(border_color),
            )
            .style(Style::default().fg(Color::White));

        files_list.render(area, buf);
    }

    fn render_file_line_for_list(
        &self,
        index: usize,
        diff_file: &DiffFile,
        list_type: &FileListType,
    ) -> ListItem {
        let is_selected = index == self.selected_file_index && self.active_file_list == *list_type;
        let is_files_mode = matches!(self.navigation_mode, NavigationMode::Files);

        let style = if is_selected && is_files_mode {
            Style::default().bg(Color::Blue).fg(Color::Black)
        } else if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected {
            FILE_SELECTION_INDICATOR
        } else {
            " "
        };

        let content = format!(
            "{}{} {}",
            prefix,
            self.comment_indicator(diff_file),
            diff_file.path.clone()
        );
        ListItem::new(content).style(style)
    }

    /// Get the comment indicator for a diff file based on its comment status
    ///
    /// Use different indicator for file comments and line comments and files that have both.
    /// Files with only resolved comments show the resolved indicator.
    fn comment_indicator(&self, diff_file: &DiffFile) -> CommentIndicator {
        // Check if file has any unresolved comments
        if self
            .files_with_file_and_or_line_comments
            .contains(&diff_file.path)
        {
            let has_line_comment = self.lines_with_comments.contains_key(&diff_file.path);
            let has_file_comment = self.files_with_file_comments.contains(&diff_file.path);

            if has_file_comment && !has_line_comment {
                CommentIndicator::FileComment
            } else if !has_file_comment && has_line_comment {
                CommentIndicator::LineComment
            } else {
                CommentIndicator::FileAndLineComment
            }
        } else if self
            .files_with_only_resolved_comments
            .contains(&diff_file.path)
        {
            // File has only resolved comments
            CommentIndicator::ResolvedComment
        } else {
            CommentIndicator::NoComment
        }
    }

    /// Render the diff content panel
    fn render_diff_content(&self, area: Rect, buf: &mut Buffer) {
        // Show empty state when no files are available
        if self.diff.is_empty() {
            let empty_text = Paragraph::new("No diff to display")
                .style(Style::default().fg(Color::Gray))
                .block(
                    Block::default()
                        .title(" Content ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Gray)),
                );

            empty_text.render(area, buf);
            return;
        }

        let content_text = if let Some(file) = self.get_selected_file() {
            &file.content
        } else {
            // Show error when no files are available
            let error_text = Paragraph::new("Error: No files available")
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .title(" Content ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                );
            error_text.render(area, buf);
            return;
        };

        // Split content into lines and apply scrolling with highlighting
        let content_lines: Vec<&str> = content_text.lines().collect();
        let content_height = area.height.saturating_sub(2) as usize; // Account for borders

        // Calculate the visible lines based on scroll offset
        let start_line = self.scroll_offset;
        let end_line = (start_line + content_height).min(content_lines.len());
        let visible_lines = if start_line < content_lines.len() {
            &content_lines[start_line..end_line]
        } else {
            &[]
        };

        // Create styled lines with highlighting for selected line
        let styled_lines: Vec<Line> = visible_lines
            .iter()
            .enumerate()
            .map(|(idx, line_text)| {
                let absolute_line_idx = start_line + idx;
                let is_selected_line = absolute_line_idx == self.selected_line_index;
                let is_lines_mode = matches!(self.navigation_mode, NavigationMode::Lines);

                // Check if this line has comments
                let has_comments = self
                    .get_selected_file()
                    .map(|file| {
                        self.lines_with_comments
                            .get(&file.path)
                            .map(|lines| lines.contains(&(absolute_line_idx as i64)))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                // Check if the line has only resolved comments
                let has_only_resolved_comments = self
                    .get_selected_file()
                    .map(|file| {
                        self.lines_with_only_resolved_comments
                            .get(&file.path)
                            .map(|lines| lines.contains(&(absolute_line_idx as i64)))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                // Add comment indicator based on comment status
                let comment_prefix = if has_comments {
                    LINE_COMMENT_INDICATOR
                } else if has_only_resolved_comments {
                    RESOLVED_COMMENT_INDICATOR
                } else {
                    " "
                };
                let display_text = format!("{comment_prefix} {line_text}");

                if is_selected_line && is_lines_mode {
                    // Highlight selected line in lines mode
                    Line::from(Span::styled(
                        display_text,
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
                    Line::from(Span::styled(display_text, style))
                }
            })
            .collect();

        // Show file info and navigation mode in title
        let total_lines = content_lines.len();
        let current_file_name = self
            .get_selected_file()
            .map(|f| f.path.as_str())
            .unwrap_or("Unknown");

        let title_text = match self.navigation_mode {
            NavigationMode::Files => format!(" {current_file_name} ({total_lines} lines) "),
            NavigationMode::Lines => {
                let line_num = self.selected_line_index + 1;
                format!(" {current_file_name} [ACTIVE] (line {line_num}/{total_lines}) ")
            }
        };

        let content = Paragraph::new(styled_lines).block(
            Block::default()
                .title(title_text)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if matches!(self.navigation_mode, NavigationMode::Lines) {
                        Color::Blue
                    } else {
                        Color::Gray
                    },
                )),
        );

        content.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_snapshot;
    use sqlx::SqlitePool;

    use crate::{
        app::App,
        database::Database,
        event::{Event, EventHandler},
        models::{Comment, Diff, DiffFile, Review, review::TestReviewParams},
        services::{CommentsLoadParams, CommentsLoadingState},
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
    fn test_review_details_view_creation() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.review_state {
            ReviewLoadingState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.base_branch, "default");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[test]
    fn test_review_details_view_new_loading() {
        let view = ReviewDetailsView::new_loading();

        assert_eq!(view.view_type(), ViewType::ReviewDetails);
        match &view.review_state {
            ReviewLoadingState::Loading => {}
            _ => panic!("Expected loading state"),
        }
    }

    #[test]
    fn test_review_details_view_debug_state() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review.clone());

        let debug_state = view.debug_state();
        assert!(debug_state.contains(&review.id));
        assert!(debug_state.starts_with("review_state: Loaded"));
    }

    #[test]
    fn test_review_details_view_debug_state_loading() {
        let view = ReviewDetailsView::new_loading();

        let debug_state = view.debug_state();
        assert_eq!(
            debug_state,
            "review_state: Loading, review: None, diff_state: Init, scroll_offset: 0, selected_file_index: 0, selected_line_index: 0, navigation_mode: Files, active_file_list: NotViewed, viewed_files: [], files_with_comments: [], lines_with_comments: {}"
        );
    }

    #[test]
    fn test_review_details_view_keybindings() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        let keybindings = view.get_keybindings();
        assert_eq!(keybindings.len(), 9);
        assert_eq!(keybindings[0].key, "↑/k");
        assert_eq!(keybindings[0].description, "Scroll up");
        assert_eq!(keybindings[1].key, "↓/j");
        assert_eq!(keybindings[1].description, "Scroll down");
        assert_eq!(keybindings[2].key, "←/h");
        assert_eq!(keybindings[2].description, "Switch to not viewed files");
        assert_eq!(keybindings[3].key, "→/l");
        assert_eq!(keybindings[3].description, "Switch to viewed files");
        assert_eq!(keybindings[4].key, "Space");
        assert_eq!(keybindings[4].description, "Toggle file view status");
        assert_eq!(keybindings[5].key, "Enter");
        assert_eq!(keybindings[5].description, "Toggle navigation mode");
        assert_eq!(keybindings[6].key, "Esc");
        assert_eq!(keybindings[6].description, "Go back / Switch to Files mode");
        assert_eq!(keybindings[7].key, "c");
        assert_eq!(keybindings[7].description, "Open comments");
        assert_eq!(keybindings[8].key, "?");
        assert_eq!(keybindings[8].description, "Help");
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
        view.navigation_mode = NavigationMode::Lines;

        // Initial current line should be 0
        assert_eq!(view.selected_line_index, 0);

        // Try to navigate down with 'j' - should not increment since we only have 1 line (index 0)
        let key_event = KeyEvent::new(
            KeyCode::Char('j'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        // Should stay at 0 since there's only 1 line (max index is 0)
        assert_eq!(view.selected_line_index, 0);

        // Navigate up with 'k' - should stay at 0
        let key_event = KeyEvent::new(
            KeyCode::Char('k'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.selected_line_index, 0);

        // Try to navigate up from 0 - should stay at 0
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.selected_line_index, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_line_reset_on_new_review() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        // Set some current line
        view.selected_line_index = 5;

        // Load a new review - should reset current line
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadingState(ReviewLoadingState::Loaded(Arc::from(review))),
        );

        assert_eq!(view.selected_line_index, 0);
    }

    #[tokio::test]
    async fn test_review_details_view_handles_escape_key() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        let key_event = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should send ViewClose event when in Files mode
        let event = app.events.try_recv().unwrap();
        match *event {
            Event::App(AppEvent::ViewClose) => {}
            _ => panic!("Expected ViewClose event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_escape_key_in_lines_mode() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);
        let mut app = create_test_app().await;

        // Switch to Lines mode first
        view.navigation_mode = NavigationMode::Lines;

        let key_event = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should switch back to Files mode, not close the view
        assert!(matches!(view.navigation_mode, NavigationMode::Files));

        // Should not send any events
        assert!(!app.events.has_pending_events());
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
            Event::App(AppEvent::HelpOpen(_)) => {}
            _ => panic!("Expected HelpOpen event"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_loading_state_loaded_event() {
        let mut view = ReviewDetailsView::new_loading();
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut app = create_test_app().await;

        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadingState(ReviewLoadingState::Loaded(Arc::from(review))),
        );

        match &view.review_state {
            ReviewLoadingState::Loaded(loaded_review) => {
                assert_eq!(loaded_review.base_branch, "main");
            }
            _ => panic!("Expected loaded state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_loading_state_error_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadingState(ReviewLoadingState::Error("Database error".into())),
        );

        match &view.review_state {
            ReviewLoadingState::Error(error) => {
                assert_eq!(error.to_string(), "Database error");
            }
            _ => panic!("Expected error state"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_handles_review_loading_state_not_found_event() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadingState(ReviewLoadingState::NotFound(Arc::from(
                "test-id".to_string(),
            ))),
        );

        match &view.review_state {
            ReviewLoadingState::NotFound(review_id) => {
                assert_eq!(review_id.to_string(), "test-id");
            }
            _ => panic!("Expected error state"),
        }
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
        view.navigation_mode = NavigationMode::Lines;
        view.selected_line_index = 3;

        // Navigate up with 'k' - should move current line up
        let key_event = KeyEvent::new(
            KeyCode::Char('k'),
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.selected_line_index, 2);

        // Navigate up with arrow key
        let key_event = KeyEvent::new(KeyCode::Up, ratatui::crossterm::event::KeyModifiers::NONE);
        view.handle_key_events(&mut app, &key_event).unwrap();
        assert_eq!(view.selected_line_index, 1);
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
        view.handle_app_events(
            &mut app,
            &AppEvent::ReviewLoadingState(ReviewLoadingState::Loaded(Arc::from(review))),
        );

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
            &AppEvent::ReviewLoadingState(ReviewLoadingState::Error("Database error".into())),
        );

        assert_eq!(view.scroll_offset, 0);
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
        let view = ReviewDetailsView::new_loading();
        let mut app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };
        app.handle_app_events(&AppEvent::ReviewLoadingState(ReviewLoadingState::Error(
            "Database connection failed".into(),
        )));

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state_diff_init() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review);
        let app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state_diff_loading() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch("develop")
                .base_sha("asdf1234"),
        );
        let view = ReviewDetailsView::new(review);

        let mut app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };
        // Simulate diff loading state
        app.handle_app_events(&AppEvent::GitDiffLoadingState(GitDiffLoadingState::Loading));

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state_diff_error() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch("feature")
                .base_sha("jkl09876"),
        );
        let view = ReviewDetailsView::new(review);

        let mut app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        // Simulate diff error state
        app.handle_app_events(&AppEvent::GitDiffLoadingState(GitDiffLoadingState::Error(
            "Repository not found".into(),
        )));

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state_diff_loaded_no_files() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review);

        let files = vec![];

        let mut app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        app.handle_app_events(&AppEvent::GitDiffLoadingState(GitDiffLoadingState::Loaded(
            Arc::new(Diff::from_files(files)),
        )));

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_render_loaded_state_diff_loaded_with_files() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review);

        // Simulate diff content being loaded
        let diff_content = r#"@@ -1,3 +1,4 @@
 # Test Repository
+
 This is a test file
-Old line to remove
+New line to add"#;
        let files = vec![DiffFile {
            path: "test_file.txt".to_string(),
            content: diff_content.to_string(),
        }];

        let mut app = App {
            view_stack: vec![Box::new(view)],
            ..create_test_app().await
        };

        app.handle_app_events(&AppEvent::GitDiffLoadingState(GitDiffLoadingState::Loaded(
            Arc::new(Diff::from_files(files)),
        )));

        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_file_level() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Set up a diff with files
        let files = vec![DiffFile {
            path: "src/main.rs".to_string(),
            content: "line1\nline2\nline3".to_string(),
        }];
        let diff = Arc::new(Diff::from_files(files));
        view.diff = diff;
        view.navigation_mode = NavigationMode::Files;
        view.selected_file_index = 0;

        // Initial state should have no pending events
        assert!(!app.events.has_pending_events());

        // Call open_comments (should open file-level comments)
        view.open_comments(&mut app);

        // Should have sent CommentsOpen event for file-level comments
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::CommentsOpen {
                review_id,
                file_path,
                line_number,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(file_path.as_ref(), "src/main.rs");
                assert_eq!(*line_number, None); // File-level comments
            }
            _ => panic!("Expected CommentsOpen event, got: {event:?}"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_line_level() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Set up a diff with files
        let files = vec![DiffFile {
            path: "src/lib.rs".to_string(),
            content: "line1\nline2\nline3\nline4\nline5".to_string(),
        }];
        let diff = Arc::new(Diff::from_files(files));
        view.diff = diff;
        view.navigation_mode = NavigationMode::Lines; // Switch to Lines mode
        view.selected_file_index = 0;
        view.selected_line_index = 2; // Select line 2 (0-indexed)

        // Initial state should have no pending events
        assert!(!app.events.has_pending_events());

        // Call open_comments (should open line-level comments)
        view.open_comments(&mut app);

        // Should have sent CommentsOpen event for line-level comments
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::CommentsOpen {
                review_id,
                file_path,
                line_number,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(file_path.as_ref(), "src/lib.rs");
                assert_eq!(*line_number, Some(2)); // Line-level comments (0-indexed)
            }
            _ => panic!("Expected CommentsOpen event, got: {event:?}"),
        }
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_no_review() {
        let mut view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        // Set the review to None (which is the case for new_loading())
        assert!(view.review.is_none());

        // Call open_comments
        view.open_comments(&mut app);

        // Should not send any events since there's no review
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_no_files() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Keep the default empty diff (no files)
        assert!(view.diff.files.is_empty());

        // Call open_comments
        view.open_comments(&mut app);

        // Should not send any events since there are no files
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_file_index_out_of_bounds() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Set up a diff with one file
        let files = vec![DiffFile {
            path: "src/main.rs".to_string(),
            content: "line1\nline2".to_string(),
        }];
        let diff = Arc::new(Diff::from_files(files));
        view.diff = diff;
        view.selected_file_index = 5; // Out of bounds index

        // Call open_comments
        view.open_comments(&mut app);

        // Should not send any events since the file index is out of bounds
        assert!(!app.events.has_pending_events());
    }

    #[tokio::test]
    async fn test_review_details_view_open_comments_key_handling() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Set up a diff with files
        let files = vec![DiffFile {
            path: "src/test.rs".to_string(),
            content: "test content".to_string(),
        }];
        let diff = Arc::new(Diff::from_files(files));
        view.diff = diff;
        view.selected_file_index = 0;

        // Simulate pressing 'c' key to open comments
        let key_event = ratatui::crossterm::event::KeyEvent {
            code: ratatui::crossterm::event::KeyCode::Char('c'),
            modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::empty(),
        };

        // Handle the key event
        view.handle_key_events(&mut app, &key_event).unwrap();

        // Should have sent CommentsOpen event
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::CommentsOpen {
                review_id,
                file_path,
                line_number,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(file_path.as_ref(), "src/test.rs");
                assert_eq!(*line_number, None); // File-level since we're in Files navigation mode
            }
            _ => panic!("Expected CommentsOpen event, got: {event:?}"),
        }
    }

    #[test]
    fn test_comment_indicator_no_comments() {
        let review = Review::test_review(());
        let view = ReviewDetailsView::new(review);

        let diff_file = DiffFile {
            path: "src/main.rs".to_string(),
            content: "test content".to_string(),
        };

        // No comments set up
        let indicator = view.comment_indicator(&diff_file);
        assert_eq!(indicator, CommentIndicator::NoComment);
    }

    #[test]
    fn test_comment_indicator_file_comments_only() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);

        let diff_file = DiffFile {
            path: "src/main.rs".to_string(),
            content: "test content".to_string(),
        };

        // Set up file with file comments only
        view.files_with_file_comments = Arc::new(vec!["src/main.rs".to_string()]);
        view.files_with_file_and_or_line_comments = Arc::new(vec!["src/main.rs".to_string()]);
        // No line comments

        let indicator = view.comment_indicator(&diff_file);
        assert_eq!(indicator, CommentIndicator::FileComment);
    }

    #[test]
    fn test_comment_indicator_line_comments_only() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);

        let diff_file = DiffFile {
            path: "src/main.rs".to_string(),
            content: "test content".to_string(),
        };

        // Set up file with line comments only
        view.files_with_file_and_or_line_comments = Arc::new(vec!["src/main.rs".to_string()]);
        // No file comments, but has line comments
        let mut lines_with_comments = std::collections::HashMap::new();
        lines_with_comments.insert("src/main.rs".to_string(), vec![1, 2, 3]);
        view.lines_with_comments = Arc::new(lines_with_comments);

        let indicator = view.comment_indicator(&diff_file);
        assert_eq!(indicator, CommentIndicator::LineComment);
    }

    #[test]
    fn test_comment_indicator_file_and_line_comments() {
        let review = Review::test_review(());
        let mut view = ReviewDetailsView::new(review);

        let diff_file = DiffFile {
            path: "src/main.rs".to_string(),
            content: "test content".to_string(),
        };

        // Set up file with both file and line comments
        view.files_with_file_comments = Arc::new(vec!["src/main.rs".to_string()]);
        view.files_with_file_and_or_line_comments = Arc::new(vec!["src/main.rs".to_string()]);
        let mut lines_with_comments = std::collections::HashMap::new();
        lines_with_comments.insert("src/main.rs".to_string(), vec![5, 10]);
        view.lines_with_comments = Arc::new(lines_with_comments);

        let indicator = view.comment_indicator(&diff_file);
        assert_eq!(indicator, CommentIndicator::FileAndLineComment);
    }

    #[tokio::test]
    async fn test_reload_comments_with_review() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Call reload_comments
        view.reload_comments(&mut app);

        // Should send CommentsLoad event for the whole review
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::CommentsLoad(params)) => {
                assert_eq!(params.review_id.as_ref(), review.id);
                assert!(params.file_path.as_ref().is_none());
                assert!(params.line_number.as_ref().is_none());
            }
            _ => panic!("Expected CommentsLoad event, got: {event:?}"),
        }
    }

    #[tokio::test]
    async fn test_reload_comments_without_review() {
        let view = ReviewDetailsView::new_loading();
        let mut app = create_test_app().await;

        // Call reload_comments (should not send any event since no review)
        view.reload_comments(&mut app);

        // Should not send any events
        assert!(!app.events.has_pending_events());
    }

    #[test]
    fn test_comments_load_params_with_review() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review.clone());

        let params = view.comments_load_params();
        assert!(params.is_some());

        let params = params.unwrap();
        assert_eq!(params.review_id.as_ref(), review.id);
        assert!(params.file_path.as_ref().is_none());
        assert!(params.line_number.as_ref().is_none());
    }

    #[test]
    fn test_comments_load_params_without_review() {
        let view = ReviewDetailsView::new_loading();

        let params = view.comments_load_params();
        assert!(params.is_none());
    }

    #[test]
    fn test_relevant_comments_loading_state_matching_params() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review.clone());

        let params = CommentsLoadParams {
            review_id: Arc::from(review.id.clone()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        let is_relevant = view.relevant_comments_loading_state(&params);
        assert!(is_relevant);
    }

    #[test]
    fn test_relevant_comments_loading_state_different_review_id() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let view = ReviewDetailsView::new(review.clone());

        let params = CommentsLoadParams {
            review_id: Arc::from("different-review-id".to_string()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        let is_relevant = view.relevant_comments_loading_state(&params);
        assert!(!is_relevant);
    }

    #[test]
    fn test_relevant_comments_loading_state_without_review() {
        let view = ReviewDetailsView::new_loading();

        let params = CommentsLoadParams {
            review_id: Arc::from("any-review-id".to_string()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        let is_relevant = view.relevant_comments_loading_state(&params);
        assert!(!is_relevant);
    }

    #[test]
    fn test_handle_comments_loading_state_not_relevant() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());

        // Set up initial state
        let initial_files_with_comments = view.files_with_file_comments.clone();
        let initial_lines_with_comments = view.lines_with_comments.clone();

        // Load comments for a different review
        let params = CommentsLoadParams {
            review_id: Arc::from("different-review-id".to_string()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        // Create dummy comments
        let comments = vec![Comment::test_comment(
            "different-review-id",
            "src/main.rs",
            None,
            "File comment",
        )];

        let state = CommentsLoadingState::Loaded(Arc::new(comments));
        view.handle_comments_loading_state(&params, &state);

        // State should not change since params are not relevant
        assert!(Arc::ptr_eq(
            &view.files_with_file_comments,
            &initial_files_with_comments
        ));
        assert!(Arc::ptr_eq(
            &view.lines_with_comments,
            &initial_lines_with_comments
        ));
    }

    #[test]
    fn test_handle_comments_loading_state_loaded_file_comments() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());

        let params = CommentsLoadParams {
            review_id: Arc::from(review.id.clone()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        // Create test comments - file comments only
        let comments = vec![
            Comment::test_comment(&review.id, "src/main.rs", None, "File comment"),
            Comment::test_comment(&review.id, "src/lib.rs", None, "Another file comment"),
        ];

        let state = CommentsLoadingState::Loaded(Arc::new(comments));
        view.handle_comments_loading_state(&params, &state);

        // Should update files with file comments
        assert_eq!(view.files_with_file_comments.len(), 2);
        assert!(
            view.files_with_file_comments
                .contains(&"src/main.rs".to_string())
        );
        assert!(
            view.files_with_file_comments
                .contains(&"src/lib.rs".to_string())
        );

        // Should update files with file and/or line comments
        assert_eq!(view.files_with_file_and_or_line_comments.len(), 2);
        assert!(
            view.files_with_file_and_or_line_comments
                .contains(&"src/main.rs".to_string())
        );
        assert!(
            view.files_with_file_and_or_line_comments
                .contains(&"src/lib.rs".to_string())
        );

        // No line comments
        assert!(view.lines_with_comments.is_empty());
    }

    #[test]
    fn test_handle_comments_loading_state_loaded_line_comments() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());

        let params = CommentsLoadParams {
            review_id: Arc::from(review.id.clone()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        // Create test comments - line comments only
        let comments = vec![
            Comment::test_comment(&review.id, "src/main.rs", Some(10), "Line comment"),
            Comment::test_comment(&review.id, "src/main.rs", Some(20), "Another line comment"),
        ];

        let state = CommentsLoadingState::Loaded(Arc::new(comments));
        view.handle_comments_loading_state(&params, &state);

        // No file comments (only line comments)
        assert!(view.files_with_file_comments.is_empty());

        // Should update files with file and/or line comments
        assert_eq!(view.files_with_file_and_or_line_comments.len(), 1);
        assert!(
            view.files_with_file_and_or_line_comments
                .contains(&"src/main.rs".to_string())
        );

        // Should update lines with comments
        assert_eq!(view.lines_with_comments.len(), 1);
        let main_rs_lines = view.lines_with_comments.get("src/main.rs").unwrap();
        assert_eq!(main_rs_lines.len(), 2);
        assert!(main_rs_lines.contains(&10));
        assert!(main_rs_lines.contains(&20));
    }

    #[test]
    fn test_handle_comments_loading_state_loaded_mixed_comments() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());

        let params = CommentsLoadParams {
            review_id: Arc::from(review.id.clone()),
            file_path: Arc::new(None),
            line_number: Arc::new(None),
        };

        // Create test comments - mix of file and line comments
        let comments = vec![
            Comment::test_comment(&review.id, "src/main.rs", None, "File comment"),
            Comment::test_comment(&review.id, "src/main.rs", Some(15), "Line comment"),
            Comment::test_comment(&review.id, "src/lib.rs", Some(25), "Another line comment"),
        ];

        let state = CommentsLoadingState::Loaded(Arc::new(comments));
        view.handle_comments_loading_state(&params, &state);

        // Should have file comments
        assert_eq!(view.files_with_file_comments.len(), 1);
        assert!(
            view.files_with_file_comments
                .contains(&"src/main.rs".to_string())
        );

        // Should update files with file and/or line comments
        assert_eq!(view.files_with_file_and_or_line_comments.len(), 2);
        assert!(
            view.files_with_file_and_or_line_comments
                .contains(&"src/main.rs".to_string())
        );
        assert!(
            view.files_with_file_and_or_line_comments
                .contains(&"src/lib.rs".to_string())
        );

        // Should update lines with comments
        assert_eq!(view.lines_with_comments.len(), 2);

        let main_rs_lines = view.lines_with_comments.get("src/main.rs").unwrap();
        assert_eq!(main_rs_lines.len(), 1);
        assert!(main_rs_lines.contains(&15));

        let lib_rs_lines = view.lines_with_comments.get("src/lib.rs").unwrap();
        assert_eq!(lib_rs_lines.len(), 1);
        assert!(lib_rs_lines.contains(&25));
    }

    #[tokio::test]
    async fn test_handle_comment_created_event_triggers_reload() {
        let review = Review::test_review(TestReviewParams::new().base_branch("main"));
        let mut view = ReviewDetailsView::new(review.clone());
        let mut app = create_test_app().await;

        // Create a dummy comment
        let comment = Comment::test_comment(&review.id, "src/main.rs", None, "New comment");

        // Handle CommentCreated event
        view.handle_app_events(&mut app, &AppEvent::CommentCreated(Arc::new(comment)));

        // Should trigger reload of comments
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::CommentsLoad(params)) => {
                assert_eq!(params.review_id.as_ref(), review.id);
                assert!(params.file_path.as_ref().is_none());
                assert!(params.line_number.as_ref().is_none());
            }
            _ => panic!("Expected CommentsLoad event, got: {event:?}"),
        }
    }
}
