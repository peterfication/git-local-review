CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line_number INTEGER,  -- NULL for file-level comments
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (review_id) REFERENCES reviews (id) ON DELETE CASCADE
);

-- Index for efficient queries
CREATE INDEX idx_comments_review_id ON comments (review_id);
CREATE INDEX idx_comments_file_path ON comments (review_id, file_path);
CREATE INDEX idx_comments_line ON comments (review_id, file_path, line_number);
CREATE INDEX idx_comments_created_at ON comments (created_at DESC);
