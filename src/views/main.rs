use crate::{
    app::App,
    event::AppEvent,
    services::review_service::ReviewsLoadingState,
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
            ReviewsLoadingState::Init => self.render_reviews_init(),
            ReviewsLoadingState::Loading => self.render_reviews_loading(),
            ReviewsLoadingState::Loaded => self.render_reviews_loaded(&app.reviews),
            ReviewsLoadingState::Error(error) => self.render_reviews_error(error),
        };

        let reviews_list = List::new(reviews)
            .block(Block::bordered().title("Reviews"))
            .style(Style::default().fg(Color::White));

        reviews_list.render(chunks[1], buf);
    }
}

impl MainView {
    fn render_reviews_init(&self) -> Vec<ListItem> {
        vec![ListItem::new("Initializing...").style(Style::default().fg(Color::Gray))]
    }

    fn render_reviews_loading(&self) -> Vec<ListItem> {
        vec![ListItem::new("Loading reviews...").style(Style::default().fg(Color::Yellow))]
    }

    fn render_reviews_loaded(&self, reviews: &[crate::models::review::Review]) -> Vec<ListItem> {
        if reviews.is_empty() {
            vec![
                ListItem::new("No reviews found - Press 'n' to create a new review")
                    .style(Style::default().fg(Color::Yellow)),
            ]
        } else {
            reviews
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

    fn render_reviews_error(&self, error: &str) -> Vec<ListItem> {
        vec![
            ListItem::new(format!("Error loading reviews: {error}"))
                .style(Style::default().fg(Color::Red)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::event::{AppEvent, Event};
    use crate::models::review::Review;
    use crate::test_utils::{fixed_time, render_app_to_terminal_backend};
    use crate::time_provider::MockTimeProvider;
    use insta::assert_snapshot;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use sqlx::SqlitePool;

    async fn create_test_app_with_reviews() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();

        // Create some test reviews with fixed timestamps
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::hours(1);

        let time_provider1 = MockTimeProvider::new(time1);
        let time_provider2 = MockTimeProvider::new(time2);

        let review1 = Review::new_with_time_provider("Review 1".to_string(), &time_provider1);
        let review2 = Review::new_with_time_provider("Review 2".to_string(), &time_provider2);
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
            view_stack: vec![Box::new(MainView)],
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

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_init() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Init,
            ..create_test_app_with_reviews().await
        };
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loading() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Loading,
            ..create_test_app_with_reviews().await
        };
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_with_reviews() {
        let app = create_test_app_with_reviews().await;
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_loaded_no_reviews() {
        let app = App {
            reviews: vec![],
            reviews_loading_state: ReviewsLoadingState::Loaded,
            ..create_test_app_with_reviews().await
        };
        assert_snapshot!(render_app_to_terminal_backend(app))
    }

    #[tokio::test]
    async fn test_main_view_render_reviews_loading_state_error() {
        let app = App {
            reviews_loading_state: ReviewsLoadingState::Error("Test error".to_string()),
            ..create_test_app_with_reviews().await
        };
        assert_snapshot!(render_app_to_terminal_backend(app))
    }
}
