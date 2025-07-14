use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::{app::App, views::View};

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
                self.main_view.render(self, area, buf);
            }
            View::ReviewCreate => {
                self.main_view.render(self, area, buf);
                self.review_create_view.render(self, area, buf);
            }
        }
    }
}
