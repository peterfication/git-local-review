use crate::{
    app::{App, ReviewsLoadingState},
    event::AppEvent,
    views::{ViewHandler, ViewType},
};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

pub struct MainView;

impl ViewHandler for MainView {
    fn view_type(&self) -> ViewType {
        ViewType::Main
    }

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

        let reviews: Vec<ListItem> = match &app.reviews_loading_state {
            ReviewsLoadingState::Init => {
                vec![ListItem::new("Initializing...").style(Style::default().fg(Color::Gray))]
            }
            ReviewsLoadingState::Loading => {
                vec![ListItem::new("Loading reviews...").style(Style::default().fg(Color::Yellow))]
            }
            ReviewsLoadingState::Loaded => {
                if app.reviews.is_empty() {
                    vec![
                        ListItem::new("No reviews found - Press 'n' to create a new review")
                            .style(Style::default().fg(Color::Yellow)),
                    ]
                } else {
                    app.reviews
                        .iter()
                        .map(|review| {
                            ListItem::new(format!(
                                "{} ({})",
                                review.title,
                                review.created_at.format("%Y-%m-%d %H:%M")
                            ))
                        })
                        .collect()
                }
            }
            ReviewsLoadingState::Error(error) => {
                vec![
                    ListItem::new(format!("Error loading reviews: {error}"))
                        .style(Style::default().fg(Color::Red)),
                ]
            }
        };

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
    use crate::event::{AppEvent, Event};
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
            events: crate::event::EventHandler::new_for_test(),
            database,
            reviews,
            reviews_loading_state: ReviewsLoadingState::Loaded,
            view_stack: vec![],
        }
    }

    #[tokio::test]
    async fn test_main_view_handle_quit_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);
        assert!(!app.events.has_pending_events());

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

        // Verify that a Quit event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_esc_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);

        // Verify that a Quit event was sent (Esc also triggers quit)
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_ctrl_c() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(app.running);
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        assert!(app.running);

        // Verify that a Quit event was sent (Ctrl+C also triggers quit)
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::Quit)));
    }

    #[tokio::test]
    async fn test_main_view_handle_create_review_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Verify that a ReviewCreateOpen event was sent
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        assert!(matches!(event, Event::App(AppEvent::ReviewCreateOpen)));
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_main_view_handle_unknown_key() {
        let mut app = create_test_app_with_reviews().await;
        let mut view = MainView;
        let initial_running = app.running;
        assert!(!app.events.has_pending_events());

        let key_event = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        view.handle_key_events(&mut app, key_event).unwrap();

        // Unknown keys should not change app state or send events
        assert_eq!(app.running, initial_running);
        assert!(!app.events.has_pending_events());
    }

    // FIXME: Is this method necessary? Or does ratatui offer a better way for this?
    fn buffer_to_string(buffer: &Buffer) -> String {
        let mut content = String::new();
        for y in 0..buffer.area().height {
            for x in 0..buffer.area().width {
                let position: ratatui::layout::Position = (x, y).into(); // Explicitly specify the type
                if let Some(cell) = buffer.cell(position) {
                    content.push(cell.symbol().chars().next().unwrap_or(' '));
                } else {
                    content.push(' '); // Fallback if the cell is None
                }
            }
            content.push('\n');
        }
        content
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_init() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Init,
            ..create_test_app_with_reviews().await
        };
        let view = MainView;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));

        view.render(&app, Rect::new(0, 0, 50, 10), &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Initializing..."));
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loading() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Loading,
            ..create_test_app_with_reviews().await
        };
        let view = MainView;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));

        view.render(&app, Rect::new(0, 0, 50, 10), &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Loading reviews..."));
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_with_reviews() {
        let app = create_test_app_with_reviews().await;
        let view = MainView;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));

        view.render(&app, Rect::new(0, 0, 50, 10), &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Review 1"));
        assert!(content.contains("Review 2"));
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_no_reviews() {
        let app = App {
            reviews: vec![],
            reviews_loading_state: ReviewsLoadingState::Loaded,
            ..create_test_app_with_reviews().await
        };
        let view = MainView;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));

        view.render(&app, Rect::new(0, 0, 50, 10), &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("No reviews found"));
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_error() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Error("Test error".to_string()),
            ..create_test_app_with_reviews().await
        };
        let view = MainView;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));

        view.render(&app, Rect::new(0, 0, 50, 10), &mut buffer);

        let content = buffer_to_string(&buffer);
        assert!(content.contains("Error loading reviews: Test error"));
    }
}
