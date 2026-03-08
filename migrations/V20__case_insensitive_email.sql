-- Enforce case-insensitive email uniqueness and normalize existing emails.
--
-- 1. Lowercase all existing email addresses so the new unique index won't
--    conflict with mixed-case duplicates.
-- 2. Drop the old case-sensitive UNIQUE constraint.
-- 3. Create a functional unique index on LOWER(email) so that
--    'User@Example.com' and 'user@example.com' are treated as the same address.

UPDATE users SET email = LOWER(email) WHERE email <> LOWER(email);

ALTER TABLE users DROP CONSTRAINT IF EXISTS users_email_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_unique_lower ON users (LOWER(email));
