use crate::app::App;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Clear, Paragraph, Widget},
};

#[derive(Default)]
pub struct ReviewCreateView {
    pub title_input: String,
}

impl ReviewCreateView {
    pub fn render(&self, _app: &App, area: Rect, buf: &mut Buffer) {
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
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);

        let title_label = Paragraph::new("Title:");
        title_label.render(chunks[0], buf);

        let title_input = Paragraph::new(self.title_input.as_str())
            .block(Block::bordered())
            .style(Style::default().fg(Color::Yellow));
        title_input.render(chunks[1], buf);

        let help = Paragraph::new("Press Enter to create, Esc to cancel")
            .style(Style::default().fg(Color::Gray));
        help.render(chunks[2], buf);
    }
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
