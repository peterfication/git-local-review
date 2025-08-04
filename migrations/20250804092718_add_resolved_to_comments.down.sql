-- Remove indexes related to resolved column
DROP INDEX IF EXISTS idx_comments_review_resolved;
DROP INDEX IF EXISTS idx_comments_resolved;

-- Remove resolved column from comments table
ALTER TABLE comments DROP COLUMN resolved;
