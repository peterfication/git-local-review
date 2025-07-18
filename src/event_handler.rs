use crate::app::App;
use crate::event::{AppEvent, Event};
use crate::services::{ReviewService, ServiceHandler};
use crate::views::{confirmation_dialog::ConfirmationDialogView, review_create::ReviewCreateView};

pub struct EventProcessor;

impl EventProcessor {
    pub async fn process_event(app: &mut App, event: Event) -> color_eyre::Result<()> {
        match event {
            Event::Tick => app.tick(),
            #[allow(clippy::single_match)]
            Event::Crossterm(event) => match event {
                crossterm::event::Event::Key(key_event) => app.handle_key_events(key_event)?,
                _ => {}
            },
            Event::App(app_event) => {
                log::info!("Processing event: {app_event:#?}");

                // First let services handle the event
                Self::handle_services(app, &app_event).await?;

                // Then let views handle the event
                app.handle_app_events(&app_event);

                // Finally handle app events globally
                match app_event {
                    AppEvent::Quit => app.quit(),
                    AppEvent::ReviewCreateOpen => Self::review_create_open(app),
                    AppEvent::ReviewCreateClose => Self::review_create_close(app),
                    AppEvent::ReviewDeleteConfirm(review_id) => {
                        Self::review_delete_confirm(app, review_id)
                    }
                    AppEvent::ReviewDeleteCancel => Self::review_delete_cancel(app),
                    AppEvent::ReviewDelete(review_id) => {
                        Self::review_delete(app, review_id).await?
                    }
                    _ => {
                        // Other events are handled by services or views
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle app events through services
    async fn handle_services(app: &mut App, event: &AppEvent) -> color_eyre::Result<()> {
        let services = vec![ReviewService::handle_app_event];

        for handler in services {
            handler(event, &app.database, &mut app.events).await?;
        }
        Ok(())
    }

    /// Open the review creation view
    fn review_create_open(app: &mut App) {
        app.push_view(Box::new(ReviewCreateView::default()));
    }

    /// Close the review creation view
    fn review_create_close(app: &mut App) {
        app.pop_view();
    }

    /// Open delete confirmation dialog
    fn review_delete_confirm(app: &mut App, review_id: String) {
        // Create a generic confirmation dialog without the specific review title
        // since we don't have access to the reviews in the App anymore
        // TODO: Load the title from the review_service / database
        let message = "Do you want to delete the selected review?".to_string();
        let confirmation_dialog = ConfirmationDialogView::new(
            message,
            AppEvent::ReviewDelete(review_id),
            AppEvent::ReviewDeleteCancel,
        );
        app.push_view(Box::new(confirmation_dialog));
    }

    /// Cancel review deletion
    fn review_delete_cancel(app: &mut App) {
        app.pop_view();
    }

    /// Delete the selected review
    async fn review_delete(app: &mut App, review_id: String) -> color_eyre::Result<()> {
        let reviews = ReviewService::delete_review_by_id(&app.database, &review_id).await?;
        app.events.send(AppEvent::ReviewsLoaded(reviews));
        app.pop_view();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::review::Review;
    use crate::views::{ViewType, main::MainView};
    use sqlx::SqlitePool;

    async fn create_test_app() -> App {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::models::review::Review::create_table(&pool)
            .await
            .unwrap();

        let database = Database::from_pool(pool);

        App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![Box::new(MainView::new())],
        }
    }

    #[tokio::test]
    async fn test_process_quit_event() {
        let mut app = create_test_app().await;
        assert!(app.running);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::Quit))
            .await
            .unwrap();

        assert!(!app.running);
    }

    #[tokio::test]
    async fn test_process_review_create_open_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen))
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
    }

    #[tokio::test]
    async fn test_process_review_create_close_event() {
        let mut app = create_test_app().await;

        // First open a review create view
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen))
            .await
            .unwrap();
        assert_eq!(app.view_stack.len(), 2);

        // Then close it
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateClose))
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_tick_event() {
        let mut app = create_test_app().await;

        // Tick event should not change anything
        EventProcessor::process_event(&mut app, Event::Tick)
            .await
            .unwrap();

        assert!(app.running);
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_crossterm_key_event() {
        let mut app = create_test_app().await;

        let key_event = ratatui::crossterm::event::KeyEvent {
            code: ratatui::crossterm::event::KeyCode::Char('q'),
            modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::empty(),
        };

        let crossterm_event = ratatui::crossterm::event::Event::Key(key_event);

        EventProcessor::process_event(&mut app, Event::Crossterm(crossterm_event))
            .await
            .unwrap();

        // The key event should be handled by the view, which only sends events
        // The app should remain running until the event is processed
        assert!(app.running);
    }

    #[tokio::test]
    async fn test_process_review_delete_confirm_event() {
        let mut app = create_test_app().await;

        // Create a review
        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();
        let review_id = review.id.clone();

        assert_eq!(app.view_stack.len(), 1); // Only MainView

        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::ReviewDeleteConfirm(review_id)),
        )
        .await
        .unwrap();

        // Should have added a confirmation dialog view
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ConfirmationDialog
        );
    }

    #[tokio::test]
    async fn test_process_review_delete_confirm_event_no_selection() {
        let mut app = create_test_app().await;

        assert_eq!(app.view_stack.len(), 1); // Only MainView

        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::ReviewDeleteConfirm("non-existent-id".to_string())),
        )
        .await
        .unwrap();

        // Should have added a confirmation dialog view even for non-existent ID
        // The delete operation will handle non-existent reviews
        assert_eq!(app.view_stack.len(), 2);
    }

    #[tokio::test]
    async fn test_process_review_delete_cancel_event() {
        let mut app = create_test_app().await;

        // Simulate having a confirmation dialog open
        let confirmation_dialog = crate::views::confirmation_dialog::ConfirmationDialogView::new(
            "Test".to_string(),
            AppEvent::ReviewDelete("test-id".to_string()),
            AppEvent::ReviewDeleteCancel,
        );
        app.push_view(Box::new(confirmation_dialog));
        assert_eq!(app.view_stack.len(), 2);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewDeleteCancel))
            .await
            .unwrap();

        // Should have closed the confirmation dialog
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_review_delete_event() {
        let mut app = create_test_app().await;

        // Create two reviews
        let review1 = Review::new("Review 1".to_string());
        let review2 = Review::new("Review 2".to_string());
        review1.save(app.database.pool()).await.unwrap();
        review2.save(app.database.pool()).await.unwrap();

        // Load reviews (they will be ordered by created_at DESC)
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        let review_id_to_delete = reviews[0].id.clone();

        // Simulate having a confirmation dialog open
        let confirmation_dialog = crate::views::confirmation_dialog::ConfirmationDialogView::new(
            "Test".to_string(),
            AppEvent::ReviewDelete(review_id_to_delete.clone()),
            AppEvent::ReviewDeleteCancel,
        );
        app.push_view(Box::new(confirmation_dialog));

        assert_eq!(app.view_stack.len(), 2);

        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::ReviewDelete(review_id_to_delete.clone())),
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoaded event and closed the dialog
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event {
            assert_eq!(reviews.len(), 1);
        } else {
            panic!("Expected ReviewsLoaded event");
        }
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);

        // Review should be deleted successfully
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        // Ensure the deleted review is not in the list
        assert!(!reviews.iter().any(|r| r.id == review_id_to_delete));
    }

    #[tokio::test]
    async fn test_process_review_delete_event_no_selection() {
        let mut app = create_test_app().await;

        // Create a review but don't select it
        let review = Review::new("Test Review".to_string());
        review.save(app.database.pool()).await.unwrap();

        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::ReviewDelete("test-id".to_string())),
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoaded event (since delete always sends event)
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event {
            assert_eq!(reviews.len(), 1); // Should still have 1 review since ID didn't match
        } else {
            panic!("Expected ReviewsLoaded event");
        }
    }
}
