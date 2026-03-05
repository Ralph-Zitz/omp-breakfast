-- Avatar support: create avatars table and add avatar_id FK to users.
--
-- Avatars are pre-populated from the minifigs/ directory on first server
-- startup (see server.rs seed_avatars). Users can pick an avatar from the
-- available set via the profile page.

CREATE TABLE IF NOT EXISTS avatars (
    avatar_id uuid PRIMARY KEY,
    name text NOT NULL,
    data bytea NOT NULL,
    content_type text NOT NULL DEFAULT 'image/png',
    created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name)
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_id uuid REFERENCES avatars(avatar_id) ON DELETE SET NULL;
