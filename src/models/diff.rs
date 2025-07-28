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
