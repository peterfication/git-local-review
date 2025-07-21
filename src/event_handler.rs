use std::sync::Arc;

use ratatui::crossterm::event::KeyEvent;

use crate::{
    app::App,
    event::{AppEvent, Event},
    services::{ReviewService, ServiceHandler},
    views::{
        ConfirmationDialogView, HelpModalView, KeyBinding, ReviewCreateView, ReviewDetailsView,
    },
};

pub struct EventProcessor;

impl EventProcessor {
    pub async fn process_event(app: &mut App, event: Arc<Event>) -> color_eyre::Result<()> {
        match *event {
            Event::Tick => app.tick(),
            #[allow(clippy::single_match)]
            Event::Crossterm(ref event) => match event {
                crossterm::event::Event::Key(key_event) => app.handle_key_events(key_event)?,
                _ => {}
            },
            Event::App(ref app_event) => {
                log::info!("Processing event: {app_event:#?}");

                // First let services handle the event
                Self::handle_services(app, app_event).await?;

                // Then let views handle the event
                app.handle_app_events(app_event);

                // Finally handle app events globally
                match *app_event {
                    AppEvent::Quit => app.quit(),
                    AppEvent::ViewClose => app.pop_view(),
                    // Events that open views
                    AppEvent::ReviewCreateOpen => Self::review_create_open(app),
                    AppEvent::ReviewDeleteConfirm(ref review_id) => {
                        Self::review_delete_confirm(app, review_id)
                    }
                    AppEvent::HelpOpen(ref keybindings) => Self::help_open(app, keybindings),
                    AppEvent::HelpKeySelected(ref key_event) => {
                        Self::help_key_selected(app, key_event)
                    }
                    AppEvent::ReviewDetailsOpen(ref review_id) => {
                        Self::review_details_open(app, review_id)
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

    /// Open delete confirmation dialog
    fn review_delete_confirm(app: &mut App, review_id: &str) {
        // Create a generic confirmation dialog without the specific review title
        // since we don't have access to the reviews in the App anymore
        // TODO: Load the title from the review_service / database
        let message = "Do you want to delete the selected review?".to_string();
        let confirmation_dialog = ConfirmationDialogView::new(
            message,
            AppEvent::ReviewDelete(review_id.into()),
            AppEvent::ViewClose,
        );
        app.push_view(Box::new(confirmation_dialog));
    }

    /// Open help modal with provided keybindings
    fn help_open(app: &mut App, keybindings: &Arc<[KeyBinding]>) {
        let help_modal = HelpModalView::new(Arc::clone(keybindings));
        app.push_view(Box::new(help_modal));
    }

    /// Handle key selected from help modal
    fn help_key_selected(app: &mut App, key_event: &KeyEvent) {
        // First close the help modal
        app.events.send(AppEvent::ViewClose);
        // Then send the selected key event through the normal event flow
        app.events.send_key_event(*key_event);
    }

    /// Open review details view
    fn review_details_open(app: &mut App, review_id: &str) {
        // Create an empty ReviewDetailsView and trigger loading
        app.push_view(Box::new(ReviewDetailsView::new_loading()));
        app.events.send(AppEvent::ReviewLoad(Arc::from(review_id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::review::Review;
    use crate::views::{MainView, ViewType};
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

        EventProcessor::process_event(&mut app, Event::App(AppEvent::Quit).into())
            .await
            .unwrap();

        assert!(!app.running);
    }

    #[tokio::test]
    async fn test_process_review_create_open_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen).into())
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewCreate
        );
    }

    #[tokio::test]
    async fn test_process_view_close_event() {
        let mut app = create_test_app().await;

        // First open a review create view
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ReviewCreateOpen).into())
            .await
            .unwrap();
        assert_eq!(app.view_stack.len(), 2);

        // Then close it
        EventProcessor::process_event(&mut app, Event::App(AppEvent::ViewClose).into())
            .await
            .unwrap();

        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_tick_event() {
        let mut app = create_test_app().await;

        // Tick event should not change anything
        EventProcessor::process_event(&mut app, Event::Tick.into())
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

        EventProcessor::process_event(&mut app, Event::Crossterm(crossterm_event).into())
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
            Event::App(AppEvent::ReviewDeleteConfirm(review_id.into())).into(),
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
            Event::App(AppEvent::ReviewDeleteConfirm(Arc::from("non-existent-id"))).into(),
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
            AppEvent::ReviewDelete("test-id".into()),
            AppEvent::ViewClose,
        );
        app.push_view(Box::new(confirmation_dialog));
        assert_eq!(app.view_stack.len(), 2);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::ViewClose).into())
            .await
            .unwrap();

        // Should have closed the confirmation dialog
        assert_eq!(app.view_stack.len(), 1);
        assert_eq!(app.view_stack.last().unwrap().view_type(), ViewType::Main);
    }

    #[tokio::test]
    async fn test_process_help_open_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1); // Only MainView

        // Create some keybindings for testing
        let keybindings: Arc<[crate::views::KeyBinding]> = Arc::new([crate::views::KeyBinding {
            key: "q".to_string(),
            description: "Quit".to_string(),
            key_event: ratatui::crossterm::event::KeyEvent {
                code: ratatui::crossterm::event::KeyCode::Char('q'),
                modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                kind: ratatui::crossterm::event::KeyEventKind::Press,
                state: ratatui::crossterm::event::KeyEventState::empty(),
            },
        }]);

        EventProcessor::process_event(&mut app, Event::App(AppEvent::HelpOpen(keybindings)).into())
            .await
            .unwrap();

        // Should have added a help modal view
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::HelpModal
        );
    }

    #[tokio::test]
    async fn test_process_help_key_selected_event() {
        let mut app = create_test_app().await;

        // First add a help modal
        let keybindings: Arc<[crate::views::KeyBinding]> = Arc::new([crate::views::KeyBinding {
            key: "q".to_string(),
            description: "Quit".to_string(),
            key_event: ratatui::crossterm::event::KeyEvent {
                code: ratatui::crossterm::event::KeyCode::Char('q'),
                modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                kind: ratatui::crossterm::event::KeyEventKind::Press,
                state: ratatui::crossterm::event::KeyEventState::empty(),
            },
        }]);
        let help_modal = HelpModalView::new(keybindings);
        app.push_view(Box::new(help_modal));
        assert_eq!(app.view_stack.len(), 2);
        assert!(!app.events.has_pending_events());

        // Now process a help key selected event
        let selected_key_event = ratatui::crossterm::event::KeyEvent {
            code: ratatui::crossterm::event::KeyCode::Char('n'),
            modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::empty(),
        };

        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::HelpKeySelected(Arc::new(selected_key_event))).into(),
        )
        .await
        .unwrap();

        // Should have sent ViewClose and the key event
        assert!(app.events.has_pending_events());

        // First event should be ViewClose
        let event1 = app.events.try_recv().unwrap();
        assert!(matches!(*event1, Event::App(AppEvent::ViewClose)));

        // Second event should be the key event as a crossterm event
        let event2 = app.events.try_recv().unwrap();
        if let Event::Crossterm(ratatui::crossterm::event::Event::Key(key_event)) = *event2 {
            assert_eq!(
                key_event.code,
                ratatui::crossterm::event::KeyCode::Char('n')
            );
            assert_eq!(
                key_event.modifiers,
                ratatui::crossterm::event::KeyModifiers::empty()
            );
        } else {
            panic!("Expected crossterm key event, got: {event2:?}");
        }
    }

    #[tokio::test]
    async fn test_help_open_function() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1); // Only MainView

        let keybindings: Arc<[crate::views::KeyBinding]> = Arc::new([crate::views::KeyBinding {
            key: "test".to_string(),
            description: "Test keybinding".to_string(),
            key_event: ratatui::crossterm::event::KeyEvent {
                code: ratatui::crossterm::event::KeyCode::Char('t'),
                modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
                kind: ratatui::crossterm::event::KeyEventKind::Press,
                state: ratatui::crossterm::event::KeyEventState::empty(),
            },
        }]);

        EventProcessor::help_open(&mut app, &keybindings);

        // Should have added help modal to view stack
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::HelpModal
        );
    }

    #[tokio::test]
    async fn test_help_key_selected_function() {
        let mut app = create_test_app().await;
        assert!(!app.events.has_pending_events());

        let key_event = ratatui::crossterm::event::KeyEvent {
            code: ratatui::crossterm::event::KeyCode::Enter,
            modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::empty(),
        };

        EventProcessor::help_key_selected(&mut app, &key_event);

        // Should have sent ViewClose and the key event
        assert!(app.events.has_pending_events());

        // First event should be ViewClose
        let event1 = app.events.try_recv().unwrap();
        assert!(matches!(*event1, Event::App(AppEvent::ViewClose)));

        // Second event should be the key event as a crossterm event
        let event2 = app.events.try_recv().unwrap();
        if let Event::Crossterm(ratatui::crossterm::event::Event::Key(received_key_event)) = *event2
        {
            assert_eq!(
                received_key_event.code,
                ratatui::crossterm::event::KeyCode::Enter
            );
            assert_eq!(
                received_key_event.modifiers,
                ratatui::crossterm::event::KeyModifiers::empty()
            );
            assert_eq!(
                received_key_event.kind,
                ratatui::crossterm::event::KeyEventKind::Press
            );
            assert_eq!(
                received_key_event.state,
                ratatui::crossterm::event::KeyEventState::empty()
            );
        } else {
            panic!("Expected crossterm key event, got: {event2:?}");
        }
    }

    #[tokio::test]
    async fn test_process_review_details_open_event() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1); // Only MainView
        assert!(!app.events.has_pending_events());

        let review_id = "test-review-id";
        EventProcessor::process_event(
            &mut app,
            Event::App(AppEvent::ReviewDetailsOpen(review_id.into())).into(),
        )
        .await
        .unwrap();

        // Should have added a ReviewDetailsView to the stack
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewDetails
        );

        // Should have sent a ReviewLoad event
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::ReviewLoad(event_review_id)) => {
                assert_eq!(event_review_id.as_ref(), review_id);
            }
            _ => panic!("Expected ReviewLoad event, got: {event:?}"),
        }
    }

    #[tokio::test]
    async fn test_review_details_open_function() {
        let mut app = create_test_app().await;
        assert_eq!(app.view_stack.len(), 1); // Only MainView
        assert!(!app.events.has_pending_events());

        let review_id = "direct-test-id";
        EventProcessor::review_details_open(&mut app, review_id);

        // Should have added a ReviewDetailsView to the stack
        assert_eq!(app.view_stack.len(), 2);
        assert_eq!(
            app.view_stack.last().unwrap().view_type(),
            ViewType::ReviewDetails
        );

        // Should have sent a ReviewLoad event
        assert!(app.events.has_pending_events());
        let event = app.events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::ReviewLoad(event_review_id)) => {
                assert_eq!(event_review_id.as_ref(), review_id);
            }
            _ => panic!("Expected ReviewLoad event, got: {event:?}"),
        }
    }
}
