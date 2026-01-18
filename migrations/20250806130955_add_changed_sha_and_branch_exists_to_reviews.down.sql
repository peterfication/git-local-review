-- Remove columns for changed SHAs and branch existence
ALTER TABLE reviews DROP COLUMN base_sha_changed;
ALTER TABLE reviews DROP COLUMN target_sha_changed;
ALTER TABLE reviews DROP COLUMN base_branch_exists;
ALTER TABLE reviews DROP COLUMN target_branch_exists;
