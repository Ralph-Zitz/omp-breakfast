-- Switch UUID primary key defaults from uuid_generate_v4() (random) to
-- uuidv7() (time-ordered), aligning the schema with the application code
-- which already generates Uuid::now_v7() in Rust.
--
-- Requires PostgreSQL 18+ (uuidv7() is a built-in core function).
-- Existing rows are unaffected — only new rows inserted without an
-- explicit ID will use the new default.

ALTER TABLE users ALTER COLUMN user_id SET DEFAULT uuidv7();
ALTER TABLE teams ALTER COLUMN team_id SET DEFAULT uuidv7();
ALTER TABLE roles ALTER COLUMN role_id SET DEFAULT uuidv7();
ALTER TABLE items ALTER COLUMN item_id SET DEFAULT uuidv7();
ALTER TABLE teamorders ALTER COLUMN teamorders_id SET DEFAULT uuidv7();
