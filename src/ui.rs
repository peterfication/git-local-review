use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Clear, List, ListItem, Paragraph, Widget},
};

use crate::app::App;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let header = Paragraph::new("Git Local Review - Press 'n' to create new review, 'q' to quit")
            .block(Block::bordered().title("git-local-review"))
            .fg(Color::Cyan);
        header.render(chunks[0], buf);

        // Reviews list
        let reviews: Vec<ListItem> = self
            .reviews
            .iter()
            .map(|review| {
                ListItem::new(format!("{} ({})", review.title, review.created_at.format("%Y-%m-%d %H:%M")))
            })
            .collect();

        let reviews_list = List::new(reviews)
            .block(Block::bordered().title("Reviews"))
            .style(Style::default().fg(Color::White));

        reviews_list.render(chunks[1], buf);

        // Render popup if visible
        if self.review_create_popup_show {
            render_create_review_popup(self, area, buf);
        }
    }
}

fn render_create_review_popup(app: &App, area: Rect, buf: &mut Buffer) {
    let popup_area = centered_rect(60, 20, area);

    Clear.render(popup_area, buf);

    let block = Block::bordered()
        .title("Create New Review")
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Length(1)])
        .split(inner);

    let title_label = Paragraph::new("Title:");
    title_label.render(chunks[0], buf);

    let title_input = Paragraph::new(app.review_create_title_input.as_str())
        .block(Block::bordered())
        .style(Style::default().fg(Color::Yellow));
    title_input.render(chunks[1], buf);

    let help = Paragraph::new("Press Enter to create, Esc to cancel")
        .style(Style::default().fg(Color::Gray));
    help.render(chunks[2], buf);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
