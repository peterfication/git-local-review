use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

pub struct MainView;

impl MainView {
    pub fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let header =
            Paragraph::new("Git Local Review - Press 'n' to create a new review, 'q' to quit")
                .block(Block::bordered().title("git-local-review"))
                .fg(Color::Cyan);
        header.render(chunks[0], buf);

        let reviews: Vec<ListItem> = app
            .reviews
            .iter()
            .map(|review| {
                ListItem::new(format!(
                    "{} ({})",
                    review.title,
                    review.created_at.format("%Y-%m-%d %H:%M")
                ))
            })
            .collect();

        let reviews_list = List::new(reviews)
            .block(Block::bordered().title("Reviews"))
            .style(Style::default().fg(Color::White));

        reviews_list.render(chunks[1], buf);
    }
}
