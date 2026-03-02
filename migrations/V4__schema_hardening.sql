-- V4: Schema hardening — NOT NULL constraints, missing indexes, timestamps.
--
-- #131 — Add missing FK index on orders.orders_item_id
-- #133 — Add NOT NULL to created/changed columns across all entity tables
-- #134 — Add NOT NULL to items.price (with DEFAULT 0 for existing NULLs)
-- #135 — Add NOT NULL DEFAULT 1 to orders.amt (fix existing NULLs first)
-- #136 — Add created/changed timestamp columns to orders table
-- #162 — Add changed timestamp column to memberof table

-- #131: Index for FK RESTRICT lookups on item deletion.
-- The composite PK (orders_teamorders_id, orders_item_id) cannot serve this
-- lookup because orders_item_id is the second column.
CREATE INDEX IF NOT EXISTS idx_orders_item ON orders (orders_item_id);

-- #133: Ensure timestamp columns cannot be NULL.
-- All rows already have DEFAULT CURRENT_TIMESTAMP values; any NULLs from
-- explicit inserts are filled before adding the constraint.
UPDATE users SET created = CURRENT_TIMESTAMP WHERE created IS NULL;
UPDATE users SET changed = CURRENT_TIMESTAMP WHERE changed IS NULL;
ALTER TABLE users ALTER COLUMN created SET NOT NULL;
ALTER TABLE users ALTER COLUMN changed SET NOT NULL;

UPDATE teams SET created = CURRENT_TIMESTAMP WHERE created IS NULL;
UPDATE teams SET changed = CURRENT_TIMESTAMP WHERE changed IS NULL;
ALTER TABLE teams ALTER COLUMN created SET NOT NULL;
ALTER TABLE teams ALTER COLUMN changed SET NOT NULL;

UPDATE roles SET created = CURRENT_TIMESTAMP WHERE created IS NULL;
UPDATE roles SET changed = CURRENT_TIMESTAMP WHERE changed IS NULL;
ALTER TABLE roles ALTER COLUMN created SET NOT NULL;
ALTER TABLE roles ALTER COLUMN changed SET NOT NULL;

UPDATE items SET created = CURRENT_TIMESTAMP WHERE created IS NULL;
UPDATE items SET changed = CURRENT_TIMESTAMP WHERE changed IS NULL;
ALTER TABLE items ALTER COLUMN created SET NOT NULL;
ALTER TABLE items ALTER COLUMN changed SET NOT NULL;

UPDATE teamorders SET created = CURRENT_TIMESTAMP WHERE created IS NULL;
UPDATE teamorders SET changed = CURRENT_TIMESTAMP WHERE changed IS NULL;
ALTER TABLE teamorders ALTER COLUMN created SET NOT NULL;
ALTER TABLE teamorders ALTER COLUMN changed SET NOT NULL;

-- #134: items.price must not be NULL so order totals can be computed.
UPDATE items SET price = 0 WHERE price IS NULL;
ALTER TABLE items ALTER COLUMN price SET NOT NULL;

-- #135: orders.amt must not be NULL; default to 1 for new rows.
UPDATE orders SET amt = 1 WHERE amt IS NULL;
ALTER TABLE orders ALTER COLUMN amt SET NOT NULL;
ALTER TABLE orders ALTER COLUMN amt SET DEFAULT 1;

-- #136: Add audit timestamps to orders table, consistent with other entity tables.
ALTER TABLE orders ADD COLUMN IF NOT EXISTS created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE orders ADD COLUMN IF NOT EXISTS changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP;

CREATE OR REPLACE TRIGGER update_orders_changed_at
  BEFORE UPDATE ON orders
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp();

-- #162: Add changed timestamp to memberof for role-change audit trail.
ALTER TABLE memberof ADD COLUMN IF NOT EXISTS changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP;

CREATE OR REPLACE TRIGGER update_memberof_changed_at
  BEFORE UPDATE ON memberof
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp();
