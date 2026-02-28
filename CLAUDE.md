# omp-breakfast — Project Context

## Overview

A breakfast ordering application for teams, built in Rust with an actix-web REST API backend and a Leptos WebAssembly single-page frontend. Users belong to teams via roles, teams can place breakfast orders composed of items. The project is used internally at LEGO (FabuLab).

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
- **Frontend framework:** Leptos 0.7 (CSR mode, client-side rendered WebAssembly SPA)
- **WASM bundler:** Trunk (builds frontend to `frontend/dist/`)
- **Frontend HTTP client:** `gloo-net` 0.6 (wraps `window.fetch`)
- **Static file serving:** `actix-files` (serves `frontend/dist/` at `/`)
- **Frontend testing:** `wasm-bindgen-test` + `wasm-pack` (headless Chrome)

## Build & Run

```bash
cargo build                    # compile backend
cargo test                     # run backend unit tests (integration tests auto-skip)
cargo watch -x check -x fmt -x run  # dev mode with auto-reload
docker compose up -d           # start Postgres + app
make build                     # build frontend (Trunk) + backend (cargo)
make test-unit                 # alias for cargo test
make test-integration          # spin up test DB (port 5433), run integration tests, tear down
make test-frontend             # run WASM tests in headless Chrome via wasm-pack
make test-all                  # run unit + integration + frontend tests
make frontend-build            # build frontend with Trunk (release)
make frontend-dev              # start Trunk dev server on http://127.0.0.1:8081
make frontend-clean            # remove frontend/dist/
make db-up                     # start test DB only
make db-down                   # stop and remove test DB
```

## Project Structure

```text
src/
  main.rs          – Entry point, calls server()
  server.rs        – Server setup: TLS, tracing, DB pool, HTTP server + static file serving
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
frontend/
  Cargo.toml       – Frontend crate config (breakfast-frontend)
  Trunk.toml       – Trunk config: output dir, watch paths, API proxies
  index.html       – Trunk HTML shell with data-trunk CSS link
  src/
    lib.rs         – Library entry point (pub mod app)
    main.rs        – Binary entry point: mounts App to <body>
    app.rs         – All UI components, auth logic, API calls
  style/
    main.css       – Modern CSS (custom properties, responsive, animations)
  tests/
    ui_tests.rs    – 16 WASM integration tests (headless Chrome)
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
- Backend serves `frontend/dist/` as static files via `actix-files`, with `index_file("index.html")`

## Frontend Architecture

The frontend is a separate Rust crate (`frontend/`) compiled to WebAssembly via Trunk. It runs entirely in the browser (CSR mode).

- **Component hierarchy:** `App` → `LoginPage` / `DashboardPage`
  - `LoginPage` uses: `LoginHeader`, `LoginForm`, `ErrorAlert`, `UsernameField`, `PasswordField`, `SubmitButton`
  - `DashboardPage` uses: `SuccessBadge`, `UserCard`
- **Page routing:** Manual via `Page` enum (`Login` / `Dashboard`) + Leptos signals (no router crate)
- **Auth flow:** Basic Auth POST to `/auth` → receive JWT tokens → store `access_token` in `localStorage` → decode JWT payload for `user_id` → GET `/api/v1.0/users/{id}` for user details → render dashboard
- **Client-side validation:** Both username and password required before form submission
- **Error display:** HTTP 401 → "Invalid username or password"; network failure → "Unable to reach the server"
- **Dev proxying:** Trunk proxies `/auth`, `/api`, `/health` to `https://127.0.0.1:8080` (configured in `Trunk.toml`)

### Frontend Conventions

- Components use `#[component]` macro and return `impl IntoView`
- Reactive state uses `ReadSignal` / `WriteSignal` pairs
- `pub` items (`JwtPayload`, `decode_jwt_payload`) are exposed via `lib.rs` for test access
- Token is stored in `localStorage` under the key `access_token`
- HTTP requests use `gloo_net::http::Request` (wraps `window.fetch`)

## Markdown Style Rules

When creating or editing `.md` files, follow these rules to avoid markdownlint warnings:

- Every file must start with a top-level heading (`# Title`)
- Leave a blank line after every heading before content
- Leave a blank line before and after lists
- Leave a blank line before and after fenced code blocks
- Always specify a language on fenced code blocks (e.g. ` ```rust `, ` ```text `, ` ```bash `)
- Leave a blank line before and after tables
- Align table separator pipes with header pipes (use ` --- ` padding, not ragged dashes)

## Unfinished Work

- Team order endpoints in `handlers/teams.rs` are stubs returning `NotImplemented`
- The `items` and `teamorders` and `orders` tables exist in `database.sql` but have no corresponding models, db functions, or handlers in Rust code
- No `items` CRUD endpoints exist
- Frontend only has login + dashboard pages; no order management UI yet
- No client-side routing library (manual signal-based page switching)
- Frontend does not yet consume the team, role, or order APIs

## Testing

### Backend

- 34 unit tests across `errors`, `middleware::auth`, and `validate` modules
- 15 integration tests in `tests/api_tests.rs` (require running Postgres, marked `#[ignore]`)
- No tests for `db.rs` functions (they require a live DB connection)
- Run unit tests only: `cargo test` or `make test-unit`
- Run integration tests: `make test-integration` (starts a test DB on port 5433 via `docker-compose.test.yml`, runs ignored tests, then tears down)
- Test DB uses `docker-compose.test.yml` overlay to expose port 5433 (avoids conflicts with dev DB on 5432)

### Frontend

- 16 WASM tests in `frontend/tests/ui_tests.rs` (run in headless Chrome via `wasm-pack`)
- Test categories:
  - JWT decode (4 tests): valid token, missing segments, bad base64, invalid JSON
  - Login page rendering (3 tests): brand/form elements, email attributes, password attributes
  - Client-side validation (3 tests): empty form, email-only, password-only
  - Login flow with mocked HTTP (3 tests): success → dashboard, 401 → error, network error → message
  - Dashboard & logout (2 tests): user card structure, logout returns to login
  - Full end-to-end cycle (1 test): login → validation → success → dashboard → logout
- Mocking strategy: overrides `window.fetch` via `js_sys::eval` to intercept `gloo-net` HTTP calls
- Run frontend tests: `make test-frontend` or `cd frontend && wasm-pack test --headless --chrome`
- Note: ChromeDriver version must match installed Chrome version

### All Tests

- Run everything: `make test-all` (backend unit + integration + frontend WASM)
