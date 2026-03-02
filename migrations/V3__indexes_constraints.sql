-- V3: Add missing indexes, drop redundant index, fix FK action, add NOT NULL constraint.
--
-- #101 — Add missing FK index on teamorders.teamorders_user_id
-- #102 — Add missing FK index on orders.orders_team_id
-- #103 — Drop redundant idx_orders_tid (leading column of composite PK)
-- #104 — Change ON DELETE CASCADE to RESTRICT on orders.orders_item_id
-- #105 — Add NOT NULL constraint on memberof.memberof_role_id

-- #101: Index for FK lookups and ON DELETE RESTRICT checks on user deletion
CREATE INDEX IF NOT EXISTS idx_teamorders_user ON teamorders (teamorders_user_id);

-- #102: Index for queries filtering on orders_team_id
CREATE INDEX IF NOT EXISTS idx_orders_team ON orders (orders_team_id);

-- #103: Drop redundant single-column index (composite PK already covers it)
DROP INDEX IF EXISTS idx_orders_tid;

-- #104: Prevent silent deletion of order history when an item is deleted.
-- Drop the existing CASCADE FK and re-add with RESTRICT.
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_orders_item_id_fkey;
ALTER TABLE orders
    ADD CONSTRAINT orders_orders_item_id_fkey
    FOREIGN KEY (orders_item_id) REFERENCES items (item_id) ON DELETE RESTRICT;

-- #105: Ensure every membership has a role (prevents NULL role bypassing RBAC).
-- Any existing NULL rows would need to be fixed before this runs.
ALTER TABLE memberof ALTER COLUMN memberof_role_id SET NOT NULL;
