# omp-breakfast — Project Context

## Overview

A breakfast ordering application for teams, built in Rust with an actix-web REST API backend and a Leptos WebAssembly single-page frontend. Users belong to teams via roles, teams can place breakfast orders composed of items. The project is used internally at LEGO (FabuLab).

## Tech Stack

- **Language:** Rust 2024 edition
- **Web framework:** actix-web 4 (with rustls TLS)
- **Database:** PostgreSQL via `deadpool-postgres` connection pool + `tokio-postgres`
- **ORM/mapping:** `tokio-pg-mapper` (derive-based row mapping)
- **Auth:** JWT (access + refresh tokens via `jsonwebtoken`) + Basic Auth (Argon2 password hashing) + RBAC (Admin/Team Admin/Member/Guest roles, admin bypass)
- **Rate limiting:** `actix-governor` on auth endpoints (6s per request, burst size 10)
- **Validation:** `validator` crate with derive macros
- **Error handling:** `thiserror` for typed error enum, `color-eyre` for colorized panic/error reports
- **Observability:** `tracing` + `tracing-subscriber` (Bunyan JSON in prod, colorized ANSI in dev), OpenTelemetry spans, `color-eyre` SpanTrace via `tracing-error`
- **API docs:** `utoipa` + `utoipa-swagger-ui` (Swagger UI at `/explorer`)
- **TLS:** rustls with local certs (mkcert) for both the web server and DB connections
- **Decimal:** `rust_decimal` for monetary/price values (numeric(10,2) in DB)
- **Frontend framework:** Leptos 0.8 (CSR mode, client-side rendered WebAssembly SPA)
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
    mod.rs         – get_client() utility, health endpoint, RBAC helpers (require_admin, require_team_admin, require_team_member, require_self_or_admin)
    users.rs       – User CRUD + auth handlers (RBAC: self or admin)
    teams.rs       – Team CRUD + team order + member management handlers (team RBAC)
    roles.rs       – Role CRUD handlers (admin-gated CUD)
    items.rs       – Item CRUD handlers (breakfast items with prices, admin-gated CUD)
    orders.rs      – Order item CRUD handlers (items within team orders, member-gated)
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
    ui_tests.rs    – 21 WASM integration tests (headless Chrome)
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
- Auth cache uses TTL (5min) and max-size (1000 entries) with FIFO eviction
- RBAC: Four roles — Admin (global superuser), Team Admin (team-scoped), Member, Guest. JWT claims stored in request extensions.
- Global Admin RBAC: `require_admin` helper checks if user holds "Admin" role in any team (via `db::is_admin`); gates team CUD, items CUD, roles CUD. Admin bypasses all team-scoped and self-only checks.
- Team RBAC: `require_team_member` and `require_team_admin` helpers gate team-scoped mutations; both allow global Admin bypass. `require_team_admin` checks for "Team Admin" role in the specific team.
- Self-or-Admin RBAC: `require_self_or_admin` helper gates user mutations (update, delete); allows the user themselves or a global Admin.
- `Error::Forbidden` variant maps to HTTP 403 for authorization failures
- Production safety: server panics at startup if JWT secret is still the default value when `ENV=production`
- Error responses are JSON `{"error": "..."}` via `ErrorResponse` struct
- 4xx errors log with `warn!()`, 5xx errors log with `error!()` for color-coded severity
- Config is layered: default.yml → environment.yml → env vars (separator: `_`)
- Backend serves `frontend/dist/` as static files via `actix-files`, with `index_file("index.html")`

## Frontend Architecture

The frontend is a separate Rust crate (`frontend/`) compiled to WebAssembly via Trunk. It runs entirely in the browser (CSR mode).

- **Component hierarchy:** `App` → `LoginPage` / `LoadingPage` / `DashboardPage`
  - `LoginPage` uses: `LoginHeader`, `LoginForm`, `ErrorAlert`, `UsernameField`, `PasswordField`, `SubmitButton`
  - `LoadingPage`: Displayed during session restoration from stored JWT token
  - `DashboardPage` uses: `SuccessBadge`, `UserCard`
- **Page routing:** Manual via `Page` enum (`Login` / `Loading` / `Dashboard`) + Leptos signals (no router crate)
- **Auth flow:** Basic Auth POST to `/auth` → receive JWT tokens → store `access_token` in `localStorage` → decode JWT payload for `user_id` → GET `/api/v1.0/users/{id}` for user details → render dashboard
- **Session restore:** On startup, checks `localStorage` for existing `access_token` → shows `LoadingPage` → validates token via user fetch → restores dashboard or falls back to login
- **Client-side validation:** Both username and password required before form submission
- **Error display:** HTTP 401 → "Invalid username or password"; network failure → "Unable to reach the server"
- **Dev proxying:** Trunk proxies `/auth`, `/api`, `/health` to `https://127.0.0.1:8080` (configured in `Trunk.toml`)

### Frontend Conventions

- Components use `#[component]` macro and return `impl IntoView`
- Reactive state uses `ReadSignal` / `WriteSignal` pairs
- `pub` items (`JwtPayload`, `decode_jwt_payload`) are exposed via `lib.rs` for test access
- Token is stored in `localStorage` under the key `access_token`
- HTTP requests use `gloo_net::http::Request` (wraps `window.fetch`)

## Frontend Roadmap

The frontend will evolve from login + dashboard into a full-featured SPA. This section captures planned layout, navigation, pages, and UI requirements.

### Layout

- **Sidebar + main content area:** A collapsible sidebar on the left provides navigation; the main content panel fills the remaining space
- The sidebar should show the app branding/logo at the top, navigation links in the middle, and the logged-in user's name + logout button at the bottom
- On mobile viewports the sidebar collapses into a hamburger menu overlay

### Navigation

- Continue using the signal-based `Page` enum approach (no router crate)
- Extend the `Page` enum with variants for each new page (e.g., `Teams`, `Orders`, `Items`, `Profile`, `Admin`, `Roles`)
- Active page is highlighted in the sidebar
- Unauthorized pages (e.g., `Admin`) should not appear in the sidebar for non-admin users

### Planned Pages

1. **Team Management** — Create, view, and edit teams; add/remove members; assign team roles
2. **Order Management** — Create and view team orders; add, edit, and remove order line items; show order totals
3. **Item Catalog** — Browse available breakfast items with descriptions and prices; admin users can create, edit, and delete items
4. **User Profile** — View and edit own profile details; change password
5. **Admin Dashboard** — Admin-only view for managing all users and assigning roles; not visible to non-admin users
6. **Role Management** — View and assign roles (admin-gated)

### UI / UX Requirements

- **Theming:** Support both light and dark mode; respect the user's OS/browser `prefers-color-scheme` preference, with a manual toggle in the sidebar or top bar
- **Responsive design:** Mobile-first CSS; must render correctly on iPhone 13 and later (Safari, ≥ 390px viewport width)
- **Toast notifications:** Non-blocking success/error toasts for CRUD operations (e.g., "Item created", "Failed to delete team")
- **Confirmation modals:** Destructive actions (delete user, remove team member, delete order) require a confirmation dialog before executing
- **Loading states:** Show skeleton loaders or spinners while fetching data from the API
- **Form validation:** Client-side validation with inline error messages before submission; mirror backend `validator` rules where applicable

## Markdown Style Rules

When creating or editing `.md` files (including `.claude/commands/*.md`), follow these rules to avoid markdownlint warnings:

- Every file must start with a top-level heading (`# Title`)
- Leave a blank line after every heading before content
- Leave a blank line before and after lists
- Leave a blank line before and after fenced code blocks
- Always specify a language on fenced code blocks (e.g. ` ```rust `, ` ```text `, ` ```bash `)
- Leave a blank line before and after tables
- Align table separator pipes with header pipes (use ` --- ` padding, not ragged dashes)
- Use sequential ordered list numbering (`1.`, `2.`, `3.`) — do not continue numbering across separate sections

## Version Bumping

When asked to bump the project version, **all** of the following steps **must** be performed:

1. Determine the bump type — `major`, `minor`, or `patch` — following semantic versioning (semver):
   - **major** (`X.0.0`): incompatible API or breaking changes
   - **minor** (`x.Y.0`): new functionality, backwards-compatible
   - **patch** (`x.y.Z`): backwards-compatible bug fixes
2. Update the `version` field in the root `Cargo.toml`
3. Update the `version` field in `frontend/Cargo.toml` to match
4. Commit the version change (message: `chore: bump version to vX.Y.Z`)
5. Create an annotated git tag: `git tag -a vX.Y.Z -m "vX.Y.Z"`
6. Push the commit **and** the tag to upstream: `git push && git push --tags`

If the bump type is not specified, ask before proceeding. Never skip the git tag or the push of tags.

## Project Assessment

When asked to **assess the project** (or "project assessment"), perform the following:

1. Run every command defined in `.claude/commands/` against the current codebase:
   - `api-completeness` — compare DB schema vs implemented endpoints and frontend consumption
   - `db-review` — review schema design, indexing, constraints, and query patterns
   - `dependency-check` — analyze Cargo dependencies for freshness, redundancy, and compatibility
   - `openapi-sync` — validate OpenAPI spec against routes and frontend API usage
   - `practices-audit` — audit code against conventions documented in this file
   - `rbac-rules` — audit RBAC enforcement against the documented role policy table
   - `review` — full code review (idioms, error handling, duplication, dead code)
   - `security-audit` — JWT/auth, input validation, secrets, TLS, Docker, frontend security
   - `test-gaps` — identify missing test coverage and suggest specific new tests
2. Collect all findings that indicate actionable changes (bugs, missing implementations, convention violations, security issues, stale dependencies, etc.)
3. Present a single consolidated plan grouped by category, listing each proposed change with:
   - Which command surfaced it
   - What needs to change and where
   - Severity (critical / important / minor / informational)
4. **Do not apply any changes** — only present the plan for approval
5. If no actionable findings are discovered, state that the project is in good shape

This assessment must consider **all** commands in `.claude/commands/` at the time it is run, including any added after this rule was written.

## Unfinished Work

- Frontend only has login + dashboard pages; remaining pages are tracked in the **Frontend Roadmap** section
- No client-side routing library (manual signal-based page switching, by design)
- Frontend does not yet consume the team, role, item, or order APIs
- Dark/light mode toggle not yet implemented
- Toast notifications and confirmation modals not yet implemented

## Testing

### Backend

- 35 unit tests across `errors`, `middleware::auth`, and `validate` modules
- 44 integration tests in `tests/api_tests.rs` (require running Postgres, marked `#[ignore]`)
- No tests for `db.rs` functions (they require a live DB connection)
- Run unit tests only: `cargo test` or `make test-unit`
- Run integration tests: `make test-integration` (starts a test DB on port 5433 via `docker-compose.test.yml`, runs ignored tests, then tears down)
- Test DB uses `docker-compose.test.yml` overlay to expose port 5433 (avoids conflicts with dev DB on 5432)

### Frontend

- 21 WASM tests in `frontend/tests/ui_tests.rs` (run in headless Chrome via `wasm-pack`)
- Test categories:
  - JWT decode (4 tests): valid token, missing segments, bad base64, invalid JSON
  - Login page rendering (3 tests): brand/form elements, email attributes, password attributes
  - Client-side validation (3 tests): empty form, email-only, password-only
  - Login flow with mocked HTTP (3 tests): success → dashboard, 401 → error, network error → message
  - Dashboard & logout (2 tests): user card structure, logout returns to login
  - Full end-to-end cycle (1 test): login → validation → success → dashboard → logout
  - Session persistence (2 tests): session persists across refresh, logout clears tokens
  - Session restore edge cases (3 tests): malformed token fallback, expired token fallback, loading page display
- Mocking strategy: overrides `window.fetch` via `js_sys::eval` to intercept `gloo-net` HTTP calls
- Run frontend tests: `make test-frontend` or `cd frontend && wasm-pack test --headless --chrome`
- Note: ChromeDriver version must match installed Chrome version

### All Tests

- Run everything: `make test-all` (backend unit + integration + frontend WASM)

## Required Test Runs

Before committing any backend changes, **both** unit tests and integration tests must pass:

1. Run `cargo test` (unit tests — must show 0 failures)
2. Run `make test-integration` (integration tests — must show 0 failures)

Do not commit if either test suite fails. If only frontend code changed, `make test-frontend` may be substituted for step 2.
