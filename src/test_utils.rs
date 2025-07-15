#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use ratatui::{Terminal, backend::TestBackend};

#[cfg(test)]
/// Usage:
/// assert_snapshot!(render_app_to_terminal_backend(app))
pub fn render_app_to_terminal_backend(app: App) -> TestBackend {
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal
        .draw(|buffer| buffer.render_widget(&app, buffer.area()))
        .unwrap();
    terminal.backend().clone()
}

#[cfg(test)]
/// Usage:
/// assert_snapshot!(render_view_to_terminal_backend(&app, |app, area, buf| {
///     view.render(app, area, buf);
/// }));
pub fn render_view_to_terminal_backend<F>(app: &App, render_fn: F) -> TestBackend
where
    F: FnOnce(&App, ratatui::layout::Rect, &mut ratatui::buffer::Buffer),
{
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal
        .draw(|frame| {
            render_fn(app, frame.area(), frame.buffer_mut());
        })
        .unwrap();
    terminal.backend().clone()
}

#[cfg(test)]
pub fn fixed_time() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}
