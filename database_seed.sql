/* ═══════════════════════════════════════════════════════════════════════════
   ⚠  WARNING: DO NOT RUN IN PRODUCTION ⚠

   This file contains DEVELOPMENT/TESTING seed data with hardcoded passwords
   and identical Argon2 hashes. Running this against a production database
   would create accounts with known credentials.
   ═══════════════════════════════════════════════════════════════════════════ */

/* ═══════════════════════════════════════════════════════════════════════════
   SEED DATA for DEVELOPMENT and TESTING

   This script inserts test/development data into an existing schema.
   It assumes the schema has already been created via:
     - Refinery migrations (migrations/V1__initial_schema.sql) in production, or
     - database.sql (full DROP/CREATE) for manual dev database resets

   Used by docker-compose's postgres-setup service to populate the database
   after migrations have run.
   ═══════════════════════════════════════════════════════════════════════════ */

-- Seed teams
INSERT INTO teams (tname, descr)
  VALUES ('League of Cool Coders', 'LEGO LPAF Team')
  ON CONFLICT (tname) DO NOTHING;

INSERT INTO teams (tname, descr)
  VALUES ('Pixel Bakers', 'PoweredUp Team')
  ON CONFLICT (tname) DO NOTHING;

-- Seed roles
INSERT INTO roles (title)
  VALUES ('Admin')
  ON CONFLICT (title) DO NOTHING;

INSERT INTO roles (title)
  VALUES ('Team Admin')
  ON CONFLICT (title) DO NOTHING;

INSERT INTO roles (title)
  VALUES ('Member')
  ON CONFLICT (title) DO NOTHING;

INSERT INTO roles (title)
  VALUES ('Guest')
  ON CONFLICT (title) DO NOTHING;

-- Seed users
INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('admin', 'root', 'admin@admin.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I')
  ON CONFLICT (email) DO NOTHING;

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U1_F', 'U1_L', 'U1_F.U1_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I')
  ON CONFLICT (email) DO NOTHING;

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U2_F', 'U2_L', 'U2_F.U2_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I')
  ON CONFLICT (email) DO NOTHING;

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U3_F', 'U3_L', 'U3_F.U3_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I')
  ON CONFLICT (email) DO NOTHING;

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U4_F', 'U4_L', 'U4_F.U4_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I')
  ON CONFLICT (email) DO NOTHING;

-- Seed items
INSERT INTO items (descr, price)
  VALUES ('håndværker', '9.00')
  ON CONFLICT (descr) DO NOTHING;

INSERT INTO items (descr, price)
  VALUES ('spansk u. birkes', '9.00')
  ON CONFLICT (descr) DO NOTHING;

INSERT INTO items (descr, price)
  VALUES ('spansk m. birkes', '9.00')
  ON CONFLICT (descr) DO NOTHING;

INSERT INTO items (descr, price)
  VALUES ('skagenslab', '9.00')
  ON CONFLICT (descr) DO NOTHING;

-- Seed memberships
INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'admin'
  AND teams.tname = 'League of Cool Coders'
  AND roles.title = 'Admin'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'U1_F'
  AND teams.tname = 'League of Cool Coders'
  AND roles.title = 'Member'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'U2_F'
  AND teams.tname = 'League of Cool Coders'
  AND roles.title = 'Member'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'U3_F'
  AND teams.tname = 'League of Cool Coders'
  AND roles.title = 'Member'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'U4_F'
  AND teams.tname = 'League of Cool Coders'
  AND roles.title = 'Team Admin'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

INSERT INTO memberof (memberof_team_id, memberof_user_id, memberof_role_id)
SELECT
  team_id,
  user_id,
  role_id
FROM
  users,
  teams,
  roles
WHERE
  firstname = 'U4_F'
  AND teams.tname = 'Pixel Bakers'
  AND roles.title = 'Member'
ON CONFLICT (memberof_team_id, memberof_user_id) DO NOTHING;

-- Seed team orders
INSERT INTO teamorders (teamorders_team_id, teamorders_user_id)
SELECT
  teams.team_id,
  users.user_id
FROM
  teams,
  users
WHERE
  teams.tname = 'League of Cool Coders'
  AND users.email = 'admin@admin.com'
ON CONFLICT DO NOTHING;

-- Seed orders
INSERT INTO orders (orders_teamorders_id, orders_item_id, orders_team_id, amt)
SELECT
  teamorders_id,
  item_id,
  team_id,
  3
FROM
  items,
  teams,
  teamorders
WHERE
  items.descr = 'håndværker'
  AND teams.team_id = teamorders.teamorders_team_id
  AND teams.tname = 'League of Cool Coders'
ON CONFLICT (orders_teamorders_id, orders_item_id) DO NOTHING;

INSERT INTO orders (orders_teamorders_id, orders_item_id, orders_team_id, amt)
SELECT
  teamorders_id,
  item_id,
  team_id,
  2
FROM
  items,
  teams,
  teamorders
WHERE
  items.descr = 'skagenslab'
  AND teams.team_id = teamorders.teamorders_team_id
  AND teams.tname = 'League of Cool Coders'
ON CONFLICT (orders_teamorders_id, orders_item_id) DO NOTHING;
