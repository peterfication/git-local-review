use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::{
    app::App,
    views::{View, ViewHandler, main::MainView, review_create::ReviewCreateView},
};

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let current_view = self.current_view();

        match current_view {
            View::Main => {
                MainView.render(self, area, buf);
            }
            View::ReviewCreate => {
                MainView.render(self, area, buf);
                ReviewCreateView.render(self, area, buf);
            }
        }
    }
}
