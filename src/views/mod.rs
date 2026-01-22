#[cfg(test)]
use std::any::Any;

use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyEvent,
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::{app::App, event::AppEvent};

pub use help_modal::KeyBinding;

pub mod comments_view;
pub mod confirmation_dialog;
pub mod help_modal;
pub mod main_view;
pub mod review_create_view;
pub mod review_details_view;
pub mod review_refresh_dialog;

pub use comments_view::CommentsView;
pub use confirmation_dialog::ConfirmationDialogView;
pub use help_modal::HelpModalView;
pub use main_view::MainView;
pub use review_create_view::ReviewCreateView;
pub use review_details_view::ReviewDetailsView;
pub use review_refresh_dialog::ReviewRefreshDialogView;

const SELECTION_INDICATOR: &str = ">";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Main,
    ReviewCreate,
    ConfirmationDialog,
    HelpModal,
    ReviewDetails,
    ReviewRefreshDialog,
    Comments,
}

pub trait ViewHandler {
    fn view_type(&self) -> ViewType;
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer);
    fn handle_key_events(&mut self, app: &mut App, key_event: &KeyEvent) -> color_eyre::Result<()>;
    /// Handle app events that this view is interested in
    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        // Default implementation does nothing
        let _ = (app, event);
    }
    /// Get the keybindings for this view to display in help modal
    fn get_keybindings(&self) -> Arc<[KeyBinding]>;

    /// Get a debug representation of the view's state for testing purposes.
    /// This is only available in test builds.
    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!("{:?}", self.view_type())
    }

    /// Downcast to Any for type-specific operations (only used for testing)
    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Downcast to Any for type-specific operations (only used for testing)
    #[cfg(test)]
    fn as_any(&self) -> &dyn Any;
}

/// Creates a centered rectangle with the given percentage dimensions within the provided area.
///
/// This is a utility function commonly used for modal dialogs and popups.
///
/// # Arguments
/// * `percent_x` - Width as a percentage of the parent area (0-100)
/// * `percent_y` - Height as a percentage of the parent area (0-100)
/// * `rectangle` - The parent rectangle to center within
///
/// # Returns
/// A `Rect` that is centered within the parent rectangle with the specified dimensions
pub fn centered_rectangle(percent_x: u16, percent_y: u16, rectangle: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rectangle);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect() {
        let parent = Rect::new(0, 0, 100, 100);
        let centered = centered_rectangle(50, 50, parent);

        // Should be centered in a 100x100 area
        assert_eq!(centered.x, 25); // (100 - 50) / 2 = 25
        assert_eq!(centered.y, 25); // (100 - 50) / 2 = 25
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
    }

    #[test]
    fn test_centered_rect_different_percentages() {
        let parent = Rect::new(10, 20, 80, 60);
        let centered = centered_rectangle(25, 50, parent);

        // Width: 25% of 80 = 20, so x offset = (80 - 20) / 2 = 30, plus parent x = 40
        // Height: 50% of 60 = 30, so y offset = (60 - 30) / 2 = 15, plus parent y = 35
        assert_eq!(centered.x, 40); // 10 + 30
        assert_eq!(centered.y, 35); // 20 + 15
        assert_eq!(centered.width, 20); // 25% of 80
        assert_eq!(centered.height, 30); // 50% of 60
    }
}
