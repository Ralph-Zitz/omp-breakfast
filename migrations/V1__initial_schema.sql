-- Initial database schema for OMP Breakfast.
--
-- This migration captures the complete schema as of v0.6.1.
-- All statements are idempotent (IF NOT EXISTS / OR REPLACE).

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

SET timezone = 'Europe/Copenhagen';

/* Users table */
CREATE TABLE IF NOT EXISTS users (
  user_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  firstname varchar(50) NOT NULL,
  lastname varchar(50) NOT NULL,
  email varchar(75) NOT NULL,
  password text NOT NULL,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (email)
);

CREATE INDEX IF NOT EXISTS idx_users_first_last ON users (firstname, lastname);

CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

/* Teams table */
CREATE TABLE IF NOT EXISTS teams (
  team_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  tname text NOT NULL,
  descr text,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (tname)
);

CREATE INDEX IF NOT EXISTS idx_teams_name ON teams (tname);

/* Roles table */
CREATE TABLE IF NOT EXISTS roles (
  role_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  title text NOT NULL,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (title)
);

/* Items table */
CREATE TABLE IF NOT EXISTS items (
  item_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  descr text NOT NULL,
  price numeric(10, 2) CHECK (price >= 0),
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (descr)
);

/* Memberof table */
CREATE TABLE IF NOT EXISTS memberof (
  memberof_team_id uuid,
  memberof_user_id uuid,
  memberof_role_id uuid,
  joined timestamptz DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (memberof_team_id, memberof_user_id),
  FOREIGN KEY (memberof_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_user_id) REFERENCES users (user_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_role_id) REFERENCES roles (role_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_memberof_user ON memberof (memberof_user_id);

CREATE INDEX IF NOT EXISTS idx_memberof_role ON memberof (memberof_role_id);

/* Team order table */
CREATE TABLE IF NOT EXISTS teamorders (
  teamorders_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  teamorders_team_id uuid NOT NULL,
  teamorders_user_id uuid,
  duedate date,
  closed boolean NOT NULL DEFAULT FALSE,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (teamorders_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (teamorders_user_id) REFERENCES users (user_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_teamorders_id_due ON teamorders (teamorders_team_id, duedate);

/* Orders table */
CREATE TABLE IF NOT EXISTS orders (
  orders_teamorders_id uuid,
  orders_item_id uuid,
  orders_team_id uuid,
  amt int CHECK (amt >= 0),
  PRIMARY KEY (orders_teamorders_id, orders_item_id),
  FOREIGN KEY (orders_teamorders_id) REFERENCES teamorders (teamorders_id) ON DELETE CASCADE,
  FOREIGN KEY (orders_item_id) REFERENCES items (item_id) ON DELETE CASCADE,
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

CREATE OR REPLACE TRIGGER enforce_order_team_id
  BEFORE INSERT OR UPDATE ON orders
  FOR EACH ROW
  EXECUTE PROCEDURE enforce_order_team_consistency ();

CREATE INDEX IF NOT EXISTS idx_orders_tid ON orders (orders_teamorders_id);

/* Token blacklist table — persists revoked JWT tokens across server restarts */
CREATE TABLE IF NOT EXISTS token_blacklist (
  jti uuid PRIMARY KEY,
  revoked_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  expires_at timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_blacklist_expires ON token_blacklist (expires_at);

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
CREATE OR REPLACE TRIGGER update_users_changed_at
  BEFORE INSERT OR UPDATE ON users
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE OR REPLACE TRIGGER update_teams_changed_at
  BEFORE UPDATE ON teams
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE OR REPLACE TRIGGER update_roles_changed_at
  BEFORE UPDATE ON roles
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE OR REPLACE TRIGGER update_items_changed_at
  BEFORE UPDATE ON items
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();

CREATE OR REPLACE TRIGGER update_teamorders_changed_at
  BEFORE UPDATE ON teamorders
  FOR EACH ROW
  EXECUTE PROCEDURE update_changed_timestamp ();
