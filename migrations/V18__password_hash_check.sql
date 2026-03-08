-- Defense-in-depth: ensure the password column always holds an Argon2id hash
-- (≥50 chars), preventing accidental plaintext storage.
ALTER TABLE users
    ADD CONSTRAINT chk_users_password_min_length CHECK (length(password) >= 50);
