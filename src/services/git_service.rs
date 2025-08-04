use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::event::{AppEvent, EventHandler};
use crate::models::{Diff, DiffFile};
use crate::services::{ServiceContext, ServiceHandler};

/// State of Git branches loading process
#[derive(Debug, Clone, PartialEq, Default)]
pub enum GitBranchesLoadingState {
    /// Initial state - no loading has been attempted
    #[default]
    Init,
    /// Currently loading branches from Git repository
    Loading,
    /// Branches have been successfully loaded
    Loaded(Arc<[String]>),
    /// Error occurred during loading
    Error(Arc<str>),
}

/// State of Git diff loading process with structured data
#[derive(Debug, Clone, PartialEq, Default)]
pub enum GitDiffLoadingState {
    /// Initial state - no loading has been attempted
    #[default]
    Init,
    /// Currently loading diff from Git repository
    Loading,
    /// Diff has been successfully loaded with structured data
    Loaded(Arc<Diff>),
    /// Error occurred during loading
    Error(Arc<str>),
}

pub struct GitService;

impl GitService {
    pub fn get_branches<P: AsRef<Path>>(repo_path: P) -> color_eyre::Result<Arc<[String]>> {
        let repo = git2::Repository::open(repo_path)?;
        let mut branches = Vec::new();

        // Get local branches
        let branch_iter = repo.branches(Some(git2::BranchType::Local))?;

        for branch_result in branch_iter {
            let (branch, _) = branch_result?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }

        // Sort branches alphabetically
        branches.sort();
        Ok(branches.into())
    }

    /// Get the SHA of a specific branch
    pub fn get_branch_sha<PathRef: AsRef<Path>>(
        repo_path: PathRef,
        branch_name: &str,
    ) -> color_eyre::Result<Option<String>> {
        let repo = git2::Repository::open(repo_path)?;

        // Try to find the branch reference
        match repo.find_reference(&Self::get_branch_reference_name(branch_name)) {
            Ok(reference) => {
                if let Some(oid) = reference.target() {
                    Ok(Some(oid.to_string()))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None), // Branch doesn't exist
        }
    }

    /// Get the full reference name for a branch
    fn get_branch_reference_name(branch_name: &str) -> String {
        format!("refs/heads/{branch_name}")
    }

    /// Get the diff between two SHAs as structured data
    pub fn get_diff_between_shas<PathRef: AsRef<Path>>(
        repo_path: PathRef,
        base_sha: &str,
        target_sha: &str,
    ) -> color_eyre::Result<Diff> {
        let repo = git2::Repository::open(repo_path)?;

        // Parse SHAs to git2::Oid
        let base_oid = git2::Oid::from_str(base_sha)?;
        let target_oid = git2::Oid::from_str(target_sha)?;

        // Get commit objects
        let base_commit = repo.find_commit(base_oid)?;
        let target_commit = repo.find_commit(target_oid)?;

        // Get trees from commits
        let base_tree = base_commit.tree()?;
        let target_tree = target_commit.tree()?;

        // Create diff between trees
        let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&target_tree), None)?;

        // Parse diff into structured format
        Self::parse_git_diff(diff)
    }

    /// Parse a `git2::Diff` into structured `DiffFile` objects.
    ///
    /// This function processes a `git2::Diff` object and extracts file-level
    /// changes into a structured format (`DiffFile`). It uses the `foreach` method
    /// provided by `git2::Diff` to iterate over files, hunks, and lines in the diff.
    ///
    /// ### Design Choices
    /// - **Shared Mutable State:** The function uses `Rc<RefCell<HashMap>>` to manage
    ///   shared mutable state across the closures required by the `foreach` method.
    ///   This approach was chosen because:
    ///   - `Rc` allows multiple closures to share ownership of the state.
    ///   - `RefCell` enables interior mutability, allowing the state to be modified
    ///     within the closures while adhering to Rust's borrowing rules.
    /// - **Alternatives Considered:** Other synchronization primitives like `Mutex` or
    ///   `RwLock` were not used because they introduce unnecessary overhead in a
    ///   single-threaded context, which is the typical use case for this function.
    ///
    /// ### Callback-Based Parsing
    /// The `git2::Diff` API is callback-based, meaning that it invokes user-provided
    /// closures for each file, hunk, and line in the diff. This design necessitates
    /// the use of shared state (`Rc<RefCell<HashMap>>`) to accumulate results across
    /// multiple callbacks.
    ///
    /// ### Output
    /// The function returns a `Diff` object containing a list of `DiffFile` objects,
    /// each representing a file in the diff along with its content.
    fn parse_git_diff(diff: git2::Diff) -> color_eyre::Result<Diff> {
        // Use Rc and RefCell to share mutable state across closures
        // HashMap to store file paths and their content (path => content)
        let files_content = Rc::new(RefCell::new(HashMap::<String, String>::new()));

        // Clone references for each closure
        let files_content_file = Rc::clone(&files_content);
        let files_content_hunk = Rc::clone(&files_content);
        let files_content_line = Rc::clone(&files_content);

        // Use foreach to collect file information
        diff.foreach(
            &mut |delta, _progress| {
                // Extract file path from delta. If new_file and old_file are both present,
                // new_file takes precedence because that's the state after the commits.
                if let Some(new_file) = delta.new_file().path() {
                    let file_path = new_file.to_string_lossy().to_string();
                    files_content_file
                        .borrow_mut()
                        .entry(file_path)
                        .or_default();
                } else if let Some(old_file) = delta.old_file().path() {
                    let file_path = old_file.to_string_lossy().to_string();
                    files_content_file
                        .borrow_mut()
                        .entry(file_path)
                        .or_default();
                }
                true
            },
            None, // No binary callback needed
            Some(&mut |delta, _hunk| {
                // Collect hunk headers
                let file_path = if let Some(new_file) = delta.new_file().path() {
                    new_file.to_string_lossy().to_string()
                } else if let Some(old_file) = delta.old_file().path() {
                    old_file.to_string_lossy().to_string()
                } else {
                    return true;
                };

                if let Some(content) = files_content_hunk.borrow_mut().get_mut(&file_path) {
                    if let Ok(header) = std::str::from_utf8(_hunk.header()) {
                        content.push_str(header);
                    }
                }
                true
            }),
            Some(&mut |delta, _hunk, line| {
                // Collect line content
                let file_path = if let Some(new_file) = delta.new_file().path() {
                    new_file.to_string_lossy().to_string()
                } else if let Some(old_file) = delta.old_file().path() {
                    old_file.to_string_lossy().to_string()
                } else {
                    return true;
                };

                if let Some(content) = files_content_line.borrow_mut().get_mut(&file_path) {
                    // Add line origin character
                    match line.origin() {
                        '+' | '-' | ' ' => content.push(line.origin()),
                        _ => {}
                    }

                    // Add line content
                    match std::str::from_utf8(line.content()) {
                        Ok(line_content) => content.push_str(line_content),
                        Err(error) => {
                            eprintln!("UTF-8 conversion error: {error}");
                            content.push_str("[INVALID UTF-8]");
                        }
                    }
                }
                true
            }),
        )?;

        // Convert HashMap to Vec<DiffFile>
        let diff_files: Vec<DiffFile> = files_content
            .borrow()
            .iter()
            .map(|(path, content)| DiffFile {
                path: path.clone(),
                content: content.clone(),
            })
            .collect();

        // Sort files by path
        let mut sorted_diff_files = diff_files;
        sorted_diff_files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(Diff::from_files(sorted_diff_files))
    }

    /// Send loading event to start the actual loading process
    fn handle_git_branches_load(events: &mut EventHandler) {
        events.send(AppEvent::GitBranchesLoading);
        events.send(AppEvent::GitBranchesLoadingState(
            GitBranchesLoadingState::Loading,
        ));
    }

    /// Send loading event to start the diff loading process
    fn handle_git_diff_load(base_sha: &Arc<str>, target_sha: &Arc<str>, events: &mut EventHandler) {
        events.send(AppEvent::GitDiffLoading {
            base_sha: Arc::clone(base_sha),
            target_sha: Arc::clone(target_sha),
        });
        events.send(AppEvent::GitDiffLoadingState(GitDiffLoadingState::Loading));
    }

    /// Actually load Git branches from repository
    async fn handle_git_branches_loading(repo_path: &str, events: &mut EventHandler) {
        match Self::get_branches(repo_path) {
            Ok(branches) => {
                events.send(AppEvent::GitBranchesLoadingState(
                    GitBranchesLoadingState::Loaded(branches),
                ));
            }
            Err(error) => {
                events.send(AppEvent::GitBranchesLoadingState(
                    GitBranchesLoadingState::Error(error.to_string().into()),
                ));
            }
        }
    }

    /// Actually load Git diff from repository
    async fn handle_git_diff_loading(
        repo_path: &str,
        base_sha: &Arc<str>,
        target_sha: &Arc<str>,
        events: &mut EventHandler,
    ) {
        match Self::get_diff_between_shas(repo_path, base_sha, target_sha) {
            Ok(diff) => {
                events.send(AppEvent::GitDiffLoadingState(GitDiffLoadingState::Loaded(
                    Arc::new(diff),
                )));
            }
            Err(error) => {
                events.send(AppEvent::GitDiffLoadingState(GitDiffLoadingState::Error(
                    format!("Error generating diff: {error}").into(),
                )));
            }
        }
    }
}

impl ServiceHandler for GitService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        context: ServiceContext<'a>,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::GitBranchesLoad => {
                    Self::handle_git_branches_load(context.events);
                }
                AppEvent::GitBranchesLoading => {
                    Self::handle_git_branches_loading(context.repo_path, context.events).await;
                }
                AppEvent::GitDiffLoad {
                    base_sha,
                    target_sha,
                } => {
                    Self::handle_git_diff_load(base_sha, target_sha, context.events);
                }
                AppEvent::GitDiffLoading {
                    base_sha,
                    target_sha,
                } => {
                    Self::handle_git_diff_loading(
                        context.repo_path,
                        base_sha,
                        target_sha,
                        context.events,
                    )
                    .await;
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
    use std::fs;
    use tempfile::TempDir;

    fn create_test_git_repo() -> color_eyre::Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path();

        // Initialize git repo
        let repo = git2::Repository::init(repo_path)?;

        // Create initial commit
        let signature = git2::Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            // Create a dummy file
            let file_path = repo_path.join("README.md");
            fs::write(&file_path, b"# Test Repository")?;
            index.add_path(Path::new("README.md"))?;
            index.write()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;

        // Create additional branches
        let head_commit = repo.head()?.peel_to_commit()?;
        repo.branch("feature/test", &head_commit, false)?;
        repo.branch("develop", &head_commit, false)?;

        Ok(temp_dir)
    }

    #[test]
    fn test_get_branches() {
        let temp_dir = create_test_git_repo().unwrap();
        let branches = GitService::get_branches(temp_dir.path()).unwrap();

        // Should contain main/master, feature/test, and develop
        assert!(branches.len() >= 3);
        assert!(branches.contains(&"develop".to_string()));
        assert!(branches.contains(&"feature/test".to_string()));
        // The initial branch could be "main" or "master" depending on git config
        assert!(branches.contains(&"main".to_string()) || branches.contains(&"master".to_string()));
    }

    #[test]
    fn test_get_branches_sorted() {
        let temp_dir = create_test_git_repo().unwrap();
        let branches = GitService::get_branches(temp_dir.path()).unwrap();

        // Should be sorted alphabetically
        let mut sorted_branches = (*branches).to_vec();
        sorted_branches.sort();
        assert_eq!(branches, Arc::from(sorted_branches));
    }

    #[test]
    fn test_get_branches_nonexistent_repo() {
        let result = GitService::get_branches("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_branch_sha() {
        let temp_dir = create_test_git_repo().unwrap();

        // Test getting SHA for main/master branch
        let main_sha = GitService::get_branch_sha(temp_dir.path(), "main");
        let master_sha = GitService::get_branch_sha(temp_dir.path(), "master");

        // Either main or master should exist and have a SHA
        assert!(
            main_sha.is_ok() && main_sha.unwrap().is_some()
                || master_sha.is_ok() && master_sha.unwrap().is_some()
        );

        // Test getting SHA for feature branch
        let feature_sha = GitService::get_branch_sha(temp_dir.path(), "feature/test").unwrap();
        assert!(feature_sha.is_some());

        // Test getting SHA for develop branch
        let develop_sha = GitService::get_branch_sha(temp_dir.path(), "develop").unwrap();
        assert!(develop_sha.is_some());

        // Test non-existent branch
        let nonexistent_sha = GitService::get_branch_sha(temp_dir.path(), "nonexistent").unwrap();
        assert!(nonexistent_sha.is_none());
    }

    #[test]
    fn test_sha_methods_nonexistent_repo() {
        let result = GitService::get_branch_sha("/nonexistent/path", "main");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_diff_between_shas() {
        let temp_dir = create_test_git_repo().unwrap();
        let repo_path = temp_dir.path();

        // Get SHAs for the initial commit and a branch
        let main_sha = GitService::get_branch_sha(repo_path, "main")
            .unwrap()
            .or_else(|| GitService::get_branch_sha(repo_path, "master").unwrap())
            .expect("Neither main nor master branch found");
        let feature_sha = GitService::get_branch_sha(repo_path, "feature/test")
            .unwrap()
            .expect("feature/test branch not found");

        // Get diff between the same commit (should be empty)
        let diff_same = GitService::get_diff_between_shas(repo_path, &main_sha, &main_sha).unwrap();
        assert!(diff_same.is_empty());

        // Note: Since we created branches from the same commit, the diff will be empty
        // In a real scenario with different commits, this would show actual changes
        let diff_between =
            GitService::get_diff_between_shas(repo_path, &main_sha, &feature_sha).unwrap();
        assert!(diff_between.is_empty()); // Expected since both point to same commit
    }

    #[test]
    fn test_get_diff_with_actual_changes() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        let repo = git2::Repository::init(repo_path).unwrap();
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();

        // Create initial commit
        let initial_sha = {
            let mut index = repo.index().unwrap();
            let file_path = repo_path.join("file.txt");
            fs::write(&file_path, b"initial content").unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let commit_id = repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "Initial commit",
                    &tree,
                    &[],
                )
                .unwrap();
            commit_id.to_string()
        };

        // Create second commit with changes
        let second_sha = {
            let mut index = repo.index().unwrap();
            let file_path = repo_path.join("file.txt");
            fs::write(&file_path, b"modified content").unwrap();
            index.add_path(Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parent_commit = repo
                .find_commit(git2::Oid::from_str(&initial_sha).unwrap())
                .unwrap();
            let commit_id = repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "Second commit",
                    &tree,
                    &[&parent_commit],
                )
                .unwrap();
            commit_id.to_string()
        };

        // Get diff between commits
        let diff = GitService::get_diff_between_shas(repo_path, &initial_sha, &second_sha).unwrap();

        // Should have one file with changes
        assert!(!diff.is_empty());
        assert_eq!(diff.file_count(), 1);

        let file = &diff.files[0];
        assert_eq!(file.path, "file.txt");
        assert!(file.content.contains("-initial content"));
        assert!(file.content.contains("+modified content"));
    }

    #[test]
    fn test_get_diff_invalid_sha() {
        let temp_dir = create_test_git_repo().unwrap();
        let repo_path = temp_dir.path();

        // Test with invalid SHA
        let result =
            GitService::get_diff_between_shas(repo_path, "invalid_sha", "another_invalid_sha");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_diff_nonexistent_repo() {
        let result = GitService::get_diff_between_shas("/nonexistent/path", "sha1", "sha2");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_git_branches_load_event() {
        let database = crate::database::Database::new().await.unwrap();
        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        let app = crate::app::App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };
        // Handle GitBranchesLoad event
        GitService::handle_app_event(
            &AppEvent::GitBranchesLoad,
            ServiceContext {
                database: &app.database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent GitBranchesLoading and GitBranchesLoadingState(Loading) events
        assert!(events.has_pending_events());

        let event1 = events.try_recv().unwrap();
        assert!(matches!(
            *event1,
            crate::event::Event::App(AppEvent::GitBranchesLoading)
        ));

        let event2 = events.try_recv().unwrap();
        assert!(matches!(
            *event2,
            crate::event::Event::App(AppEvent::GitBranchesLoadingState(
                GitBranchesLoadingState::Loading
            ))
        ));

        // No more events should be pending
        assert!(!events.has_pending_events());
    }

    #[tokio::test]
    async fn test_handle_git_branches_loading_event_success() {
        let temp_dir = create_test_git_repo().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Change to test repo directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let database = crate::database::Database::new().await.unwrap();
        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        let app = crate::app::App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: temp_dir.path().to_string_lossy().to_string(),
        };
        // Handle GitBranchesLoading event
        GitService::handle_app_event(
            &AppEvent::GitBranchesLoading,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent GitBranchesLoadingState(Loaded) event
        assert!(events.has_pending_events());

        let event = events.try_recv().unwrap();
        if let crate::event::Event::App(AppEvent::GitBranchesLoadingState(
            GitBranchesLoadingState::Loaded(branches),
        )) = &*event
        {
            // Should contain the test branches
            assert!(branches.len() >= 3);
            assert!(branches.contains(&"develop".to_string()));
            assert!(branches.contains(&"feature/test".to_string()));
            assert!(
                branches.contains(&"main".to_string()) || branches.contains(&"master".to_string())
            );
        } else {
            panic!("Expected GitBranchesLoadingState::Loaded event, got: {event:?}");
        }

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap_or_else(
            // FIXLATER: Don't panic because in CI it doesn't work
            |e| println!("Failed to restore original directory: {e}"),
        );
    }

    #[tokio::test]
    async fn test_handle_git_branches_loading_event_error() {
        let original_dir = std::env::current_dir().unwrap();

        // Change to non-git directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let database = crate::database::Database::new().await.unwrap();
        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        let app = crate::app::App {
            running: true,
            events: crate::event::EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: temp_dir.path().to_string_lossy().to_string(),
        };
        // Handle GitBranchesLoading event
        GitService::handle_app_event(
            &AppEvent::GitBranchesLoading,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent GitBranchesLoadingState(Error) event
        assert!(events.has_pending_events());

        let event = events.try_recv().unwrap();
        if let crate::event::Event::App(AppEvent::GitBranchesLoadingState(
            GitBranchesLoadingState::Error(error),
        )) = &*event
        {
            // Should contain error message about not being a git repository
            assert!(error.contains("repository") || error.contains("not found"));
        } else {
            panic!("Expected GitBranchesLoadingState::Error event, got: {event:?}");
        }

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_git_branches_loading_state_default() {
        let state = GitBranchesLoadingState::default();
        assert_eq!(state, GitBranchesLoadingState::Init);
    }

    #[tokio::test]
    async fn test_git_branches_loading_state_clone() {
        let branches = vec!["main".to_string(), "develop".to_string()];
        let state = GitBranchesLoadingState::Loaded(branches.clone().into());
        let cloned_state = state.clone();

        assert_eq!(state, cloned_state);

        if let (
            GitBranchesLoadingState::Loaded(original),
            GitBranchesLoadingState::Loaded(cloned),
        ) = (state, cloned_state)
        {
            // Arc should point to the same data
            assert_eq!(original, cloned);
        }
    }

    #[tokio::test]
    async fn test_git_branches_loading_state_debug() {
        let state_init = GitBranchesLoadingState::Init;
        let state_loading = GitBranchesLoadingState::Loading;
        let state_loaded = GitBranchesLoadingState::Loaded(vec!["main".to_string()].into());
        let state_error = GitBranchesLoadingState::Error("test error".into());

        assert!(format!("{state_init:?}").contains("Init"));
        assert!(format!("{state_loading:?}").contains("Loading"));
        assert!(format!("{state_loaded:?}").contains("Loaded"));
        assert!(format!("{state_error:?}").contains("Error"));
    }

    #[tokio::test]
    async fn test_handle_git_branches_load_function() {
        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        // Call the private function through ServiceHandler
        GitService::handle_git_branches_load(&mut events);

        // Should have sent GitBranchesLoading and GitBranchesLoadingState(Loading) events
        assert!(events.has_pending_events());

        let event1 = events.try_recv().unwrap();
        assert!(matches!(
            *event1,
            crate::event::Event::App(AppEvent::GitBranchesLoading)
        ));

        let event2 = events.try_recv().unwrap();
        assert!(matches!(
            *event2,
            crate::event::Event::App(AppEvent::GitBranchesLoadingState(
                GitBranchesLoadingState::Loading
            ))
        ));

        // No more events should be pending
        assert!(!events.has_pending_events());
    }

    #[tokio::test]
    async fn test_handle_git_branches_loading_function_success() {
        let temp_dir = create_test_git_repo().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Change to test repo directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        // Call the private function
        GitService::handle_git_branches_loading(&temp_dir.path().to_string_lossy(), &mut events)
            .await;

        // Should have sent GitBranchesLoadingState(Loaded) event
        assert!(events.has_pending_events());

        let event = events.try_recv().unwrap();
        if let crate::event::Event::App(AppEvent::GitBranchesLoadingState(
            GitBranchesLoadingState::Loaded(branches),
        )) = &*event
        {
            // Should contain the test branches
            assert!(branches.len() >= 3);
            assert!(branches.contains(&"develop".to_string()));
            assert!(branches.contains(&"feature/test".to_string()));
        } else {
            panic!("Expected GitBranchesLoadingState::Loaded event, got: {event:?}");
        }

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_handle_git_branches_loading_function_error() {
        let original_dir = std::env::current_dir().unwrap();

        // Change to non-git directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let mut events = crate::event::EventHandler::new_for_test();

        // Initially no events
        assert!(!events.has_pending_events());

        // Call the private function
        GitService::handle_git_branches_loading(&temp_dir.path().to_string_lossy(), &mut events)
            .await;

        // Should have sent GitBranchesLoadingState(Error) event
        assert!(events.has_pending_events());

        let event = events.try_recv().unwrap();
        if let crate::event::Event::App(AppEvent::GitBranchesLoadingState(
            GitBranchesLoadingState::Error(error),
        )) = &*event
        {
            // Should contain error message
            assert!(!error.is_empty());
        } else {
            panic!("Expected GitBranchesLoadingState::Error event, got: {event:?}");
        }

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}
