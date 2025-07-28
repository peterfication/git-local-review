-- Create file_views table to track which files have been viewed for each review
CREATE TABLE file_views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    review_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE,
    UNIQUE(review_id, file_path)
);

-- Create index for efficient lookups
CREATE INDEX idx_file_views_review_id ON file_views(review_id);
CREATE INDEX idx_file_views_review_file ON file_views(review_id, file_path);
