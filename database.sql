/* Drop Tables */
DROP TABLE IF EXISTS memberof;

DROP TABLE IF EXISTS orders;

DROP TABLE IF EXISTS teamorders;

DROP TABLE IF EXISTS users;

DROP TABLE IF EXISTS teams;

DROP TABLE IF EXISTS roles;

DROP TABLE IF EXISTS items;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

/* Set Timezone */
SET timezone = 'Europe/Copenhagen';

/* Users table */
CREATE TABLE users (
  user_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  firstname varchar(50) NOT NULL,
  lastname varchar(50) NOT NULL,
  email varchar(75) NOT NULL,
  password text NOT NULL,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (email)
);

CREATE INDEX idx_users_first_last ON users (firstname, lastname);

CREATE INDEX idx_users_email ON users (email);

/* Teams table */
CREATE TABLE teams (
  team_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  tname text NOT NULL,
  descr text,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (tname)
);

CREATE INDEX idx_teams_name ON teams (tname);

/* Roles table */
CREATE TABLE roles (
  role_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  title text NOT NULL,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (title)
);

/* Items table */
CREATE TABLE items (
  item_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  descr text NOT NULL,
  price numeric(10, 2),
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (descr)
);

/* Memberof table */
CREATE TABLE memberof (
  memberof_team_id uuid,
  memberof_user_id uuid,
  memberof_role_id uuid,
  joined timestamptz DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (memberof_team_id, memberof_user_id),
  FOREIGN KEY (memberof_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_user_id) REFERENCES users (user_id) ON DELETE CASCADE,
  FOREIGN KEY (memberof_role_id) REFERENCES roles (role_id) ON DELETE RESTRICT
);

CREATE INDEX idx_memberof_user ON memberof (memberof_user_id);

CREATE INDEX idx_memberof_role ON memberof (memberof_role_id);

/* Team order table */
CREATE TABLE teamorders (
  teamorders_id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
  teamorders_team_id uuid NOT NULL,
  teamorders_user_id uuid,
  duedate date,
  closed boolean DEFAULT FALSE,
  created timestamptz DEFAULT CURRENT_TIMESTAMP,
  changed timestamptz DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (teamorders_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  FOREIGN KEY (teamorders_user_id) REFERENCES users (user_id) ON DELETE RESTRICT
);

CREATE INDEX idx_teamorders_id_due ON teamorders (teamorders_team_id, duedate);

/* Orders table */
CREATE TABLE orders (
  orders_teamorders_id uuid,
  orders_item_id uuid,
  orders_team_id uuid,
  amt int,
  PRIMARY KEY (orders_teamorders_id, orders_item_id),
  FOREIGN KEY (orders_teamorders_id) REFERENCES teamorders (teamorders_id) ON DELETE CASCADE,
  FOREIGN KEY (orders_item_id) REFERENCES items (item_id) ON DELETE CASCADE,
  FOREIGN KEY (orders_team_id) REFERENCES teams (team_id) ON DELETE CASCADE,
  UNIQUE (orders_teamorders_id, orders_item_id)
);

CREATE INDEX idx_orders_tid ON orders (orders_teamorders_id);

/* Token blacklist table — persists revoked JWT tokens across server restarts */
CREATE TABLE token_blacklist (
  jti uuid PRIMARY KEY,
  revoked_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  expires_at timestamptz NOT NULL
);

CREATE INDEX idx_token_blacklist_expires ON token_blacklist (expires_at);

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
  BEFORE INSERT OR UPDATE ON users
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

INSERT INTO teamorders (teamorders_team_id)
SELECT
  team_id
FROM
  teams
WHERE
  teams.tname = 'League of Cool Coders';

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
