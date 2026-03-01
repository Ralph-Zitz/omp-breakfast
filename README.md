# Breakfast App (WIP)

## Backend written in [Rust](https://www.rust-lang.org/) using [actix-web](https://actix.rs/)

### Setup

1. Git clone this repo
2. Install Rust. See guide [here](https://www.rust-lang.org/tools/install)
3. Install a postgres client (on Mac): `brew install postgresql@15`
4. Add some nice tools for auto-reloading: `cargo install cargo-watch`
5. Install JSON parser (on Mac): `brew install jq`
6. Install certificate tools (on Mac): `brew install mkcert nss`
7. Generate CA files for signing certificates for the web server: `mkcert -install`
8. Generate the certificates with the web server: `mkcert localhost postgres 127.0.0.1 ::1`
9. Place the generated certificates in the root folder of this project
10. Modify the content of file `server.rs` and adjust paths to read the generated certificates
11. Run `docker compose up -d` to start the local Postgresql database and the application

To only run the database:

* Run the command: `docker compose run postgres-setup postgres -d`

Then manually connect by running the application from the command line:

* Build and run the server: `cargo watch -x check -x fmt -x run`
* Check that connection to database is ok: `curl -v https://localhost:8080/health -s | jq`

Make profit :wink:

### Testing

The project has two levels of tests:

**Unit tests** — run without any external dependencies:
```bash
make test-unit     # or: cargo test
```

**Integration tests** — require a PostgreSQL database (managed via Docker):
```bash
make test-integration
```
This automatically:
1. Starts an isolated Postgres container on port 5433 (via `Dockerfile.postgres`)
2. Runs the V1 migration (creates schema) via init script
3. Seeds it with `database_seed.sql`
4. Runs the 15 integration tests
5. Tears down the container

**All tests** — run unit and integration tests in sequence:
```bash
make test-all
```

**Prerequisites for integration tests:**
- Docker must be running
- A PostgreSQL client (`psql`) must be installed (`brew install postgresql@15`)

You can also manage the test database manually:
```bash
make db-up         # start test DB on port 5433
make db-down       # stop and remove test DB
```

### Database Initialization

The application uses **different initialization strategies** for development vs production:

**Production (and docker-compose):**
- The application runs Refinery migrations at startup (`migrations/V1__initial_schema.sql`)
- Migrations are tracked in the `refinery_schema_history` table
- No seed data is inserted in production

**Development (docker-compose):**
- `docker compose up` starts the `postgres-setup` service
- This service runs `init_dev_db.sh` which:
  1. Creates the migration tracking table
  2. Marks V1 migration as applied (so app doesn't re-run it)
  3. Loads seed data from `database_seed.sql`
- The application sees the schema is already at V1 and continues normally

**Manual database reset (development only):**
```bash
# If you need to completely reset your local database from scratch:
docker compose down -v
PGPASSWORD=actix psql -h localhost -p 5432 -U actix actix < database.sql
# Or just restart docker-compose which will reinitialize:
docker compose up -d
```

The `database.sql` file is kept for manual resets but is no longer used by docker-compose.
