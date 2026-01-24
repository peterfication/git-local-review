use std::{future::Future, pin::Pin};

use crate::event::AppEvent;
use crate::models::Review;
use crate::services::{GitService, ServiceContext, ServiceHandler};
use crate::time_provider::{SystemTimeProvider, TimeProvider};

pub struct BranchStatusService;

impl BranchStatusService {
    /// Check all reviews against current Git repository state
    async fn handle_reviews_branch_status_check(context: ServiceContext<'_>) {
        log::info!("Checking branch status for all reviews");

        // Get all reviews from the database
        match Review::list_all(context.database.pool()).await {
            Ok(reviews) => {
                for mut review in reviews {
                    // Check if branches still exist and if SHAs changed
                    let base_branch_exists =
                        GitService::get_branch_sha(context.repo_path, &review.base_branch)
                            .map(|sha_opt| sha_opt.is_some())
                            .unwrap_or(false);

                    let target_branch_exists =
                        GitService::get_branch_sha(context.repo_path, &review.target_branch)
                            .map(|sha_opt| sha_opt.is_some())
                            .unwrap_or(false);

                    let mut base_sha_changed = None;
                    let mut target_sha_changed = None;

                    // Check if SHAs changed (only if branches exist and we have original SHAs)
                    if base_branch_exists
                        && let Ok(Some(current_base_sha)) =
                            GitService::get_branch_sha(context.repo_path, &review.base_branch)
                        && let Some(original_base_sha) = &review.base_sha
                        && current_base_sha != *original_base_sha
                    {
                        base_sha_changed = Some(current_base_sha);
                    }

                    if target_branch_exists
                        && let Ok(Some(current_target_sha)) =
                            GitService::get_branch_sha(context.repo_path, &review.target_branch)
                        && let Some(original_target_sha) = &review.target_sha
                        && current_target_sha != *original_target_sha
                    {
                        target_sha_changed = Some(current_target_sha);
                    }

                    // Update the review only if there are changes to report
                    if base_sha_changed.is_some()
                        || target_sha_changed.is_some()
                        || review.base_branch_exists != Some(base_branch_exists)
                        || review.target_branch_exists != Some(target_branch_exists)
                    {
                        // Capture values before moving them
                        let base_changed = base_sha_changed.is_some();
                        let target_changed = target_sha_changed.is_some();

                        // Update the review's updated_at timestamp
                        review.updated_at = SystemTimeProvider.now();
                        if let Err(e) = review
                            .update_branch_status(
                                context.database.pool(),
                                base_sha_changed,
                                target_sha_changed,
                                Some(base_branch_exists),
                                Some(target_branch_exists),
                            )
                            .await
                        {
                            log::error!(
                                "Failed to update branch status for review {}: {e}",
                                review.id
                            );
                        } else {
                            log::info!(
                                "Updated branch status for review {} (base_exists: {base_branch_exists}, target_exists: {target_branch_exists}, base_changed: {base_changed}, target_changed: {target_changed})",
                                review.id
                            );
                        }
                    }
                }

                // Trigger a reload of reviews to reflect the changes
                context.events.send(AppEvent::ReviewsLoad);
            }
            Err(e) => {
                log::error!("Failed to load reviews for branch status check: {e}");
            }
        }
    }
}

impl ServiceHandler for BranchStatusService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        context: ServiceContext<'a>,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::ReviewsBranchStatusCheck => {
                    Self::handle_reviews_branch_status_check(context).await;
                }
                _ => {
                    // Other events are ignored
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{fs, path::Path};

    use git2::build::CheckoutBuilder;
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    use crate::{
        database::Database,
        event::{AppEvent, Event, EventHandler},
        models::Review,
    };

    const BASE_BRANCH: &str = "base-test";
    const BASE_BRANCH_REF: &str = "refs/heads/base-test";
    const TARGET_BRANCH: &str = "target-test";
    const TARGET_BRANCH_REF: &str = "refs/heads/target-test";

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    fn create_test_git_repo() -> (TempDir, git2::Repository) {
        let temp_dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(temp_dir.path()).unwrap();

        let commit_id = {
            let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
            let mut index = repo.index().unwrap();
            let readme_path = temp_dir.path().join("README.md");
            fs::write(&readme_path, b"Initial commit contents").unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )
            .unwrap()
        };
        {
            let head_commit = repo.find_commit(commit_id).unwrap();
            repo.branch(BASE_BRANCH, &head_commit, false).unwrap();
            repo.branch(TARGET_BRANCH, &head_commit, false).unwrap();
        }
        repo.set_head(BASE_BRANCH_REF).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();

        (temp_dir, repo)
    }

    fn commit_on_branch(
        repo: &git2::Repository,
        branch_ref: &str,
        file_name: &str,
        contents: &str,
    ) {
        repo.set_head(branch_ref).unwrap();
        repo.checkout_head(Some(CheckoutBuilder::default().force()))
            .unwrap();

        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let workdir = repo.workdir().unwrap();
        let file_path = workdir.join(file_name);
        fs::write(&file_path, contents).unwrap();
        index.add_path(Path::new(file_name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Test commit",
            &tree,
            &[&parent_commit],
        )
        .unwrap();
    }

    async fn save_review(database: &Database, review: &Review) {
        review.save(database.pool()).await.unwrap();
    }

    async fn run_branch_status_check(
        database: &Database,
        repo_path: &str,
        events: &mut EventHandler,
    ) {
        let event = AppEvent::ReviewsBranchStatusCheck;
        BranchStatusService::handle_app_event(
            &event,
            ServiceContext {
                database,
                repo_path,
                events,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_branch_status_service_detects_sha_changes() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let (temp_dir, repo) = create_test_git_repo();
        let repo_path = temp_dir.path().to_str().unwrap();

        let original_base_sha = GitService::get_branch_sha(repo_path, BASE_BRANCH)
            .unwrap()
            .unwrap();
        let original_target_sha = GitService::get_branch_sha(repo_path, TARGET_BRANCH)
            .unwrap()
            .unwrap();

        let review = Review::builder()
            .base_branch(BASE_BRANCH)
            .target_branch(TARGET_BRANCH)
            .base_sha(Some(original_base_sha.clone()))
            .target_sha(Some(original_target_sha.clone()))
            .base_branch_exists(Some(true))
            .target_branch_exists(Some(true))
            .build();
        save_review(&database, &review).await;

        commit_on_branch(&repo, BASE_BRANCH_REF, "README.md", "Updated base branch");
        commit_on_branch(
            &repo,
            TARGET_BRANCH_REF,
            "README.md",
            "Updated target branch",
        );

        let new_base_sha = GitService::get_branch_sha(repo_path, BASE_BRANCH)
            .unwrap()
            .unwrap();
        let new_target_sha = GitService::get_branch_sha(repo_path, TARGET_BRANCH)
            .unwrap()
            .unwrap();
        assert_ne!(original_base_sha, new_base_sha);
        assert_ne!(original_target_sha, new_target_sha);

        run_branch_status_check(&database, repo_path, &mut events).await;

        let updated_review = Review::find_by_id(database.pool(), &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated_review.base_sha_changed.as_deref(),
            Some(new_base_sha.as_str())
        );
        assert_eq!(
            updated_review.target_sha_changed.as_deref(),
            Some(new_target_sha.as_str())
        );
        assert_eq!(updated_review.base_branch_exists, Some(true));
        assert_eq!(updated_review.target_branch_exists, Some(true));

        let sent_event = events.try_recv().expect("expected ReviewsLoad event");
        match &*sent_event {
            Event::App(AppEvent::ReviewsLoad) => {}
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_branch_status_service_updates_branch_existence() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let (temp_dir, _repo) = create_test_git_repo();
        let repo_path = temp_dir.path().to_str().unwrap();

        let base_sha = GitService::get_branch_sha(repo_path, BASE_BRANCH)
            .unwrap()
            .unwrap();

        let review = Review::builder()
            .base_branch(BASE_BRANCH)
            .target_branch("does-not-exist")
            .base_sha(Some(base_sha.clone()))
            .target_sha(None)
            .base_branch_exists(Some(true))
            .target_branch_exists(Some(true))
            .build();
        save_review(&database, &review).await;

        run_branch_status_check(&database, repo_path, &mut events).await;

        let updated_review = Review::find_by_id(database.pool(), &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated_review.base_sha_changed, None);
        assert_eq!(updated_review.target_sha_changed, None);
        assert_eq!(updated_review.base_branch_exists, Some(true));
        assert_eq!(updated_review.target_branch_exists, Some(false));

        let sent_event = events.try_recv().expect("expected ReviewsLoad event");
        match &*sent_event {
            Event::App(AppEvent::ReviewsLoad) => {}
            other => panic!("Unexpected event: {other:?}"),
        }
    }
}
