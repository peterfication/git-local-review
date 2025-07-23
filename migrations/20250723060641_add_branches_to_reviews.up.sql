-- NOTE: This is not safe but at the current project state it is acceptable.
ALTER TABLE reviews ADD COLUMN base_branch TEXT NOT NULL;
ALTER TABLE reviews ADD COLUMN target_branch TEXT NOT NULL;
