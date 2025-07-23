use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};
use crate::services::ServiceHandler;

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
}

impl ServiceHandler for GitService {
    fn handle_app_event<'a>(
        _event: &'a AppEvent,
        _database: &'a Database,
        _events: &'a mut EventHandler,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // GitService doesn't handle any events currently
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
        let mut sorted_branches = (*branches).to_vec().clone();
        sorted_branches.sort();
        assert_eq!(branches, Arc::from(sorted_branches));
    }

    #[test]
    fn test_get_branches_nonexistent_repo() {
        let result = GitService::get_branches("/nonexistent/path");
        assert!(result.is_err());
    }
}
