-- V15: Change cascading FK deletes to RESTRICT for defense-in-depth.
--
-- #669: memberof.memberof_user_id ON DELETE CASCADE silently removes all team
--       memberships when a user is deleted, bypassing guard_last_admin_membership.
--       The application now explicitly removes memberships in a transaction
--       before deleting the user, so the cascade is no longer needed.
--
-- #670: teamorders.teamorders_team_id and orders.orders_team_id ON DELETE CASCADE
--       would silently destroy all order history when a team is deleted. The
--       delete_team handler already returns 409 when a team has orders, so
--       RESTRICT adds defense-in-depth at the DB level.

-- #669: memberof.memberof_user_id CASCADE → RESTRICT
ALTER TABLE memberof DROP CONSTRAINT IF EXISTS memberof_memberof_user_id_fkey;
ALTER TABLE memberof
    ADD CONSTRAINT memberof_memberof_user_id_fkey
    FOREIGN KEY (memberof_user_id) REFERENCES users (user_id) ON DELETE RESTRICT;

-- #670: teamorders.teamorders_team_id CASCADE → RESTRICT
ALTER TABLE teamorders DROP CONSTRAINT IF EXISTS teamorders_teamorders_team_id_fkey;
ALTER TABLE teamorders
    ADD CONSTRAINT teamorders_teamorders_team_id_fkey
    FOREIGN KEY (teamorders_team_id) REFERENCES teams (team_id) ON DELETE RESTRICT;

-- #670: orders.orders_team_id CASCADE → RESTRICT
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_orders_team_id_fkey;
ALTER TABLE orders
    ADD CONSTRAINT orders_orders_team_id_fkey
    FOREIGN KEY (orders_team_id) REFERENCES teams (team_id) ON DELETE RESTRICT;
