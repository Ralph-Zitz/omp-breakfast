-- V13: Add pickup_user_id to team orders.
--
-- Each order can optionally have a designated pickup user — the team member
-- responsible for physically collecting the order. The pickup user must belong
-- to the same team, but that constraint is enforced at the application layer
-- (handler RBAC) rather than via a database trigger, consistent with the
-- existing team-membership enforcement pattern.
--
-- Once a pickup user is assigned, only a global Admin or Team Admin for the
-- order's team may change the assignment.

ALTER TABLE teamorders
    ADD COLUMN IF NOT EXISTS pickup_user_id uuid REFERENCES users (user_id) ON DELETE SET NULL;

-- FK index for efficient joins and cascaded delete scans.
CREATE INDEX IF NOT EXISTS idx_teamorders_pickup_user ON teamorders (pickup_user_id)
    WHERE pickup_user_id IS NOT NULL;
