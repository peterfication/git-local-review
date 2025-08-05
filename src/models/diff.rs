use std::sync::Arc;

/// Represents a single file in a Git diff
#[derive(Debug, Clone, PartialEq)]
pub struct DiffFile {
    /// Path to the file being changed
    pub path: String,
    /// Diff content for this specific file
    pub content: String,
}

/// Represents a complete Git diff with structured data
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    /// List of files changed in this diff
    pub files: Arc<[DiffFile]>,
}

impl Diff {
    /// Create a new empty diff
    pub fn empty() -> Self {
        Self {
            files: Arc::new([]),
        }
    }

    /// Create a diff from a vector of files
    pub fn from_files(files: Vec<DiffFile>) -> Self {
        Self {
            files: files.into(),
        }
    }

    /// Check if the diff is empty (no files)
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the number of files in the diff
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for Diff {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_file_equality() {
        let file1 = DiffFile {
            path: "foo.txt".to_string(),
            content: "diff content".to_string(),
        };
        let file2 = DiffFile {
            path: "foo.txt".to_string(),
            content: "diff content".to_string(),
        };
        let file3 = DiffFile {
            path: "bar.txt".to_string(),
            content: "other diff".to_string(),
        };
        assert_eq!(file1, file2);
        assert_ne!(file1, file3);
    }

    #[test]
    fn test_diff_empty() {
        let diff = Diff::empty();
        assert!(diff.is_empty());
        assert_eq!(diff.file_count(), 0);
        assert_eq!(diff.files.len(), 0);
    }

    #[test]
    fn test_diff_from_files() {
        let files = vec![
            DiffFile {
                path: "a.txt".to_string(),
                content: "diff a".to_string(),
            },
            DiffFile {
                path: "b.txt".to_string(),
                content: "diff b".to_string(),
            },
        ];
        let diff = Diff::from_files(files.clone());
        assert!(!diff.is_empty());
        assert_eq!(diff.file_count(), 2);
        assert_eq!(&*diff.files, files.as_slice());
    }

    #[test]
    fn test_diff_default() {
        let diff = Diff::default();
        assert!(diff.is_empty());
        assert_eq!(diff.file_count(), 0);
    }
}
