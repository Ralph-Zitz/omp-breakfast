-- V12: Drop unused index, add NOT NULL to orders_team_id, remove session-scoped SET timezone.
--
-- #374 — idx_teamorders_id_due is never used by any query (all order queries
--        filter by team_id alone or by primary key). We have
--        idx_teamorders_team_created from V6 for the main query pattern.
-- #325 — orders.orders_team_id is uuid (nullable) despite a trigger that
--        prevents NULL values. Make the constraint explicit.

-- #374: Drop unused covering index on (teamorders_team_id, duedate).
DROP INDEX IF EXISTS idx_teamorders_id_due;

-- #325: Make orders_team_id NOT NULL (the trigger already prevents NULLs,
-- but an explicit column constraint makes the schema self-documenting).
ALTER TABLE orders ALTER COLUMN orders_team_id SET NOT NULL;
