# omp-breakfast — Project Context

## Overview
A breakfast ordering REST API for teams, built in Rust with actix-web. Users belong to teams via roles, teams can place breakfast orders composed of items. The project is used internally at LEGO (FabuLab).

## Tech Stack
- **Language:** Rust 2021 edition
- **Web framework:** actix-web 4 (with rustls TLS)
- **Database:** PostgreSQL via `deadpool-postgres` connection pool + `tokio-postgres`
- **ORM/mapping:** `tokio-pg-mapper` (derive-based row mapping)
- **Auth:** JWT (access + refresh tokens via `jsonwebtoken`) + Basic Auth (Argon2 password hashing)
- **Validation:** `validator` crate with derive macros
- **Error handling:** `thiserror` for typed error enum, `color-eyre` for colorized panic/error reports
- **Observability:** `tracing` + `tracing-subscriber` (Bunyan JSON in prod, colorized ANSI in dev), OpenTelemetry spans, `color-eyre` SpanTrace via `tracing-error`
- **API docs:** `utoipa` + `utoipa-swagger-ui` (Swagger UI at `/explorer`)
- **TLS:** rustls with local certs (mkcert) for both the web server and DB connections

## Build & Run
```bash
cargo build                    # compile
cargo test                     # run unit tests only (integration tests auto-skip)
cargo watch -x check -x fmt -x run  # dev mode with auto-reload
docker compose up -d           # start Postgres + app
make test-unit                 # alias for cargo test
make test-integration          # spin up test DB (port 5433), run integration tests, tear down
make test-all                  # run unit tests, then integration tests
make db-up                     # start test DB only
make db-down                   # stop and remove test DB
```

## Project Structure
```
src/
  main.rs          – Entry point, calls server()
  server.rs        – Server setup: TLS, tracing, DB pool, HTTP server
  config.rs        – Settings loaded from config/*.yml + env vars
  models.rs        – All data structs (User, Team, Role, Order, Claims, State)
  db.rs            – All database query functions (one per operation)
  errors.rs        – Error enum with thiserror + ResponseError impl (maps to HTTP status codes)
  validate.rs      – Generic validation wrapper using validator crate
  routes.rs        – All route definitions with auth middleware wiring
  lib.rs           – Module declarations
  handlers/
    mod.rs         – get_client() utility + health endpoint
    users.rs       – User CRUD + auth handlers
    teams.rs       – Team CRUD + order stub handlers (NotImplemented)
    roles.rs       – Role CRUD handlers
  middleware/
    auth.rs        – JWT/Basic auth validators, token generation/verification, blacklist
    openapi.rs     – OpenApi derive + Swagger UI endpoint
config/
  default.yml      – Base config
  development.yml  – Dev overrides (local DB)
  production.yml   – Prod overrides
database.sql       – Full schema + seed data
tests/
  api_tests.rs     – Integration tests (ignored without running DB)
```

## Key Conventions
- Every handler returns `Result<impl Responder, Error>` using the custom `errors::Error` enum
- DB functions take a `&Client` and return `Result<T, Error>`, using `.map_err(Error::Db)?` pattern
- All handlers are instrumented with `#[instrument(skip(state), level = "debug")]`
- Validation uses `validate(&json)?` before any DB call
- JWT auth uses access tokens (15min) + refresh tokens (7 days) with token rotation
- Token revocation uses an in-memory `flurry::HashMap` blacklist (not persisted)
- Error responses are JSON `{"error": "..."}` via `ErrorResponse` struct
- 4xx errors log with `warn!()`, 5xx errors log with `error!()` for color-coded severity
- Config is layered: default.yml → environment.yml → env vars (separator: `_`)

## Unfinished Work
- Team order endpoints in `handlers/teams.rs` are stubs returning `NotImplemented`
- The `items` and `teamorders` and `orders` tables exist in `database.sql` but have no corresponding models, db functions, or handlers in Rust code
- No `items` CRUD endpoints exist

## Testing
- 27 unit tests across `errors`, `middleware::auth`, and `validate` modules
- 15 integration tests in `tests/api_tests.rs` (require running Postgres, marked `#[ignore]`)
- No tests for `db.rs` functions (they require a live DB connection)
- Run unit tests only: `cargo test` or `make test-unit`
- Run integration tests: `make test-integration` (starts a test DB on port 5433 via `docker-compose.test.yml`, runs ignored tests, then tears down)
- Run everything: `make test-all`
- Test DB uses `docker-compose.test.yml` overlay to expose port 5433 (avoids conflicts with dev DB on 5432)
