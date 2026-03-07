-- Prevent teamorders.teamorders_team_id from being updated when child orders
-- rows exist, which would cause the denormalized orders.orders_team_id to
-- drift out of sync.

CREATE OR REPLACE FUNCTION guard_teamorders_team_id_change ()
  RETURNS TRIGGER
  AS $guard_teamorders_team_id_change$
BEGIN
  IF OLD.teamorders_team_id IS DISTINCT FROM NEW.teamorders_team_id THEN
    IF EXISTS (
      SELECT 1 FROM orders WHERE orders_teamorders_id = NEW.teamorders_id
    ) THEN
      RAISE EXCEPTION 'Cannot change team on order % — it has existing line items',
        NEW.teamorders_id;
    END IF;
  END IF;
  RETURN NEW;
END;
$guard_teamorders_team_id_change$
LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER guard_teamorders_team_id
  BEFORE UPDATE ON teamorders
  FOR EACH ROW
  EXECUTE FUNCTION guard_teamorders_team_id_change ();
