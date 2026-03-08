-- Add upper bound to orders.amt CHECK constraint.
--
-- The API validates amt between 1 and 10000, but the DB only enforced amt >= 1.
-- Align the database constraint with the application rule.

-- Cap any existing rows that exceed the new upper bound (should not exist in practice).
UPDATE orders SET amt = 10000 WHERE amt > 10000;

ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_amt_check;
ALTER TABLE orders ADD CONSTRAINT orders_amt_check CHECK (amt >= 1 AND amt <= 10000);
