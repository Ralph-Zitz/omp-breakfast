-- Add CHECK constraints on user text columns to match Rust validation rules.
-- Defense-in-depth: these mirror the `validator` crate limits on the API layer.

ALTER TABLE users
  ADD CONSTRAINT chk_users_firstname_length CHECK (char_length(firstname) <= 50);

ALTER TABLE users
  ADD CONSTRAINT chk_users_lastname_length CHECK (char_length(lastname) <= 50);

ALTER TABLE users
  ADD CONSTRAINT chk_users_email_length CHECK (char_length(email) <= 255);
