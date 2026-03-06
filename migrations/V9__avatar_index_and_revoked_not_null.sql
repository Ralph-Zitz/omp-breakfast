-- Add missing index on users.avatar_id FK column for JOIN/delete performance.
CREATE INDEX IF NOT EXISTS idx_users_avatar ON users (avatar_id);

-- Enforce NOT NULL on token_blacklist.revoked_at to prevent rows without a
-- revocation timestamp. Backfill any existing NULLs with the current time.
UPDATE token_blacklist SET revoked_at = CURRENT_TIMESTAMP WHERE revoked_at IS NULL;
ALTER TABLE token_blacklist ALTER COLUMN revoked_at SET NOT NULL;
