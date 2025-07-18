use ratatui::{buffer::Buffer, crossterm::event::KeyEvent, layout::Rect};

use crate::{app::App, event::AppEvent};

pub mod confirmation_dialog;
pub mod main_view;
pub mod review_create_view;

pub use confirmation_dialog::ConfirmationDialogView;
pub use main_view::MainView;
pub use review_create_view::ReviewCreateView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Main,
    ReviewCreate,
    ConfirmationDialog,
}

pub trait ViewHandler {
    fn view_type(&self) -> ViewType;
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer);
    fn handle_key_events(&mut self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()>;
    /// Handle app events that this view is interested in
    fn handle_app_events(&mut self, app: &mut App, event: &AppEvent) {
        // Default implementation does nothing
        let _ = (app, event);
    }

    /// Get a debug representation of the view's state for testing purposes.
    /// This is only available in test builds.
    #[cfg(test)]
    fn debug_state(&self) -> String {
        format!("{:?}", self.view_type())
    }

    /// Downcast to Any for type-specific operations (only used for testing)
    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Downcast to Any for type-specific operations (only used for testing)
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any;
}
