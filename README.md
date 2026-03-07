# Breakfast App

A breakfast ordering application for teams, built in Rust with an [actix-web](https://actix.rs/) REST API backend and a [Leptos](https://leptos.dev/) WebAssembly single-page frontend. Used internally at LEGO (FabuLab).

## Tech Stack

- **Backend:** Rust / actix-web 4 with TLS (rustls)
- **Frontend:** Leptos 0.8 (CSR mode, compiled to WebAssembly via Trunk)
- **Database:** PostgreSQL 18 via `deadpool-postgres` connection pool
- **Auth:** JWT (access + refresh tokens) + Basic Auth (Argon2id password hashing) + RBAC
- **API docs:** Swagger UI at `/explorer` (via utoipa)
- **Observability:** `tracing` with OpenTelemetry spans
- **Containerization:** Docker Compose (app + PostgreSQL with TLS)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [Trunk](https://trunkrs.dev/) for frontend builds: `cargo install trunk`
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) for frontend tests: `cargo install wasm-pack`
- Docker for running PostgreSQL
- PostgreSQL client: `brew install postgresql` (macOS)
- Certificate tools: `brew install mkcert nss` (macOS)
- Optional: `cargo install cargo-watch` for auto-reload during development
- Optional: `brew install jq` for pretty-printing JSON responses

## Setup

1. Clone this repo
2. Generate a local CA for signing certificates: `mkcert -install`
3. Generate TLS certificates: `mkcert localhost postgres 127.0.0.1 ::1`
4. Place the generated files (`localhost.pem`, `localhost_key.pem`, `localhost_ca.pem`) in the project root
5. Start everything: `docker compose up -d`

To run only the database and start the backend manually:

```bash
docker compose up -d postgres            # start PostgreSQL
docker compose run --rm postgres-setup   # initialize schema + seed data
cargo watch -x check -x fmt -x run      # build and run with auto-reload
```

Verify the server is running:

```bash
curl -k https://localhost:8080/health | jq
```

The frontend dev server (with API proxying) can be started separately:

```bash
make frontend-dev   # serves at http://127.0.0.1:8081
```

## Testing

The project has three test suites:

**Unit tests** (238 tests) — no external dependencies:

```bash
make test-unit     # or: cargo test
```

**Integration tests** (279 tests: 167 API + 112 DB) — require PostgreSQL:

```bash
make test-integration
```

This automatically starts an isolated Postgres container on port 5433, runs all thirteen migrations (V1–V13), seeds test data, executes all integration tests, and tears down the container.

**Frontend WASM tests** (79 tests) — require Chrome:

```bash
make test-frontend
```

**All tests** — run all three suites plus a dependency audit:

```bash
make test-all
```

**Prerequisites for integration tests:**

- Docker must be running
- A PostgreSQL client (`psql`) must be installed

You can also manage the test database manually:

```bash
make db-up         # start test DB on port 5433
make db-down       # stop and remove test DB
```

## Database Initialization

The application uses [Refinery](https://github.com/rust-db/refinery) for schema migrations. Thirteen migrations exist:

| Migration | Description |
| --- | --- |
| V1 | Initial schema (tables, triggers) |
| V2 | UUID v7 defaults |
| V3 | Indexes, FK constraints, NOT NULL |
| V4 | Schema hardening |
| V5 | Trigger fix on users, NOT NULL on teamorders/memberof |
| V6 | Unique constraint on orders, covering index |
| V7 | Drop redundant idx_users_email and idx_teams_name indexes |
| V8 | Avatars table + users.avatar_id FK column |
| V9 | Avatar FK index + token_blacklist.revoked_at NOT NULL |
| V10 | Guard teamorders_team_id with trigger |
| V11 | CHECK constraints on text column lengths |
| V12 | Drop unused idx_teamorders_id_due, NOT NULL on orders_team_id |
| V13 | Adds pickup_user_id column to teamorders table |

**Production:** The application runs pending migrations automatically at startup. No seed data is inserted. The first user to register via the login page becomes the global Admin.

**Development (docker-compose):** The `postgres-setup` service runs `init_dev_db.sh`, which applies the idempotent migrations (V1–V9) and creates the Refinery tracking table. On first startup, the application's migration runner re-applies V1–V9 (safe — idempotent), records them, then applies V10–V13 for the first time. The first user registers via `POST /auth/register` (or through the login page registration form) and becomes the Admin.

**Manual database reset (development only):**

```bash
docker compose down -v   # remove volumes
docker compose up -d     # reinitialize from scratch
```

## Make Targets

| Target | Description |
| --- | --- |
| `make build` | Build frontend (Trunk) + backend (cargo) |
| `make test-unit` | Run backend unit tests |
| `make test-integration` | Run integration tests (manages test DB lifecycle) |
| `make test-frontend` | Run frontend WASM tests (headless Chrome) |
| `make test-all` | Run all test suites + dependency audit |
| `make frontend-build` | Build frontend with Trunk (release) |
| `make frontend-dev` | Start Trunk dev server on `http://127.0.0.1:8081` |
| `make frontend-clean` | Remove `frontend/dist/` |
| `make db-up` | Start test DB on port 5433 |
| `make db-down` | Stop and remove test DB |
| `make check` | Run `cargo check` |
| `make fmt` | Run `cargo fmt` |
| `make audit` | Run `cargo audit` |
| `make audit-install` | Install `cargo-audit` |

## Configuration

Configuration is layered: `config/default.yml` → `config/{ENV}.yml` → environment variables (prefix: `BREAKFAST_`, separator: `_`).

Key environment variables:

| Variable | Description | Default |
| --- | --- | --- |
| `ENV` | Environment (`development` / `production`) | `development` |
| `BREAKFAST_SERVER_SECRET` | Server secret (must change in production) | `Very Secret` |
| `BREAKFAST_SERVER_JWTSECRET` | JWT signing secret (must change in production) | `Very Secret` |
| `BREAKFAST_PG_USER` | Database user (must change in production) | `actix` |
| `BREAKFAST_PG_PASSWORD` | Database password (must change in production) | `actix` |
| `BREAKFAST_PG_HOST` | Database hostname | `localhost` |
| `BREAKFAST_PG_PORT` | Database port | `5432` |

The server panics at startup if default secrets or database credentials are used in production (`ENV=production`).

## License

MIT — see [LICENSE](LICENSE) for details.
