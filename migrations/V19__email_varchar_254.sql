-- Expand email column from VARCHAR(75) to VARCHAR(254) per RFC 5321.
-- Also update the associated CHECK constraint to match.
ALTER TABLE users ALTER COLUMN email TYPE varchar(254);

ALTER TABLE users DROP CONSTRAINT IF EXISTS chk_users_email_length;
ALTER TABLE users
    ADD CONSTRAINT chk_users_email_length CHECK (char_length(email) <= 254);
