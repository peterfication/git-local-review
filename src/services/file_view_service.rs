use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    database::Database,
    event::{AppEvent, EventHandler, ReviewId},
    models::FileView,
    services::{ServiceContext, ServiceHandler},
};

/// Service for handling file view operations
pub struct FileViewService;

impl ServiceHandler for FileViewService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        context: ServiceContext<'a>,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::FileViewToggle {
                    review_id,
                    file_path,
                } => {
                    Self::handle_file_view_toggle(
                        context.database,
                        context.events,
                        review_id,
                        file_path,
                    )
                    .await?;
                }
                AppEvent::FileViewsLoad { review_id } => {
                    Self::handle_file_views_load(context.database, context.events, review_id)
                        .await?;
                }
                _ => {
                    // Event not handled by this service
                }
            }
            Ok(())
        })
    }
}

impl FileViewService {
    /// Toggle the view status of a file for a review
    async fn handle_file_view_toggle(
        database: &Database,
        events: &mut EventHandler,
        review_id: &ReviewId,
        file_path: &str,
    ) -> color_eyre::Result<()> {
        let pool = database.pool();

        // Check if file is currently viewed
        let is_viewed = match FileView::is_file_viewed(pool, review_id, file_path).await {
            Ok(viewed) => viewed,
            Err(error) => {
                events.send(AppEvent::FileViewToggleError {
                    review_id: Arc::from(review_id),
                    file_path: Arc::from(file_path),
                    error: Arc::from(format!("Failed to check file view status: {error}")),
                });
                return Ok(());
            }
        };

        let result = if is_viewed {
            // Mark as unviewed
            FileView::mark_as_unviewed(pool, review_id, file_path).await
        } else {
            // Mark as viewed
            FileView::mark_as_viewed(pool, review_id, file_path).await
        };

        match result {
            Ok(()) => {
                // Send success event and reload file views
                events.send(AppEvent::FileViewToggled {
                    review_id: Arc::from(review_id),
                    file_path: Arc::from(file_path),
                    is_viewed: !is_viewed,
                });
                events.send(AppEvent::FileViewsLoad {
                    review_id: Arc::from(review_id),
                });
            }
            Err(e) => {
                events.send(AppEvent::FileViewToggleError {
                    review_id: Arc::from(review_id),
                    file_path: Arc::from(file_path),
                    error: Arc::from(format!("Failed to toggle file view: {e}")),
                });
            }
        }

        Ok(())
    }

    /// Load the viewed files for a review
    async fn handle_file_views_load(
        database: &Database,
        events: &mut EventHandler,
        review_id: &ReviewId,
    ) -> color_eyre::Result<()> {
        let pool = database.pool();

        events.send(AppEvent::FileViewsLoading {
            review_id: Arc::from(review_id),
        });

        match FileView::get_viewed_files(pool, review_id).await {
            Ok(viewed_files) => {
                events.send(AppEvent::FileViewsLoaded {
                    review_id: Arc::from(review_id),
                    viewed_files: Arc::from(viewed_files),
                });
            }
            Err(e) => {
                events.send(AppEvent::FileViewsLoadError {
                    review_id: Arc::from(review_id),
                    error: Arc::from(format!("Failed to load file views: {e}")),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::SqlitePool;

    use crate::{
        app::App,
        database::Database,
        event::{Event, EventHandler},
        models::Review,
    };

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    async fn create_test_review(database: &Database) -> Review {
        let review = Review::test_review(());
        review.save(database.pool()).await.unwrap();
        review
    }

    #[tokio::test]
    async fn test_handle_file_view_toggle_mark_as_viewed() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let review = create_test_review(&database).await;
        let file_path = "src/main.rs";

        // Ensure file is not viewed initially
        assert!(
            !FileView::is_file_viewed(database.pool(), &review.id, file_path)
                .await
                .unwrap()
        );

        // Toggle file view (should mark as viewed)
        let event = AppEvent::FileViewToggle {
            review_id: Arc::from(review.id.as_str()),
            file_path: Arc::from(file_path),
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        FileViewService::handle_app_event(
            &event,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Verify file is now viewed
        assert!(
            FileView::is_file_viewed(app.database.pool(), &review.id, file_path)
                .await
                .unwrap()
        );

        // Check that success event was sent
        let sent_event = events.try_recv().unwrap();
        match &*sent_event {
            Event::App(AppEvent::FileViewToggled {
                review_id,
                file_path: sent_file_path,
                is_viewed,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(sent_file_path.as_ref(), file_path);
                assert!(*is_viewed);
            }
            _ => panic!("Expected FileViewToggled event, got: {sent_event:?}"),
        }

        // Check that file views reload event was sent
        let reload_event = events.try_recv().unwrap();
        match &*reload_event {
            Event::App(AppEvent::FileViewsLoad { review_id }) => {
                assert_eq!(review_id.as_ref(), review.id);
            }
            _ => panic!("Expected FileViewsLoad event, got: {reload_event:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_file_view_toggle_mark_as_unviewed() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let review = create_test_review(&database).await;
        let file_path = "src/main.rs";

        // First mark as viewed
        FileView::mark_as_viewed(database.pool(), &review.id, file_path)
            .await
            .unwrap();
        assert!(
            FileView::is_file_viewed(database.pool(), &review.id, file_path)
                .await
                .unwrap()
        );

        // Toggle file view (should mark as unviewed)
        let event = AppEvent::FileViewToggle {
            review_id: Arc::from(review.id.as_str()),
            file_path: Arc::from(file_path),
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        FileViewService::handle_app_event(
            &event,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Verify file is now unviewed
        assert!(
            !FileView::is_file_viewed(app.database.pool(), &review.id, file_path)
                .await
                .unwrap()
        );

        // Check that success event was sent
        let sent_event = events.try_recv().unwrap();
        match &*sent_event {
            Event::App(AppEvent::FileViewToggled {
                review_id,
                file_path: sent_file_path,
                is_viewed,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(sent_file_path.as_ref(), file_path);
                assert!(!*is_viewed);
            }
            _ => panic!("Expected FileViewToggled event, got: {sent_event:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_file_views_load_success() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let review = create_test_review(&database).await;

        let file_paths = vec!["src/main.rs", "src/lib.rs"];
        for file_path in &file_paths {
            FileView::mark_as_viewed(database.pool(), &review.id, file_path)
                .await
                .unwrap();
        }

        // Load file views
        let event = AppEvent::FileViewsLoad {
            review_id: Arc::from(review.id.as_str()),
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        FileViewService::handle_app_event(
            &event,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Check loading event was sent
        let loading_event = events.try_recv().unwrap();
        match &*loading_event {
            Event::App(AppEvent::FileViewsLoading { review_id }) => {
                assert_eq!(review_id.as_ref(), review.id);
            }
            _ => panic!("Expected FileViewsLoading event, got: {loading_event:?}"),
        }

        // Check loaded event was sent
        let loaded_event = events.try_recv().unwrap();
        match &*loaded_event {
            Event::App(AppEvent::FileViewsLoaded {
                review_id,
                viewed_files,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(viewed_files.len(), 2);
                assert!(viewed_files.contains(&"src/main.rs".to_string()));
                assert!(viewed_files.contains(&"src/lib.rs".to_string()));
            }
            _ => panic!("Expected FileViewsLoaded event, got: {loaded_event:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_file_views_load_empty() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let review = create_test_review(&database).await;

        // Load file views (none marked as viewed)
        let event = AppEvent::FileViewsLoad {
            review_id: Arc::from(review.id.as_str()),
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        FileViewService::handle_app_event(
            &event,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Skip loading event
        events.try_recv().unwrap();

        // Check loaded event was sent with empty list
        let loaded_event = events.try_recv().unwrap();
        match &*loaded_event {
            Event::App(AppEvent::FileViewsLoaded {
                review_id,
                viewed_files,
            }) => {
                assert_eq!(review_id.as_ref(), review.id);
                assert_eq!(viewed_files.len(), 0);
            }
            _ => panic!("Expected FileViewsLoaded event, got: {loaded_event:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_unrelated_event() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Send an unrelated event
        let event = AppEvent::Quit;

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        FileViewService::handle_app_event(
            &event,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // No events should be sent
        assert!(!events.has_pending_events());
    }
}
