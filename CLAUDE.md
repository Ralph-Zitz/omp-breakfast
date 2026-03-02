# omp-breakfast — Project Context

## Overview

A breakfast ordering application for teams, built in Rust with an actix-web REST API backend and a Leptos WebAssembly single-page frontend. Users belong to teams via roles, teams can place breakfast orders composed of items. The project is used internally at LEGO (FabuLab).

## Tech Stack

- **Language:** Rust 2024 edition
- **Web framework:** actix-web 4 (with rustls TLS) + `actix-cors` for CORS policy
- **Database:** PostgreSQL via `deadpool-postgres` connection pool + `tokio-postgres`
- **ORM/mapping:** Custom `FromRow` trait in `src/from_row.rs` (manual row mapping, no external dependency); DB functions organized in `src/db/` module directory by domain
- **Auth:** JWT (access + refresh tokens via `jsonwebtoken`) + Basic Auth (Argon2 password hashing) + RBAC (Admin/Team Admin/Member/Guest roles, admin bypass); in-memory caching via `dashmap` (concurrent HashMap)
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
  server.rs        – Server setup: TLS, tracing, DB pool, HTTP server, static file serving, background tasks
  bin/
    healthcheck.rs – Minimal TLS healthcheck binary for distroless Docker containers
  config.rs        – Settings loaded from config/*.yml + env vars
  from_row.rs      – Custom FromRow trait and FromRowError enum (manual row mapping)
  models.rs        – All data structs (User, Team, Role, Order, Claims, State)
  db/
    mod.rs         – Module declarations + re-exports of all public DB functions
    migrate.rs     – Refinery migration runner (embed_migrations! + run_migrations)
    health.rs      – Database health check (check_db)
    users.rs       – User CRUD (get_users, get_user, get_user_by_email, create_user, update_user, delete_user, delete_user_by_email)
    teams.rs       – Team CRUD + user-team queries (get_teams, get_team, create_team, update_team, delete_team, get_user_teams, get_team_users)
    roles.rs       – Role CRUD (get_roles, get_role, create_role, update_role, delete_role)
    items.rs       – Item CRUD (get_items, get_item, create_item, update_item, delete_item)
    orders.rs      – Team order CRUD (get_team_orders, get_team_order, create_team_order, update_team_order, delete_team_order, delete_team_orders)
    order_items.rs – Order item CRUD + closed-order check (is_team_order_closed, get_order_items, get_order_item, create_order_item, update_order_item, delete_order_item)
    membership.rs  – Team membership + RBAC queries (is_admin, is_admin_or_team_admin, is_team_admin_of_user, get_member_role, check_team_access, add_team_member, remove_team_member, update_member_role)
    tokens.rs      – Token blacklist persistence (revoke_token_db, is_token_revoked_db, cleanup_expired_tokens)
  errors.rs        – Error enum with thiserror + ResponseError impl (maps to HTTP status codes)
  validate.rs      – Generic validation wrapper using validator crate
  routes.rs        – All route definitions with auth middleware wiring
  lib.rs           – Module declarations
  handlers/
    mod.rs         – get_client() utility, health endpoint, RBAC helpers (require_admin, require_admin_or_team_admin, require_team_admin, require_team_member, require_self_or_admin, require_self_or_admin_or_team_admin, requesting_user_id)
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
    ui_tests.rs    – 23 WASM integration tests (headless Chrome)
config/
  default.yml      – Base config
  development.yml  – Dev overrides (local DB)
  production.yml   – Prod overrides
database.sql       – Full schema (deprecated — kept for manual dev resets only; seed data moved to database_seed.sql)
database_seed.sql  – Seed data for development/testing
init_dev_db.sh     – Docker development database initialization script
migrations/
  V1__initial_schema.sql – Refinery migration for the database schema
  V2__uuid_v7_defaults.sql – UUID v7 default migration (PostgreSQL 18+)
  V3__indexes_constraints.sql – Indexes, FK RESTRICT, NOT NULL constraints
tests/
  api_tests.rs     – API integration tests (ignored without running DB)
  db_tests.rs      – DB function integration tests (ignored without running DB)
```

## Key Conventions

- CORS is enforced via `actix-cors` middleware with an explicit same-origin allowlist (methods: GET/POST/PUT/DELETE/OPTIONS; headers: Authorization, Content-Type, Accept; max-age: 3600s)
- Every handler returns `Result<impl Responder, Error>` using the custom `errors::Error` enum
- DB functions take a `&Client` and return `Result<T, Error>`, using `.map_err(Error::Db)?` pattern. Functions that perform multi-step mutations (`add_team_member`, `update_member_role`) take `&mut Client` and wrap operations in a database transaction.
- All handlers are instrumented with `#[instrument(skip(state), level = "debug")]`
- Validation uses `validate(&json)?` before any DB call
- JWT auth uses access tokens (15min) + refresh tokens (7 days) with token rotation
- Token revocation uses a DB-backed `token_blacklist` table (persisted across restarts) with an in-memory `dashmap::DashMap` cache for fast-path lookups. A background task runs every hour to clean up expired entries from both the database (via `db::cleanup_expired_tokens`) and the in-memory map (via `DashMap::retain()`).
- Auth cache uses TTL (5min) and max-size (1000 entries) with FIFO eviction
- RBAC: Four roles — Admin (global superuser), Team Admin (team-scoped), Member, Guest. JWT claims stored in request extensions.
- Global Admin RBAC: `require_admin` helper checks if user holds "Admin" role in any team (via `db::is_admin`); gates team CUD, items CUD, roles CUD. Admin bypasses all team-scoped and self-only checks.
- Admin-or-Team-Admin RBAC: `require_admin_or_team_admin` helper checks if user holds "Admin" or "Team Admin" role in any team (via `db::is_admin_or_team_admin`); gates user creation.
- Team RBAC: `require_team_member` and `require_team_admin` helpers gate team-scoped mutations; both allow global Admin bypass. `require_team_admin` checks for "Team Admin" role in the specific team.
- Self-or-Admin-or-Team-Admin RBAC: `require_self_or_admin_or_team_admin` helper gates user mutations (update, delete); allows the user themselves, a global Admin, or a Team Admin of any team where the target user is also a member (checked via `db::is_team_admin_of_user` — a self-join on `memberof`). The legacy `require_self_or_admin` helper is retained but no longer used by any handler.
- `Error::Forbidden` variant maps to HTTP 403 for authorization failures
- `Error::Unauthorized` variant maps to HTTP 401 for authentication failures
- Production safety: server panics at startup if `server.secret` or `server.jwtsecret` is still the default value when `ENV=production`
- Error responses are JSON `{"error": "..."}` via `ErrorResponse` struct; DB constraint violations return sanitized messages (never raw SQL)
- List queries (`get_users`, `get_teams`, `get_roles`, `get_items`, `get_team_orders`, `get_order_items`) log a `warn!()` when a row fails to map instead of silently dropping it
- `get_user_teams` and `get_team_users` return an empty `[]` (200 OK) when no records are found, rather than a 404 error
- 4xx errors log with `warn!()`, 5xx errors log with `error!()` for color-coded severity
- Config is layered: default.yml → environment.yml → env vars (separator: `_`)
- Health endpoint (`/health`) returns HTTP 503 with `{"up": false}` when the database is unreachable, and HTTP 200 with `{"up": true}` when healthy
- Backend serves `frontend/dist/` as static files via `actix-files`, with `index_file("index.html")`
- Static files are served with a `Content-Security-Policy` header: `default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'`. The `'unsafe-inline'` directive in `script-src` is required because Trunk generates an inline `<script type="module">` to initialize the WASM module; removing it causes a white-screen failure in Chrome.

## Frontend Architecture

The frontend is a separate Rust crate (`frontend/`) compiled to WebAssembly via Trunk. It runs entirely in the browser (CSR mode).

- **Component hierarchy:** `App` → `LoginPage` / `LoadingPage` / `DashboardPage`
  - `LoginPage` uses: `LoginHeader`, `LoginForm`, `ErrorAlert`, `UsernameField`, `PasswordField`, `SubmitButton`
  - `LoadingPage`: Displayed during session restoration from stored JWT token
  - `DashboardPage` uses: `SuccessBadge`, `UserCard`
- **Page routing:** Manual via `Page` enum (`Login` / `Loading` / `Dashboard`) + Leptos signals (no router crate)
- **Auth flow:** Basic Auth POST to `/auth` → receive JWT tokens → store `access_token` and `refresh_token` in `sessionStorage` → decode JWT payload for `user_id` → GET `/api/v1.0/users/{id}` for user details → render dashboard. On logout, both access and refresh tokens are revoked server-side via `POST /auth/revoke` (fire-and-forget).
- **Session restore:** On startup, checks `sessionStorage` for existing `access_token` → shows `LoadingPage` → if token is expired, attempts refresh via `POST /auth/refresh` → validates token via user fetch → restores dashboard or falls back to login
- **Token refresh:** Transparent refresh via `try_refresh_token()` — when the access token is expired or within 60 seconds of expiry, the frontend automatically calls `POST /auth/refresh` with the stored refresh token, stores the new token pair, and retries the original request. If refresh fails, tokens are cleared and the user is redirected to login.
- **Client-side validation:** Both username and password required before form submission
- **Error display:** HTTP 401 → "Invalid username or password"; network failure → "Unable to reach the server"
- **Dev proxying:** Trunk proxies `/auth`, `/api`, `/health` to `https://127.0.0.1:8080` (configured in `Trunk.toml`)

### Frontend Conventions

- Components use `#[component]` macro and return `impl IntoView`
- Reactive state uses `ReadSignal` / `WriteSignal` pairs
- `pub` items (`JwtPayload`, `decode_jwt_payload`) are exposed via `lib.rs` for test access
- Tokens are stored in `sessionStorage` under the keys `access_token` and `refresh_token` (chosen over `localStorage` to limit token exposure — tokens are cleared when the browser tab closes, reducing the window for XSS-based token theft)
- HTTP requests use `gloo_net::http::Request` (wraps `window.fetch`); authenticated requests use `authed_get()` helper which transparently refreshes expired tokens
- `js-sys` is used for `Date::now()` to check token expiry on the client side

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
   - `cross-ref-check` — validate CLAUDE.md, commands, and migration references against disk
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
6. **Persist findings** — after presenting the plan, write **all** findings (critical, important, minor, and informational) to `.claude/assessment-findings.md` using the format described below. This file is the bridge between the assessment and the `/resume-assessment` command, which loads it in future sessions to continue work.
7. **Archive resolved items** — after updating the findings file, move all items marked `[x]` in `.claude/assessment-findings.md` to `.claude/resolved-findings.md`, organized under their original severity section (Critical, Important, Minor, Informational). Remove the moved items from `assessment-findings.md`. Update the "Last updated" date in `resolved-findings.md`.

### Assessment findings file format (`.claude/assessment-findings.md`)

When writing to the findings file, follow these rules:

- **Mark resolved items.** When an item is fixed, mark it `[x]` in its current severity section. Do not move it yet — archival happens in step 7. **Before marking any item as resolved, all project tests must pass.** Run `cargo test` (unit tests) and `make test-integration` (integration tests) — if either suite has failures, fix the regressions before marking items `[x]`. If only frontend code changed, `make test-frontend` may substitute for integration tests.
- **Update open items.** If a previously tracked `[ ]` item is still found by the current assessment, update its description, file references, and line numbers to reflect the current state of the code (lines may have shifted).
- **Remove stale items.** If a previously tracked `[ ]` item is no longer surfaced by any command (i.e., it was fixed but not checked off), mark it `[x]` with a note: "Resolved — no longer surfaced by assessment."
- **Append new items.** If the assessment surfaces new findings not already in the file, append them under the appropriate severity section and category heading (or create a new heading).
- **Update metadata.** Set the "Last assessed" date at the top of the file to the current date.
- **Preserve the file structure.** The file must always contain these sections in order: preamble with date, "How to use", "Critical Items", "Important Items", "Minor Items", "Informational Items" (each with sub-headings by category), "Completed Items" (brief pointer to resolved-findings.md), "Notes". Omit a severity section only if it has never had any items.
- **Item format.** Each item must include: checkbox (`- [ ]`), finding number and title in bold, file path and line range, problem description, fix instructions, and source command(s). Follow the format already established in the file.
- **Archive resolved items.** After updating the findings file, move all `[x]` items to `.claude/resolved-findings.md` under their original severity section. Remove the `[x]` items from `assessment-findings.md`. The resolved file uses the same section structure (Critical, Important, Minor, Informational) and item format.

This assessment must consider **all** commands in `.claude/commands/` at the time it is run, including any added after this rule was written.

## Unfinished Work

- Frontend only has login + dashboard pages; remaining pages are tracked in the **Frontend Roadmap** section
- No client-side routing library (manual signal-based page switching, by design)
- Frontend does not yet consume the team, role, item, or order APIs (auth and user-detail endpoints are consumed, including token refresh and token revocation)
- Dark/light mode toggle not yet implemented
- Toast notifications and confirmation modals not yet implemented

## Testing

### Backend

- 148 unit tests across `config`, `db::migrate`, `errors`, `from_row`, `handlers`, `middleware::auth`, `middleware::openapi`, `routes`, `server`, `validate` modules and the `healthcheck` binary
- 67 API integration tests in `tests/api_tests.rs` (require running Postgres, marked `#[ignore]`)
- 86 DB function integration tests in `tests/db_tests.rs` (require running Postgres, marked `#[ignore]`)
- Run unit tests only: `cargo test` or `make test-unit`
- Run integration tests: `make test-integration` (starts a test DB on port 5433 via `docker-compose.test.yml`, runs all ignored tests, then tears down)
- Test DB uses `docker-compose.test.yml` overlay to expose port 5433 (avoids conflicts with dev DB on 5432)

### Frontend

- 23 WASM tests in `frontend/tests/ui_tests.rs` (run in headless Chrome via `wasm-pack`)
- Test categories:
  - JWT decode (4 tests): valid token, missing segments, bad base64, invalid JSON
  - Login page rendering (3 tests): brand/form elements, email attributes, password attributes
  - Client-side validation (3 tests): empty form, email-only, password-only
  - Login flow with mocked HTTP (3 tests): success → dashboard, 401 → error, network error → message
  - Dashboard & logout (2 tests): user card structure, logout returns to login
  - Full end-to-end cycle (1 test): login → validation → success → dashboard → logout
  - Session persistence (2 tests): session persists across refresh, logout clears tokens
  - Session restore edge cases (3 tests): malformed token fallback, expired token fallback, loading page display
- Mocking strategy: overrides `window.fetch` via `js_sys::eval` to intercept `gloo-net` HTTP calls; uses `Promise`-based `setTimeout` wrapper for async timing (no `gloo-timers` dependency)
- Run frontend tests: `make test-frontend` or `cd frontend && wasm-pack test --headless --chrome`
- Note: ChromeDriver version must match installed Chrome version

### All Tests

- Run everything: `make test-all` (backend unit + integration + frontend WASM + dependency audit)
- Dependency audit: `make audit` runs `cargo audit`; `make test-all` includes it automatically via `audit-if-available`

## Required Test Runs

Before committing any changes, **all** applicable test suites must pass:

1. Run `cargo test` (unit tests — must show 0 failures)
2. Run `make test-integration` (integration tests — must show 0 failures)
3. Run `make test-frontend` (frontend WASM tests — must show 0 failures)

Do not commit if any test suite fails. If only frontend code changed, step 2 may be skipped. If only backend code changed, step 3 may be skipped.

When asked to "run all tests", run all three suites (or equivalently `make test-all`).
