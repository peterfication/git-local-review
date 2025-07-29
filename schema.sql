CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);
CREATE TABLE reviews (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
, base_branch TEXT NOT NULL, target_branch TEXT NOT NULL, base_sha TEXT, target_sha TEXT);
CREATE TABLE file_views (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    review_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE,
    UNIQUE(review_id, file_path)
);
CREATE TABLE sqlite_sequence(name,seq);
CREATE INDEX idx_file_views_review_id ON file_views(review_id);
CREATE INDEX idx_file_views_review_file ON file_views(review_id, file_path);
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    review_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line_number INTEGER,  -- NULL for file-level comments
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (review_id) REFERENCES reviews (id) ON DELETE CASCADE
);
CREATE INDEX idx_comments_review_id ON comments (review_id);
CREATE INDEX idx_comments_file_path ON comments (review_id, file_path);
CREATE INDEX idx_comments_line ON comments (review_id, file_path, line_number);
CREATE INDEX idx_comments_created_at ON comments (created_at DESC);
