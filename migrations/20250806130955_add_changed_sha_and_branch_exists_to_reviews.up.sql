-- Add columns to track changed SHAs and branch existence
ALTER TABLE reviews ADD COLUMN base_sha_changed TEXT;
ALTER TABLE reviews ADD COLUMN target_sha_changed TEXT;
ALTER TABLE reviews ADD COLUMN base_branch_exists BOOLEAN;
ALTER TABLE reviews ADD COLUMN target_branch_exists BOOLEAN;
