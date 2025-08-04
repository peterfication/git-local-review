-- Add resolved column to comments table
ALTER TABLE comments ADD COLUMN resolved BOOLEAN NOT NULL DEFAULT FALSE;

-- Create index on resolved column for efficient queries
CREATE INDEX idx_comments_resolved ON comments(resolved);

-- Create composite index for efficient queries on review_id and resolved status
CREATE INDEX idx_comments_review_resolved ON comments(review_id, resolved);
