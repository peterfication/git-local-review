pub mod main;
pub mod review_create;

use crate::app::App;
use ratatui::{buffer::Buffer, crossterm::event::KeyEvent, layout::Rect};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum View {
    #[default]
    Main,
    ReviewCreate,
}

pub trait ViewHandler {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer);
    fn handle_key_events(&self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()>;
}
