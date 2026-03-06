/* ═══════════════════════════════════════════════════════════════════════════
   WARNING: This script is for DEVELOPMENT and TESTING only - MANUAL RESET

   ⚠️  DEPRECATED for docker-compose workflows ⚠️

   docker-compose now uses:
     - migrations/V1__initial_schema.sql through V7 (via Refinery migrations)
     - database_seed.sql (for test data)

   This file is kept ONLY for manual database resets during development:
     $ PGPASSWORD=actix psql -h localhost -p 5432 -U actix actix < database.sql

   It drops ALL tables and recreates them from scratch, destroying all data.
   DO NOT run this against a production database.

   Production deployments use refinery migrations (see migrations/ directory).
   ═══════════════════════════════════════════════════════════════════════════ */

/* Drop Tables */
DROP TABLE IF EXISTS memberof;

DROP TABLE IF EXISTS orders;

DROP TABLE IF EXISTS teamorders;

DROP TABLE IF EXISTS avatars;

DROP TABLE IF EXISTS users;

DROP TABLE IF EXISTS teams;

DROP TABLE IF EXISTS roles;

DROP TABLE IF EXISTS items;

DROP TABLE IF EXISTS token_blacklist;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

/* Set Timezone */
SET timezone = 'Europe/Copenhagen';

/* Users table */
CREATE TABLE users (
  user_id uuid DEFAULT uuidv7 () PRIMARY KEY,
  firstname varchar(50) NOT NULL,
  lastname varchar(50) NOT NULL,
  email varchar(75) NOT NULL,
  password text NOT NULL,
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (email)
);

CREATE INDEX idx_users_first_last ON users (firstname, lastname);

/* Teams table */
CREATE TABLE teams (
  team_id uuid DEFAULT uuidv7 () PRIMARY KEY,
  tname text NOT NULL,
  descr text,
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (tname)
);

/* Roles table */
CREATE TABLE roles (
  role_id uuid DEFAULT uuidv7 () PRIMARY KEY,
  title text NOT NULL,
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (title)
);

/* Items table */
CREATE TABLE items (
  item_id uuid DEFAULT uuidv7 () PRIMARY KEY,
  descr text NOT NULL,
  price numeric(10, 2) NOT NULL CHECK (price >= 0),
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (descr)
);

/* Memberof table */
CREATE TABLE memberof (
  memberof_team_id uuid,
  memberof_user_id uuid,
  memberof_role_id uuid NOT NULL,
  joined timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (memberof_team_id, memberof_user_id),
  FOREIGN KEY (memberof_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_user_id) REFERENCES users (user_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_role_id) REFERENCES roles (role_id) ON DELETE RESTRICT
);

CREATE INDEX idx_memberof_user ON memberof (memberof_user_id);

CREATE INDEX idx_memberof_role ON memberof (memberof_role_id);

/* Team order table */
CREATE TABLE teamorders (
  teamorders_id uuid DEFAULT uuidv7 () PRIMARY KEY,
  teamorders_team_id uuid NOT NULL,
  teamorders_user_id uuid NOT NULL,
  duedate date,
  closed boolean NOT NULL DEFAULT FALSE,
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (teamorders_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (teamorders_user_id) REFERENCES users (user_id) ON DELETE RESTRICT
);

CREATE INDEX idx_teamorders_id_due ON teamorders (teamorders_team_id, duedate);

CREATE INDEX idx_teamorders_team_created ON teamorders (teamorders_team_id, created DESC);

CREATE INDEX idx_teamorders_user ON teamorders (teamorders_user_id);

/* Orders table */
CREATE TABLE orders (
  orders_teamorders_id uuid,
  orders_item_id uuid,
  orders_team_id uuid,
  amt int NOT NULL DEFAULT 1 CHECK (amt >= 1),
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (orders_teamorders_id, orders_item_id),
  FOREIGN KEY (orders_teamorders_id) REFERENCES teamorders (teamorders_id) ON DELETE CASCADE,
  FOREIGN KEY (orders_item_id) REFERENCES items (item_id) ON DELETE RESTRICT,
  FOREIGN KEY (orders_team_id) REFERENCES teams (team_id) ON DELETE CASCADE
);

/* Enforce that orders.orders_team_id matches the team on the referenced team order.
   This prevents the denormalized team_id from drifting out of sync. */
CREATE OR REPLACE FUNCTION enforce_order_team_consistency ()
  RETURNS TRIGGER
  AS $enforce_order_team_consistency$
DECLARE
  expected_team_id uuid;
BEGIN
  SELECT teamorders_team_id INTO expected_team_id
  FROM teamorders
  WHERE teamorders_id = NEW.orders_teamorders_id;

  IF expected_team_id IS NULL THEN
    RAISE EXCEPTION 'Team order % does not exist', NEW.orders_teamorders_id;
  END IF;

  IF NEW.orders_team_id IS DISTINCT FROM expected_team_id THEN
    RAISE EXCEPTION 'orders_team_id (%) does not match team order team (%)',
      NEW.orders_team_id, expected_team_id;
  END IF;

  RETURN NEW;
END;
$enforce_order_team_consistency$
LANGUAGE plpgsql;

CREATE TRIGGER enforce_order_team_id
  BEFORE INSERT OR UPDATE ON orders
  FOR EACH ROW
  EXECUTE PROCEDURE enforce_order_team_consistency ();

CREATE INDEX idx_orders_team ON orders (orders_team_id);

CREATE INDEX idx_orders_item ON orders (orders_item_id);

/* Token blacklist table — persists revoked JWT tokens across server restarts */
CREATE TABLE token_blacklist (
  jti uuid PRIMARY KEY,
  revoked_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at timestamptz NOT NULL
);

CREATE INDEX idx_token_blacklist_expires ON token_blacklist (expires_at);

/* Avatars table */
CREATE TABLE avatars (
  avatar_id uuid PRIMARY KEY,
  name text NOT NULL,
  data bytea NOT NULL,
  content_type text NOT NULL DEFAULT 'image/png',
  created timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (name)
);

ALTER TABLE users ADD COLUMN avatar_id uuid REFERENCES avatars (avatar_id) ON DELETE SET NULL;

CREATE INDEX idx_users_avatar ON users (avatar_id);

/* Create on-update/change function */
CREATE OR REPLACE FUNCTION update_changed_timestamp ()
  RETURNS TRIGGER
  AS $update_changed_timestamp$
BEGIN
  IF ROW (NEW.*) IS DISTINCT FROM ROW (OLD.*) THEN
    NEW.changed = now();
    RETURN new;
  ELSE
    RETURN old;
  END IF;
END;
$update_changed_timestamp$
LANGUAGE plpgsql;

/* Create triggers */
CREATE TRIGGER update_users_changed_at
  BEFORE UPDATE ON users
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_teams_changed_at
  BEFORE UPDATE ON teams
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_roles_changed_at
  BEFORE UPDATE ON roles
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_items_changed_at
  BEFORE UPDATE ON items
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_teamorders_changed_at
  BEFORE UPDATE ON teamorders
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_orders_changed_at
  BEFORE UPDATE ON orders
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE TRIGGER update_memberof_changed_at
  BEFORE UPDATE ON memberof
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

INSERT INTO teams (tname, descr)
  VALUES ('League of Cool Coders', 'LEGO LPAF Team');

INSERT INTO teams (tname, descr)
  VALUES ('Pixel Bakers', 'PoweredUp Team');

INSERT INTO roles (title)
  VALUES ('Admin');

INSERT INTO roles (title)
  VALUES ('Team Admin');

INSERT INTO roles (title)
  VALUES ('Member');

INSERT INTO roles (title)
  VALUES ('Guest');

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('admin', 'root', 'admin@admin.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I');

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U1_F', 'U1_L', 'U1_F.U1_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I');

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U2_F', 'U2_L', 'U2_F.U2_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I');

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U3_F', 'U3_L', 'U3_F.U3_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I');

INSERT INTO users (firstname, lastname, email, PASSWORD)
  VALUES ('U4_F', 'U4_L', 'U4_F.U4_L@LEGO.com', '$argon2id$v=19$m=19456,t=2,p=1$dGVzdHNhbHQxMjM0NTY$y8G3TwVWPtVgPT/SfBZY08vUClR99BeLYo3HHqJ5v8I');

INSERT INTO items (descr, price)
  VALUES ('håndværker', '9.00');

INSERT INTO items (descr, price)
  VALUES ('spansk u. birkes', '9.00');

INSERT INTO items (descr, price)
  VALUES ('spansk m. birkes', '9.00');

INSERT INTO items (descr, price)
  VALUES ('skagenslab', '9.00');

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
  AND roles.title = 'Admin';

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
  AND roles.title = 'Member';

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
  AND roles.title = 'Member';

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
  AND roles.title = 'Member';

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
  AND roles.title = 'Team Admin';

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
  AND roles.title = 'Member';

INSERT INTO teamorders (teamorders_team_id, teamorders_user_id)
SELECT
  teams.team_id,
  users.user_id
FROM
  teams,
  users
WHERE
  teams.tname = 'League of Cool Coders'
  AND users.email = 'admin@admin.com';

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
  AND teams.tname = 'League of Cool Coders';

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
  AND teams.tname = 'League of Cool Coders';
