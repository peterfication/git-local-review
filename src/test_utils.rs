#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use ratatui::{Terminal, backend::TestBackend};

#[cfg(test)]
pub fn get_terminal_backend(app: App) -> TestBackend {
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal
        .draw(|buffer| buffer.render_widget(&app, buffer.area()))
        .unwrap();
    terminal.backend().clone()
}
