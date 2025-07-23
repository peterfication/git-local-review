-- NOTE: This is not safe but at the current project state it is acceptable.
ALTER TABLE reviews ADD COLUMN title TEXT NOT NULL;
