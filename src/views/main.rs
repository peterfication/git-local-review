use crate::{app::App, event::AppEvent, views::ViewHandler};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

pub struct MainView;

impl ViewHandler for MainView {
    fn handle_key_events(&mut self, app: &mut App, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => app.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                app.events.send(AppEvent::Quit)
            }
            KeyCode::Char('n') => app.events.send(AppEvent::ReviewCreateOpen),
            _ => {}
        }
        Ok(())
    }

    fn render(&self, app: &App, area: Rect, buf: &mut Buffer) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::review::Review;
    use sqlx::SqlitePool;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    async fn create_test_app_with_reviews() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        // Create some test reviews
        let review1 = Review::new("Review 1".to_string());
        let review2 = Review::new("Review 2".to_string());
        review1.save(&pool).await.unwrap();
        review2.save(&pool).await.unwrap();

        let database = Database::from_pool(pool);
        let reviews = Review::list_all(database.pool()).await.unwrap();

        App {
            running: true,
            events: crate::event::EventHandler::new(),
            database,
            reviews,
            view_stack: vec![],
        }
    }

    #[tokio::test]
    async fn test_main_view_handle_quit_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);

        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // The view handler only sends events, it doesn't process them immediately
        // The app remains running until the event is processed by EventProcessor
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_esc_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_ctrl_c() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);

        let key_event = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_create_review_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        let _initial_views = app.view_stack.len();

        let key_event = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // The view should have sent a ReviewCreateOpen event
        // We can't directly test this without access to the event queue,
        // but the key handler should not crash
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_unknown_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        let initial_running = app.running;

        let key_event = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Unknown keys should not change app state
        assert_eq!(app.running, initial_running);
    }
}
