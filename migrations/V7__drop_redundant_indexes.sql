-- V7: Drop redundant indexes that duplicate UNIQUE constraint auto-indexes.
--
-- The UNIQUE constraints on users(email) and teams(tname) already create
-- implicit B-tree indexes enforcing uniqueness. The manually created
-- idx_users_email and idx_teams_name are therefore redundant — the planner
-- will never prefer them over the constraint-backed indexes, and they incur
-- extra write overhead on INSERT/UPDATE.
DROP INDEX IF EXISTS idx_users_email;
DROP INDEX IF EXISTS idx_teams_name;
