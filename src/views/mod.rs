pub mod main;
pub mod review_create;

use crate::app::App;
use ratatui::{buffer::Buffer, crossterm::event::KeyEvent, layout::Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    Main,
    ReviewCreate,
}

pub trait ViewHandler {
    fn render(&self, app: &App, area: Rect, buf: &mut Buffer);
    fn handle_key_events(&mut self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()>;
    fn view_type(&self) -> ViewType;
}
