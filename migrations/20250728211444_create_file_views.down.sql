-- Drop file_views table and its indexes
DROP INDEX IF EXISTS idx_file_views_review_file;
DROP INDEX IF EXISTS idx_file_views_review_id;
DROP TABLE IF EXISTS file_views;
