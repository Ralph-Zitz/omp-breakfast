-- V5: Fix users trigger and add missing NOT NULL constraints
--
-- #195: The update_users_changed_at trigger fires on INSERT OR UPDATE,
--       but the INSERT path is unnecessary because DEFAULT CURRENT_TIMESTAMP
--       already sets the `changed` column on insert. All other table
--       triggers use BEFORE UPDATE only.
--
-- #202: teamorders.teamorders_user_id allows NULL but no code path ever
--       creates an order without setting this field.
--
-- #229: memberof.joined allows NULL but has DEFAULT CURRENT_TIMESTAMP.
--       V4 hardening added NOT NULL to created/changed but missed joined.

-- Fix #195: Change users trigger from INSERT OR UPDATE to UPDATE only
CREATE OR REPLACE TRIGGER update_users_changed_at
  BEFORE UPDATE ON users
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

-- Fix #202: Make teamorders_user_id NOT NULL
UPDATE teamorders SET teamorders_user_id = (
  SELECT user_id FROM users LIMIT 1
) WHERE teamorders_user_id IS NULL;

ALTER TABLE teamorders ALTER COLUMN teamorders_user_id SET NOT NULL;

-- Fix #229: Make memberof.joined NOT NULL
UPDATE memberof SET joined = CURRENT_TIMESTAMP WHERE joined IS NULL;

ALTER TABLE memberof ALTER COLUMN joined SET NOT NULL;
