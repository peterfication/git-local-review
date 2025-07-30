use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    database::Database,
    event::{AppEvent, EventHandler, ReviewId},
    models::Comment,
    services::ServiceHandler,
};

/// Loading state for comments
#[derive(Debug, Clone)]
pub enum CommentsLoadingState {
    Init,
    Loading,
    Loaded(Arc<Vec<Comment>>),
    Error(Arc<str>),
}

/// Service for handling comment operations
pub struct CommentService;

impl ServiceHandler for CommentService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        database: &'a Database,
        events: &'a mut EventHandler,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = color_eyre::Result<()>> + Send + 'a>>
    {
        Box::pin(async move {
            match event {
                AppEvent::CommentsLoad {
                    review_id,
                    file_path,
                    line_number,
                } => {
                    Self::handle_comments_load(database, events, review_id, file_path, line_number)
                        .await?;
                }
                AppEvent::CommentCreate {
                    review_id,
                    file_path,
                    line_number,
                    content,
                } => {
                    Self::handle_comment_create(
                        database,
                        events,
                        review_id,
                        file_path,
                        *line_number,
                        content,
                    )
                    .await?;
                }
                AppEvent::CommentMetadataLoad { review_id } => {
                    Self::handle_comment_metadata_load(database, events, review_id).await?;
                }
                _ => {
                    // Event not handled by this service
                }
            }
            Ok(())
        })
    }
}

impl CommentService {
    /// Load comments for a review, file or line
    async fn handle_comments_load(
        database: &Database,
        events: &mut EventHandler,
        review_id: &Arc<ReviewId>,
        file_path: &Arc<Option<String>>,
        line_number: &Arc<Option<i64>>,
    ) -> color_eyre::Result<()> {
        let pool = database.pool();

        events.send(AppEvent::CommentsLoadingState {
            review_id: review_id.clone(),
            file_path: file_path.clone(),
            line_number: line_number.clone(),
            state: CommentsLoadingState::Loading,
        });

        let result = match file_path.as_ref() {
            Some(file_path_present) => {
                match **line_number {
                    Some(line_number_present) => {
                        // Load comments for a specific line
                        Comment::find_for_line(
                            pool,
                            review_id,
                            file_path_present,
                            line_number_present,
                        )
                        .await
                    }
                    None => {
                        // Load comments for a whole file (file-level and line-level comments)
                        Comment::find_for_file(pool, review_id, file_path_present).await
                    }
                }
            }
            None => {
                // Load comments for the whole file (file-level comments only)
                Comment::find_for_review(pool, review_id).await
            }
        };

        match result {
            Ok(comments) => {
                events.send(AppEvent::CommentsLoadingState {
                    review_id: review_id.clone(),
                    file_path: file_path.clone(),
                    line_number: line_number.clone(),
                    state: CommentsLoadingState::Loaded(Arc::new(comments)),
                });
            }
            Err(error) => {
                events.send(AppEvent::CommentsLoadingState {
                    review_id: review_id.clone(),
                    file_path: file_path.clone(),
                    line_number: line_number.clone(),
                    state: CommentsLoadingState::Error(Arc::from(format!(
                        "Failed to load comments: {error}"
                    ))),
                });
            }
        }

        Ok(())
    }

    /// Create a new comment
    async fn handle_comment_create(
        database: &Database,
        events: &mut EventHandler,
        review_id: &ReviewId,
        file_path: &str,
        line_number: Option<i64>,
        content: &str,
    ) -> color_eyre::Result<()> {
        let pool = database.pool();

        // Validate content
        let trimmed_content = content.trim();
        if trimmed_content.is_empty() {
            events.send(AppEvent::CommentCreateError(Arc::from(
                "Comment content cannot be empty".to_string(),
            )));
            return Ok(());
        }

        let comment = Comment::new(review_id, file_path, line_number, trimmed_content);

        // Save comment to database
        match comment.create(pool).await {
            Ok(()) => {
                // Send success event
                events.send(AppEvent::CommentCreated(Arc::from(comment.clone())));

                // Trigger reload of comments for the same target
                events.send(AppEvent::CommentsLoad {
                    review_id: Arc::from(review_id),
                    file_path: Arc::new(Some(file_path.to_string())),
                    line_number: Arc::from(line_number),
                });
            }
            Err(error) => {
                events.send(AppEvent::CommentCreateError(Arc::from(format!(
                    "Failed to create comment: {error}"
                ))));
            }
        }

        Ok(())
    }

    /// Load comment metadata for a review (files with comments, lines with comments per file)
    async fn handle_comment_metadata_load(
        database: &Database,
        events: &mut EventHandler,
        review_id: &ReviewId,
    ) -> color_eyre::Result<()> {
        let pool = database.pool();

        // Get all files with comments
        let files_with_comments = match Comment::get_files_with_comments(pool, review_id).await {
            Ok(files) => Arc::new(files),
            Err(error) => {
                log::error!("Failed to load files with comments: {error}");
                Arc::new(vec![])
            }
        };

        // Get lines with comments for each file
        let mut lines_with_comments = HashMap::new();
        for file_path in files_with_comments.iter() {
            match Comment::get_lines_with_comments(pool, review_id, file_path).await {
                Ok(lines) => {
                    lines_with_comments.insert(file_path.clone(), lines);
                }
                Err(error) => {
                    log::error!("Failed to load lines with comments for {file_path}: {error}");
                }
            }
        }

        // Send metadata loaded event
        events.send(AppEvent::CommentMetadataLoaded {
            review_id: Arc::from(review_id),
            files_with_comments,
            lines_with_comments: Arc::from(lines_with_comments),
        });

        Ok(())
    }

    /// Check if a file has any comments (used for comment indicators)
    pub async fn file_has_comments(
        database: &Database,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<bool> {
        Comment::file_has_comments(database.pool(), review_id, file_path).await
    }

    /// Check if a specific line has comments (used for comment indicators)
    pub async fn line_has_comments(
        database: &Database,
        review_id: &str,
        file_path: &str,
        line_number: i64,
    ) -> color_eyre::Result<bool> {
        Comment::line_has_comments(database.pool(), review_id, file_path, line_number).await
    }

    /// Get all files with comments for a review
    pub async fn get_files_with_comments(
        database: &Database,
        review_id: &str,
    ) -> color_eyre::Result<Vec<String>> {
        Comment::get_files_with_comments(database.pool(), review_id).await
    }

    /// Get all line numbers with comments for a specific file
    pub async fn get_lines_with_comments(
        database: &Database,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<Vec<i64>> {
        Comment::get_lines_with_comments(database.pool(), review_id, file_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::Database, event::EventHandler, models::Comment};
    use sqlx::SqlitePool;

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    #[tokio::test]
    async fn test_comment_service_create_file_comment() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(database.pool()).await.unwrap();

        // Create a file comment
        CommentService::handle_comment_create(
            &database,
            &mut events,
            &review.id,
            "src/main.rs",
            None,
            "This is a file comment",
        )
        .await
        .unwrap();

        // Should send CommentCreated event
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentCreated(comment)) => {
                assert_eq!(comment.review_id, review.id);
                assert_eq!(comment.file_path, "src/main.rs");
                assert_eq!(comment.line_number, None);
                assert_eq!(comment.content, "This is a file comment");
                assert!(comment.is_file_comment());
            }
            _ => panic!("Expected CommentCreated event"),
        }

        // Should send CommentsLoad event to reload
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoad {
                review_id,
                file_path,
                line_number,
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                match file_path.as_ref() {
                    Some(file_path_conent) => {
                        assert_eq!(file_path_conent, "src/main.rs");
                    }
                    None => panic!("Expected file path to be Some"),
                }
                assert_eq!(*line_number.as_ref(), None);
            }
            _ => panic!("Expected CommentsLoad event"),
        }
    }

    #[tokio::test]
    async fn test_comment_service_create_line_comment() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(database.pool()).await.unwrap();

        // Create a line comment
        CommentService::handle_comment_create(
            &database,
            &mut events,
            &review.id,
            "src/main.rs",
            Some(42),
            "This is a line comment",
        )
        .await
        .unwrap();

        // Should send CommentCreated event
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentCreated(comment)) => {
                assert_eq!(comment.review_id, review.id);
                assert_eq!(comment.file_path, "src/main.rs");
                assert_eq!(comment.line_number, Some(42));
                assert_eq!(comment.content, "This is a line comment");
                assert!(comment.is_line_comment());
            }
            _ => panic!("Expected CommentCreated event"),
        }

        // Should send CommentsLoad event to reload
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoad {
                review_id,
                file_path,
                line_number,
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                match file_path.as_ref() {
                    Some(file_path_content) => {
                        assert_eq!(file_path_content, "src/main.rs");
                    }
                    None => panic!("Expected file path to be Some"),
                }
                match line_number.as_ref() {
                    Some(line_number_value) => {
                        assert_eq!(*line_number_value, 42);
                    }
                    None => panic!("Expected line number to be Some"),
                }
            }
            _ => panic!("Expected CommentsLoad event"),
        }
    }

    #[tokio::test]
    async fn test_comment_service_create_empty_comment() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Try to create a comment with empty content
        CommentService::handle_comment_create(
            &database,
            &mut events,
            "review-123",
            "src/main.rs",
            None,
            "   ", // Only whitespace
        )
        .await
        .unwrap();

        // Should send CommentCreateError event
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentCreateError(error)) => {
                assert_eq!(error.to_string(), "Comment content cannot be empty");
            }
            _ => panic!("Expected CommentCreateError event"),
        }
    }

    #[tokio::test]
    async fn test_comment_service_load_file_comments() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(database.pool()).await.unwrap();

        // Create a test comment first
        let comment = Comment::new(&review.id, "src/main.rs", None, "Test comment");
        comment.create(database.pool()).await.unwrap();

        let review_id = Arc::from(review.id.clone());
        let file_path = Arc::from(Some("src/main.rs".to_string()));
        let line_number = Arc::from(None);

        // Load comments for the file
        CommentService::handle_comments_load(
            &database,
            &mut events,
            &review_id,
            &file_path,
            &line_number,
        )
        .await
        .unwrap();

        // Should send Loading state first
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoadingState {
                review_id,
                file_path,
                line_number,
                state: CommentsLoadingState::Loading,
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                assert_eq!(file_path.as_deref(), Some("src/main.rs"));
                assert!(line_number.is_none());
            }
            _ => panic!("Expected Loading state"),
        }

        // Should send Loaded state with comments
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoadingState {
                review_id,
                file_path,
                line_number,
                state: CommentsLoadingState::Loaded(comments),
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                assert_eq!(file_path.as_deref(), Some("src/main.rs"));
                assert!(line_number.is_none());
                assert_eq!(comments.len(), 1);
                assert_eq!(comments[0].content, "Test comment");
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[tokio::test]
    async fn test_comment_service_load_line_comments() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(database.pool()).await.unwrap();

        // Create a test line comment
        let comment = Comment::new(&review.id, "src/main.rs", Some(10), "Line comment");
        comment.create(database.pool()).await.unwrap();

        let review_id = Arc::from(review.id.clone());
        let file_path = Arc::from(Some("src/main.rs".to_string()));
        let line_number = Arc::from(Some(10));

        // Load comments for the specific line
        CommentService::handle_comments_load(
            &database,
            &mut events,
            &review_id,
            &file_path,
            &line_number,
        )
        .await
        .unwrap();

        // Should send Loading state first
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoadingState {
                review_id,
                file_path,
                line_number,
                state: CommentsLoadingState::Loading,
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                assert_eq!(file_path.as_deref(), Some("src/main.rs"));
                match line_number.as_ref() {
                    Some(line_number_value) => {
                        assert_eq!(*line_number_value, 10);
                    }
                    None => panic!("Expected line number to be Some"),
                }
            }
            _ => panic!("Expected Loading state"),
        }

        // Should send Loaded state with comments
        let event = events.try_recv().unwrap();
        match &*event {
            crate::event::Event::App(AppEvent::CommentsLoadingState {
                review_id,
                file_path,
                line_number,
                state: CommentsLoadingState::Loaded(comments),
            }) => {
                assert_eq!(review_id.to_string(), review.id);
                assert_eq!(file_path.as_deref(), Some("src/main.rs"));
                match line_number.as_ref() {
                    Some(line_number_value) => {
                        assert_eq!(*line_number_value, 10);
                    }
                    None => panic!("Expected line number to be Some"),
                }
                assert_eq!(comments.len(), 1);
                assert_eq!(comments[0].content, "Line comment");
                assert_eq!(comments[0].line_number, Some(10));
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[tokio::test]
    async fn test_comment_service_helper_methods() {
        let database = create_test_database().await;

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(database.pool()).await.unwrap();

        // Create test comments
        let file_comment = Comment::new(&review.id, "src/main.rs", None, "File comment");
        file_comment.create(database.pool()).await.unwrap();

        let line_comment = Comment::new(&review.id, "src/main.rs", Some(5), "Line comment");
        line_comment.create(database.pool()).await.unwrap();

        // Test file_has_comments
        let has_comments = CommentService::file_has_comments(&database, &review.id, "src/main.rs")
            .await
            .unwrap();
        assert!(has_comments);

        let no_comments = CommentService::file_has_comments(&database, &review.id, "src/other.rs")
            .await
            .unwrap();
        assert!(!no_comments);

        // Test line_has_comments
        let line_has_comments =
            CommentService::line_has_comments(&database, &review.id, "src/main.rs", 5)
                .await
                .unwrap();
        assert!(line_has_comments);

        let line_no_comments =
            CommentService::line_has_comments(&database, &review.id, "src/main.rs", 10)
                .await
                .unwrap();
        assert!(!line_no_comments);

        // Test get_files_with_comments
        let files = CommentService::get_files_with_comments(&database, &review.id)
            .await
            .unwrap();
        assert_eq!(files, vec!["src/main.rs"]);

        // Test get_lines_with_comments
        let lines = CommentService::get_lines_with_comments(&database, &review.id, "src/main.rs")
            .await
            .unwrap();
        assert_eq!(lines, vec![5]);
    }
}
