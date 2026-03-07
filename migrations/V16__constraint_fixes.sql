-- V16: Defense-in-depth constraint fixes.
--
-- #674: users.email CHECK(≤255) is redundant — column is varchar(75).
--       Change to CHECK(≤75) to match the column type.
--
-- #675: items.price CHECK(≥0) allows zero-price items, but the API
--       validator requires price > 0. Tighten the DB constraint.
--
-- #676: memberof.memberof_team_id ON DELETE CASCADE silently removes
--       memberships when a team is deleted. Change to RESTRICT so the
--       application's 409 guard is enforced at the DB level too.

-- #674: Fix email CHECK to match varchar(75)
ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_email_length;
ALTER TABLE users
    ADD CONSTRAINT chk_users_email_length CHECK (char_length(email) <= 75);

-- #675: items.price must be strictly positive
ALTER TABLE items DROP CONSTRAINT IF EXISTS items_price_check;
ALTER TABLE items
    ADD CONSTRAINT items_price_check CHECK (price > 0);

-- #676: memberof.memberof_team_id CASCADE → RESTRICT
ALTER TABLE memberof DROP CONSTRAINT IF EXISTS memberof_memberof_team_id_fkey;
ALTER TABLE memberof
    ADD CONSTRAINT memberof_memberof_team_id_fkey
    FOREIGN KEY (memberof_team_id) REFERENCES teams (team_id) ON DELETE RESTRICT;
