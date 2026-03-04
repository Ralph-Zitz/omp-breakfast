-- V6: Tighten orders.amt constraint and add composite index for team orders.
--
-- #274 — DB CHECK allows amt >= 0 but API validation requires amt >= 1.
--        Align the database constraint with the application rule.
-- #275 — get_team_orders queries WHERE teamorders_team_id = $1 ORDER BY
--        created DESC without a covering composite index. The existing
--        idx_teamorders_id_due covers (team_id, duedate), not (team_id, created).

-- #274: Tighten amt constraint from >= 0 to >= 1.
-- First fix any existing zero-quantity rows (should not exist in practice).
UPDATE orders SET amt = 1 WHERE amt = 0;
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_amt_check;
ALTER TABLE orders ADD CONSTRAINT orders_amt_check CHECK (amt >= 1);

-- #275: Add composite index for the most common team-orders query pattern.
CREATE INDEX IF NOT EXISTS idx_teamorders_team_created
    ON teamorders (teamorders_team_id, created DESC);
