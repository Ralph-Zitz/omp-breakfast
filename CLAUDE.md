# omp-breakfast — Project Context

## Overview

A breakfast ordering application for teams, built in Rust with an actix-web REST API backend and a Leptos WebAssembly single-page frontend. Users belong to teams via roles, teams can place breakfast orders composed of items. The project is used internally at LEGO (FabuLab).

## Tech Stack

- **Language:** Rust 2024 edition
- **Web framework:** actix-web 4 (with rustls TLS) + `actix-cors` for CORS policy
- **Database:** PostgreSQL via `deadpool-postgres` connection pool + `tokio-postgres`
- **ORM/mapping:** Custom `FromRow` trait in `src/from_row.rs` (manual row mapping, no external dependency); DB functions organized in `src/db/` module directory by domain
- **Auth:** JWT (access + refresh tokens via `jwt-compact`) + Basic Auth (Argon2 password hashing) + RBAC (Admin/Team Admin/Member/Guest roles, admin bypass); in-memory caching via `dashmap` (concurrent HashMap)
- **Rate limiting:** `actix-governor` on auth endpoints (6s per request, burst size 10)
- **Validation:** `validator` crate with derive macros
- **Error handling:** `thiserror` for typed error enum, `color-eyre` for colorized panic/error reports
- **Observability:** `tracing` + `tracing-subscriber` (structured JSON in prod via `fmt::layer().json()`, colorized ANSI in dev), OpenTelemetry spans, `color-eyre` SpanTrace via `tracing-error`
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
  models.rs        – All data structs (User, Team, Role, Order, Claims, State, StatusResponse, PaginationParams, PaginatedResponse)
  db/
    mod.rs         – Module declarations + re-exports of all public DB functions
    migrate.rs     – Refinery migration runner (embed_migrations! + run_migrations)
    health.rs      – Database health check (check_db)
    users.rs       – User CRUD (get_users, get_user, get_user_by_email, get_password_hash, create_user, update_user, delete_user, delete_user_by_email, count_users, bootstrap_first_user)
    teams.rs       – Team CRUD + user-team queries (get_teams, get_team, create_team, update_team, delete_team, get_user_teams, get_team_users)
    roles.rs       – Role CRUD + bootstrap (get_roles, get_role, create_role, update_role, delete_role, seed_default_roles)
    items.rs       – Item CRUD (get_items, get_item, create_item, update_item, delete_item)
    orders.rs      – Team order CRUD (get_team_orders, get_team_order, create_team_order, update_team_order, delete_team_order, delete_team_orders, reopen_team_order, count_team_orders)
    order_items.rs – Order item CRUD + closed-order check (is_team_order_closed, get_order_items, get_order_item, create_order_item, update_order_item, delete_order_item, get_order_total)
    membership.rs  – Team membership + RBAC queries (count_admins, is_admin, is_admin_or_team_admin, is_team_admin_of_user, get_member_role, check_team_access, add_team_member, remove_team_member, update_member_role, would_admins_remain_without)
    tokens.rs      – Token blacklist persistence (revoke_token_db, is_token_revoked_db, cleanup_expired_tokens)
    avatars.rs     – Avatar CRUD (get_avatars, get_avatar, insert_avatar, count_avatars, set_user_avatar)
  errors.rs        – Error enum with thiserror + ResponseError impl (maps to HTTP status codes)
  validate.rs      – Generic validation wrapper using validator crate
  routes.rs        – All route definitions with auth middleware wiring (includes /auth/register for first-user registration)
  lib.rs           – Module declarations
  handlers/
    mod.rs         – get_client() utility, health endpoint (with setup_required flag), RBAC helpers (require_admin, require_admin_or_team_admin, require_team_admin, require_team_member, require_order_owner_or_team_admin, require_self_or_admin_or_team_admin, guard_admin_role_assignment, guard_admin_demotion, guard_last_admin_membership, requesting_user_id), response helpers (created_with_location, delete_response)
    users.rs       – User CRUD + auth handlers (RBAC: self or admin) + register_first_user (first-user bootstrap)
    teams.rs       – Team CRUD + team order + member management handlers (team RBAC)
    roles.rs       – Role CRUD handlers (admin-gated CUD)
    items.rs       – Item CRUD handlers (breakfast items with prices, admin-gated CUD)
    orders.rs      – Order item CRUD handlers (items within team orders, owner/team-admin-gated)
    avatars.rs     – Avatar handlers (list, serve image, set/remove user avatar)
  middleware/
    mod.rs         – Module declarations
    auth.rs        – JWT/Basic auth validators, token generation/verification, blacklist
    openapi.rs     – OpenApi derive + Swagger UI endpoint
frontend/
  Cargo.toml       – Frontend crate config (breakfast-frontend)
  Trunk.toml       – Trunk config: output dir, watch paths, API proxies
  index.html       – Trunk HTML shell with data-trunk CSS link
  assets/          – Static assets (favicons, images) copied into dist/ by Trunk
  src/
    lib.rs         – Library entry point (pub mod api, app, components, pages)
    main.rs        – Binary entry point: mounts App to <body>
    app.rs         – Root App component, Page enum, AppShell layout, session restore
    api.rs         – HTTP helpers (authed_get/post/put/delete), JWT decode, UserContext, session storage
    components/
      mod.rs       – Module declarations + `LoadingSpinner` component, `PaginationBar` component, `role_tag_class()` CSS helper
      card.rs      – UserCard component
      icons.rs     – SVG icon components (ChevronDown, Plus, Edit, Trash, etc.)
      modal.rs     – ConfirmModal component (destructive-action confirmation dialog)
      sidebar.rs   – Sidebar + MobileHeader navigation components
      theme_toggle.rs – Dark/light mode toggle (ThemeToggle, init_theme)
      toast.rs     – Toast notification system (ToastContext, ToastRegion, show_toast)
    pages/
      mod.rs       – Module declarations
      admin.rs     – Admin dashboard page (user management, role assignment)
      dashboard.rs – Dashboard page (SuccessBadge, UserCard)
      items.rs     – Item catalog page (browse, create, edit, delete items)
      loading.rs   – Loading page (session restoration spinner)
      login.rs     – Login/registration page (LoginHeader, LoginForm, ErrorAlert, NameField, fields; dual-mode: login or first-user registration)
      orders.rs    – Order management page (team orders, line items, totals, pickup user assignment)
      order_components.rs – Order sub-components (OrderDetail, CreateOrderDialog, pickup user selector)
      profile.rs   – User profile page (view/edit profile, change password)
      roles.rs     – Role management page (view/assign roles, admin-gated)
      teams.rs     – Team management page (CRUD teams, members, team roles)
  style/
    main.css       – App-level styles using CONNECT design system tokens (--ds-* custom properties)
    bundled.css    – Concatenated CONNECT token + component CSS (built by bundle-css.sh)
    connect/
      tokens.css   – Imports CONNECT core tokens + enterprise theme from connect-design-system/
      components.css – Imports all CONNECT component CSS modules from connect-design-system/
  tests/
    ui_admin_dialogs.rs – Admin dialog WASM tests (password reset, create/edit user dialogs)
    ui_helpers.rs      – Shared test utilities (flush, mock fetch, mount helpers)
    ui_jwt.rs          – JWT decode tests
    ui_login.rs        – Login page rendering, validation, and flow tests
    ui_pages.rs        – Page rendering and navigation tests (all pages)
    ui_session.rs      – Session persistence, restore, and token refresh tests
    ui_table_styling.rs – Table CSS class and actions column tests
    ui_theme.rs        – Theme toggle tests
  bundle-css.sh    – Script to bundle CONNECT CSS into style/bundled.css
connect-design-system/ – Local clone of git@github.com:LEGO/connect-design-system.git (gitignored, read-only asset source)
config/
  default.yml      – Base config
  development.yml  – Dev overrides (local DB)
  docker-base.yml  – Sanitized base config for Docker images (all secret fields empty; supply via env vars)
  production.yml   – Prod overrides
init_dev_db.sh     – Test database initialization script (auto-discovers and applies all migration files; used only by postgres-setup in docker-compose.test.yml)
Dockerfile.breakfast – Multi-stage Docker build for the application
Dockerfile.postgres  – Custom Postgres image with init scripts
docker-compose.yml   – Development stack (app + Postgres; migrations handled by refinery on app startup)
docker-compose.test.yml – Test stack overlay (port 5433 + postgres-setup service for schema init)
frontend-issues/       – Screenshots and descriptions of UI issues (for bug reporting)
frontend-fixes/        – Documentation of UI fixes based on resolved frontend issues
LICENSE            – MIT license
Makefile           – Build, test, and dev convenience targets
minifigs/          – Pre-resized 128×128 LEGO minifigure PNG thumbnails used as user profile avatars (committed to git)
NEW-UI-COMPONENTS.md – Registry of custom UI components not available in the CONNECT design system
README.md          – Project readme
migrations/
  V1__initial_schema.sql – Refinery migration for the database schema
  V2__uuid_v7_defaults.sql – UUID v7 default migration (PostgreSQL 18+)
  V3__indexes_constraints.sql – Indexes, FK RESTRICT, NOT NULL constraints
  V4__schema_hardening.sql – Schema hardening migration
  V5__trigger_and_notnull_fixes.sql – Trigger fix on users, NOT NULL on teamorders_user_id and memberof.joined
  V6__order_constraint_and_index.sql – NOT NULL + unique constraint on orders, covering index
  V7__drop_redundant_indexes.sql – Drops redundant idx_users_email and idx_teams_name (duplicated by UNIQUE constraints)
  V8__avatars.sql – Avatars table + users.avatar_id FK column
  V9__avatar_index_and_revoked_not_null.sql – Avatar FK index + token_blacklist.revoked_at NOT NULL
  V10__guard_teamorders_team_id.sql – Guard teamorders_team_id with trigger
  V11__text_column_check_constraints.sql – CHECK constraints on text column lengths
  V12__cleanup_index_and_constraints.sql – Drop unused idx_teamorders_id_due, NOT NULL on orders_team_id
  V13__pickup_user.sql – Adds pickup_user_id column to teamorders table (FK to users, partial index)
  V14__user_text_check_constraints.sql – CHECK constraints on users.firstname (≤50), users.lastname (≤50), users.email (≤255)
  V15__restrict_cascade_fks.sql – Change memberof.memberof_user_id, teamorders.teamorders_team_id, orders.orders_team_id FKs from CASCADE to RESTRICT
tests/
  common/          – Shared test helpers (setup, state, DB utilities)
  api_auth.rs      – Auth API integration tests (login, register, refresh, revoke)
  api_avatars.rs   – Avatar API integration tests
  api_items.rs     – Item CRUD API integration tests
  api_misc.rs      – Miscellaneous API tests (health, CORS, errors)
  api_orders.rs    – Order and order-item API integration tests
  api_roles.rs     – Role CRUD API integration tests
  api_teams.rs     – Team and membership API integration tests
  api_users.rs     – User CRUD API integration tests
  db_avatars.rs    – Avatar DB function tests
  db_health.rs     – DB health check tests
  db_items.rs      – Item DB function tests
  db_membership.rs – Membership DB function tests
  db_orders.rs     – Order DB function tests
  db_roles.rs      – Role DB function tests
  db_teams.rs      – Team DB function tests
  db_tokens.rs     – Token blacklist DB function tests
  db_users.rs      – User DB function tests
```

## Key Conventions

- CORS is enforced via `actix-cors` middleware with an explicit same-origin allowlist (methods: GET/POST/PUT/DELETE/OPTIONS; headers: Authorization, Content-Type, Accept; max-age: 3600s)
- Every handler returns `Result<impl Responder, Error>` using the custom `errors::Error` enum
- DB functions take a `&Client` and return `Result<T, Error>`, using `.map_err(Error::Db)?` pattern. Functions that perform multi-step mutations (`add_team_member`, `update_member_role`, `remove_team_member`, `delete_user`, `delete_user_by_email`) take `&mut Client` and wrap operations in a database transaction.
- **Update functions must return 404 (not 500) when the target resource does not exist.** Use `query_opt()` + `.ok_or_else(|| Error::NotFound(...))` — never `query_one()`, which maps missing rows to a generic DB error (500). This is a permanent design decision; do not revert to `query_one()` in update functions.
- All handlers are instrumented with `#[instrument(..., level = "debug")]` — `state` is always skipped; handlers may also skip `req`, `json`, `basic`, `body` as appropriate
- Validation uses `validate(&json)?` before any DB call
- JWT auth uses access tokens (15min) + refresh tokens (7 days) with token rotation; on refresh, the old access token is revoked if provided in the request body (`RefreshRequest { access_token }`)
- Token revocation uses a DB-backed `token_blacklist` table (persisted across restarts) with an in-memory `dashmap::DashMap` cache for fast-path lookups. A background task runs every hour to clean up expired entries from both the database (via `db::cleanup_expired_tokens`) and the in-memory map (via `DashMap::retain()`).
- Auth cache uses TTL (5min) and max-size (1000 entries) with FIFO eviction
- Avatar cache: `DashMap<Uuid, (Vec<u8>, String)>` maps avatar_id → (image bytes, content_type). Loaded at startup from the database; pre-resized minifig PNGs from `minifigs/` are seeded into the `avatars` table on first run. Served with `Cache-Control: public, max-age=31536000, immutable`.
- Account lockout: after 5 failed login attempts within 15 minutes, the account is temporarily locked (HTTP 429). Attempts are tracked in-memory per email and cleared on successful login.
- First-user registration: `POST /auth/register` is a one-time bootstrap endpoint. It accepts a `CreateUserEntry` payload (firstname, lastname, email, password), validates that no users exist (`count_users() == 0`, else 403), creates the user, seeds the four default roles via `seed_default_roles()`, creates a "Default" bootstrap team, and assigns the new user as Admin. Rate-limited like other auth endpoints. No JWT/auth middleware required.
- RBAC: Four roles — Admin (global superuser), Team Admin (team-scoped), Member, Guest. JWT claims stored in request extensions.
- GET RBAC policy: All GET endpoints require only JWT authentication — no team-scoped RBAC. Data visibility is open to all authenticated users (no multi-tenant isolation). Team-scoped RBAC is enforced only on mutations (POST/PUT/DELETE) within individual handlers.
- Global Admin RBAC: `require_admin` helper checks if user holds "Admin" role in any team (via `db::is_admin`); gates team CUD, items CUD, roles CUD. Admin bypasses all team-scoped and self-only checks.
- Admin-or-Team-Admin RBAC: `require_admin_or_team_admin` helper checks if user holds "Admin" or "Team Admin" role in any team (via `db::is_admin_or_team_admin`); gates user creation.
- Team RBAC: `require_team_member` and `require_team_admin` helpers gate team-scoped mutations; both allow global Admin bypass. `require_team_admin` checks for "Team Admin" role in the specific team.
- Order RBAC: `require_order_owner_or_team_admin` gates single-order mutations (update, delete); allows the order creator, a Team Admin for the team, or a global Admin. Regular members and guests may only mutate their own orders.
- Pickup user RBAC: Each team order can optionally have a `pickup_user_id` — the team member responsible for collecting the order. The pickup user must belong to the same team (validated via `get_member_role` on create and update). Once a pickup user is assigned (`order.pickup_user_id.is_some()`), only a global Admin or Team Admin for the order's team may change the assignment (enforced via `require_team_admin`). First-time assignment is allowed by any team member who can update the order.
- Order Items RBAC: Creating an order item requires team membership (any role — by design, all team members may add items to a breakfast order). Updating or deleting an order item requires `require_order_owner_or_team_admin` (same as team orders) — this checks the **team order creator** (`teamorders.teamorders_user_id`), not the individual line-item contributor, because order items have no per-item `user_id` column. This is intentional: breakfast orders are collaborative, so ownership is at the order level, not the line-item level. Adding items to a closed order is blocked by `guard_open_order`.
- Admin role guard: `guard_admin_role_assignment` prevents non-admin users from assigning the "Admin" role. Called after `require_team_admin` in membership handlers (add member, update role). Only global Admins may grant Admin privileges; Team Admins may assign any other role.
- Admin demotion guard: `guard_admin_demotion` prevents non-admin users from demoting or removing a global Admin. Called after `require_team_admin` in `update_member_role` and `remove_team_member` handlers. If the target user is a global Admin, only another global Admin may change their role or remove them from a team. Team Admins cannot modify global Admins' memberships.
- Last admin guard: `guard_last_admin_membership` prevents operations that would leave zero global Admins. Called after `guard_admin_demotion` in `update_member_role` and `remove_team_member` handlers. Uses `db::would_admins_remain_without` to check whether at least one Admin would remain after excluding the target membership. Returns 403 if the operation would orphan the system.
- Self-or-Admin-or-Team-Admin RBAC: `require_self_or_admin_or_team_admin` helper gates user mutations (update, delete); allows the user themselves, a global Admin, or a Team Admin of any team where the target user is also a member (checked via `db::is_team_admin_of_user` — a self-join on `memberof`).
- `Error::Forbidden` variant maps to HTTP 403 for authorization failures
- `Error::Unauthorized` variant maps to HTTP 401 for authentication failures
- Production safety: server panics at startup if `server.secret` or `server.jwtsecret` is still the default value when `ENV=production`, if `pg.user` or `pg.password` is still the default `actix`, or if `pg.host` is still the placeholder `pick.a.proper.hostname`
- Error responses are JSON `{"error": "..."}` via `ErrorResponse` struct; DB constraint violations return sanitized messages (never raw SQL)
- **Pagination:** All list endpoints accept `?limit=` and `?offset=` query parameters via `PaginationParams` (default limit 50, max 100, offset ≥ 0). Responses are wrapped in `PaginatedResponse<T>` with `items`, `total`, `limit`, `offset` fields. DB list functions return `(Vec<T>, i64)` where the second element is the total count from a `SELECT COUNT(*)` query. Sanitization via `PaginationParams::sanitize()` clamps values to valid ranges.
- List queries (`get_users`, `get_teams`, `get_roles`, `get_items`, `get_team_orders`, `get_order_items`) log a `warn!()` when a row fails to map instead of silently dropping it
- `get_user_teams` and `get_team_users` return an empty `[]` (200 OK) when no records are found, rather than a 404 error
- 4xx errors log with `warn!()`, 5xx errors log with `error!()` for color-coded severity
- Config is layered: default.yml → environment.yml → env vars (prefix: `BREAKFAST_`, separator: `_`)
- Health endpoint (`/health`) returns HTTP 503 with `{"up": false}` when the database is unreachable, and HTTP 200 with `{"up": true, "setup_required": <bool>}` when healthy. `setup_required` is `true` when no users exist in the database (first-user registration needed).
- Backend serves `frontend/dist/` as static files via `actix-files`, with `index_file("index.html")`
- Static files are served with a `Content-Security-Policy` header: `default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' https://assets.lego.com; connect-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'`. The `'unsafe-inline'` directive in `script-src` is required because Trunk generates an inline `<script type="module">` to initialize the WASM module; removing it causes a white-screen failure in Chrome. The `font-src` directive includes `https://assets.lego.com` to allow loading the LEGO Typewell proprietary font from the LEGO CDN.
- Security headers: `Strict-Transport-Security` (HSTS with `preload`), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()` are set globally via `DefaultHeaders`
- Password hashing uses explicit Argon2id parameters (`Algorithm::Argon2id`, `Version::V0x13`, `Params::new(47104, 1, 1, None)` — OWASP recommended: 46 MiB memory, 1 iteration, 1 lane) rather than `Argon2::default()` to prevent silent weakening via crate updates

## Frontend Architecture

The frontend is a separate Rust crate (`frontend/`) compiled to WebAssembly via Trunk. It runs entirely in the browser (CSR mode). The codebase is organized into modules:

- `api.rs` — HTTP client helpers, JWT decoding, `UserContext` builder, session storage utilities, `HealthResponse` struct, `check_setup_required()` for first-user detection
- `app.rs` — Root `App` component, `Page` enum, `AppShell` layout, session restore logic
- `components/` — Reusable UI components (card, icons, modal, sidebar, theme toggle, toast)
- `pages/` — Page-level components (one file per page)

- **Component hierarchy:** `App` → `LoginPage` / `LoadingPage` / `AppShell`
  - `AppShell` uses: `MobileHeader`, `Sidebar`, `ToastRegion`, and routes to page components
  - `LoginPage` uses: `LoginHeader`, `LoginForm`, `ErrorAlert`, `NameField`, `UsernameField`, `PasswordField`, `SubmitButton` (dual-mode: login or first-user registration)
  - `LoadingPage`: Displayed during session restoration from stored JWT token
  - `DashboardPage` uses: `SuccessBadge`, `UserCard`
  - `TeamsPage`, `OrdersPage`, `ItemsPage`, `ProfilePage`, `AdminPage`, `RolesPage`: Full CRUD pages with forms, tables, modals, and toast notifications
- **Page routing:** Manual via `Page` enum (`Loading` / `Login` / `Dashboard` / `Teams` / `Orders` / `Items` / `Profile` / `Admin` / `Roles`) + Leptos signals (no router crate). `AppShell` wraps all authenticated pages with sidebar navigation.
- **First-user registration:** On load, the login page checks `GET /health` for `setup_required: true`. If true, shows a registration form (first name, last name, email, password) that `POST`s to `/auth/register` to create the first admin account, then auto-logs in. If false, shows the standard login form.
- **Auth flow:** Basic Auth POST to `/auth` → receive JWT tokens → store `access_token` and `refresh_token` in `sessionStorage` → decode JWT payload for `user_id` → GET `/api/v1.0/users/{id}` for user details → render dashboard. On logout, both access and refresh tokens are revoked server-side via `POST /auth/revoke` (fire-and-forget).
- **Session restore:** On startup, checks `sessionStorage` for existing `access_token` → shows `LoadingPage` → if token is expired, attempts refresh via `POST /auth/refresh` → validates token via user fetch → restores dashboard or falls back to login
- **Token refresh:** Transparent refresh via `try_refresh_token()` — when the access token is expired or within 60 seconds of expiry, the frontend automatically calls `POST /auth/refresh` with the stored refresh token and the old access token in the request body (so the server can revoke it immediately), stores the new token pair, and retries the original request. If refresh fails, tokens are cleared and the user is redirected to login.
- **Client-side validation:** Both username and password required before form submission; registration mode also validates first/last name (2–50 chars) and password length (≥ 8)
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
- **Component reuse:** Always prefer existing CONNECT Design System components from `connect-design-system/` before creating custom ones. If a required component is missing from the design system, create it under `frontend/src/` following the same folder structure and naming conventions. Any newly created UI component must be documented in `NEW-UI-COMPONENTS.md` at the project root (component name, purpose, props, and rationale for why an existing CONNECT component was not suitable).
- **Table column alignment:** All table header cells (`<th class="connect-table-header-cell">`) must be left-aligned (`text-align: left`) so that header text aligns horizontally with the data below it. All table cells (both `th` and `td`) must use `vertical-align: middle` for consistent vertical placement of content. Apply these rules globally in `frontend/style/main.css` — do not add inline styles to individual tables.
- **Actions column spacing:** Table columns that contain action buttons (edit, delete, etc.) must use the `connect-table-header-cell--actions` / `connect-table-cell--actions` modifier classes and must never have a hard-coded `width` that is too narrow for their button content. The actions column width must adapt to its content (`width: auto`) and buttons must have a gap between them (`gap: var(--ds-layout-spacing-100, 8px)`). A fixed width that clips or wraps buttons breaks the horizontal separator line above the row and must not be used.

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

## CONNECT Design System

The frontend UI is built on the LEGO CONNECT Design System. A local clone of the design system repository lives at `connect-design-system/` in the project root (gitignored — not committed to this repo). It serves as a **read-only asset source** for CSS tokens, component styles, and SVG icons.

- **Source repo:** `git@github.com:LEGO/connect-design-system.git`
- **Local path:** `connect-design-system/` (added to `.gitignore`)
- **Token system:** All CSS custom properties use the `--ds-{category}-{subcategory}-{variant}-{state}` naming convention
- **Component CSS:** Class names follow `.connect-{component}--{modifier}` (modified BEM), imported from `.module.css` files in `connect-components-styles`
- **Theme:** Enterprise theme (`connect-theme-enterprise`) with light/dark mode via `data-mode` attribute and `@media (prefers-color-scheme)`
- **Typography:** LEGO Typewell proprietary font loaded from `https://assets.lego.com/fonts/v6/typewell/` CDN via `@font-face` declarations; Noto Sans and system-ui as fallbacks
- **Icons:** 1,048 SVG icons available in `connect-design-system/packages/icons/src/svgs/` (40×40 viewBox, `fill="currentColor"`)
- **CSS imports:** `frontend/style/connect/tokens.css` imports core tokens + enterprise theme; `frontend/style/connect/components.css` imports all 48+ component style modules. Both use relative `@import` paths into `connect-design-system/`.

### Keeping the design system up to date

The design system clone is updated automatically during project assessments (see below). To update manually:

```bash
cd connect-design-system && git pull
```

After pulling, check for CSS token renames, new/removed component classes, or changed `@font-face` URLs that may require frontend CSS or component updates.

If the `connect-design-system/` directory does not exist (fresh checkout), clone it:

```bash
git clone git@github.com:LEGO/connect-design-system.git connect-design-system
```

## Project Assessment

When asked to **assess the project** (or "project assessment"), perform the following:

1. Run every command defined in `.claude/commands/` against the current codebase. **Each command must be executed in a dedicated subagent** (one subagent per command). The subagent prompt must include the full contents of the command file and instruct the subagent to perform the analysis and return its findings. **Parallelise subagent invocations where possible** — since each command is independent and read-only, launch as many concurrently as the tooling allows. Commands:
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
   - `resume-assessment` — loads assessment-findings.md and continues unfinished work
2. **Update the CONNECT Design System** — run `cd connect-design-system && git pull` to fetch the latest upstream changes. If `git pull` reports new commits:
   - Diff the incoming changes (`git log --oneline HEAD@{1}..HEAD` and `git diff HEAD@{1}..HEAD -- packages/`).
   - Identify any CSS token renames/removals, component class changes, new components, `@font-face` URL updates, or icon additions/removals that affect the frontend.
   - If breaking or noteworthy changes are found, include a **Design System Migration** section in the assessment output listing each change with its impact on the frontend and a concrete migration plan (same approach used for the initial migration: map old → new tokens/classes, update `frontend/style/` imports, update component markup in `frontend/src/app.rs`, update test selectors in `frontend/tests/ui_*.rs`).
   - If `git pull` reports "Already up to date", note this in the assessment and skip the migration section.
   - If the `connect-design-system/` directory does not exist, clone it: `git clone git@github.com:LEGO/connect-design-system.git connect-design-system`.
3. Collect all findings that indicate actionable changes (bugs, missing implementations, convention violations, security issues, stale dependencies, etc.)
4. **Cross-check against resolved findings** — before finalising the findings list, read `.claude/resolved-findings.md` and verify that no new finding re-introduces a previously resolved issue. For every candidate finding:
   - Search resolved-findings.md for the same file/function/pattern.
   - If a resolved item already covers the same concern **and the current code still reflects the resolution**, discard the candidate (it is a false positive).
   - If a resolved item covers the concern but **the code has regressed** (the fix was reverted or broken by a later change), flag it explicitly as a **regression** with a reference to the original resolved finding number.
   - If a candidate finding **contradicts** a resolved item's fix (e.g., recommending the opposite change), discard the candidate and note the conflict in the assessment notes.
5. Present a single consolidated plan grouped by category, listing each proposed change with:
   - Which command surfaced it
   - What needs to change and where
   - Severity (critical / important / minor / informational)
6. **Do not apply any changes** — only present the plan for approval
7. If no actionable findings are discovered, state that the project is in good shape
8. **Persist findings** — after presenting the plan, write **all** findings (critical, important, minor, and informational) to `.claude/assessment-findings.md` using the format described below. This file is the bridge between the assessment and the `/resume-assessment` command, which loads it in future sessions to continue work.
9. **Archive resolved items** — after updating the findings file, move all items marked `[x]` in `.claude/assessment-findings.md` to `.claude/resolved-findings.md`, organized under their original severity section (Critical, Important, Minor, Informational). Remove the moved items from `assessment-findings.md`. Update the "Last updated" date in `resolved-findings.md`.

### Assessment findings file format (`.claude/assessment-findings.md`)

When writing to the findings file, follow these rules:

- **Mark resolved items.** When an item is fixed, mark it `[x]` in its current severity section. Do not move it yet — archival happens in step 8. **Before marking any item as resolved, all project tests must pass.** Run `cargo test` (unit tests) and `make test-integration` (integration tests) — if either suite has failures, fix the regressions before marking items `[x]`. If only frontend code changed, `make test-frontend` may substitute for integration tests.
- **Update open items.** If a previously tracked `[ ]` item is still found by the current assessment, update its description, file references, and line numbers to reflect the current state of the code (lines may have shifted).
- **Remove stale items.** If a previously tracked `[ ]` item is no longer surfaced by any command (i.e., it was fixed but not checked off), mark it `[x]` with a note: "Resolved — no longer surfaced by assessment."
- **Append new items.** If the assessment surfaces new findings not already in the file, append them under the appropriate severity section and category heading (or create a new heading).
- **Update metadata.** Set the "Last assessed" date at the top of the file to the current date.
- **Preserve the file structure.** The file must always contain these sections in order: preamble with date, "How to use", "Critical Items", "Important Items", "Minor Items", "Informational Items" (each with sub-headings by category), "Completed Items" (brief pointer to resolved-findings.md), "Notes". Omit a severity section only if it has never had any items.
- **Item format.** Each item must include: checkbox (`- [ ]`), finding number and title in bold, file path and line range, problem description, fix instructions, and source command(s). Follow the format already established in the file.
- **Archive resolved items.** After updating the findings file, move all `[x]` items to `.claude/resolved-findings.md` under their original severity section. Remove the `[x]` items from `assessment-findings.md`. The resolved file uses the same section structure (Critical, Important, Minor, Informational) and item format.

This assessment must consider **all** commands in `.claude/commands/` at the time it is run, including any added after this rule was written.

## Unfinished Work

- No client-side routing library (manual signal-based page switching, by design)
- Frontend WASM tests cover all pages with rendering and basic interaction tests (85 tests); deeper workflow and edge-case tests for individual pages are still missing

## Testing

### Backend

- 248 unit tests across `config`, `db::migrate`, `errors`, `from_row`, `handlers`, `middleware::auth`, `middleware::openapi`, `models`, `routes`, `server`, `validate` modules and the `healthcheck` binary
- 168 API integration tests in `tests/api_*.rs` (require running Postgres, marked `#[ignore]`)
- 120 DB function integration tests in `tests/db_*.rs` (require running Postgres, marked `#[ignore]`)
- Run unit tests only: `cargo test` or `make test-unit`
- Run integration tests: `make test-integration` (starts a test DB on port 5433 via `docker-compose.test.yml`, runs all ignored tests, then tears down)
- Test DB uses `docker-compose.test.yml` overlay to expose port 5433 (avoids conflicts with dev DB on 5432)

### Frontend

- 85 WASM tests in `frontend/tests/ui_*.rs` (run in headless Chrome via `wasm-pack`)
- Test categories:
  - JWT decode (4 tests): valid token, missing segments, bad base64, invalid JSON
  - Login page rendering (3 tests): brand/form elements, email attributes, password attributes
  - Client-side validation (3 tests): empty form, email-only, password-only
  - Login flow with mocked HTTP (3 tests): success → dashboard, 401 → error, network error → message
  - Dashboard & logout (2 tests): user card structure, logout returns to login
  - Full end-to-end cycle (1 test): login → validation → success → dashboard → logout
  - Session persistence (2 tests): session persists across refresh, logout clears tokens
  - Session restore edge cases (3 tests): malformed token fallback, expired token fallback, loading page display
  - Token refresh retry (2 tests): authed_get retry after 401, token stored after refresh
  - authed_get double-failure (2 tests): retry after 401 fails, double-failure falls back to login
  - Theme toggle (4 tests): dark/light mode switch, round-trip toggle, ARIA attributes
  - Page rendering (14 tests): TeamsPage (2), ItemsPage (2), OrdersPage (2), ProfilePage (2 + team memberships), AdminPage (2), RolesPage (2) — navigation, data rendering, admin visibility
  - Login error differentiation (2 tests): 429 rate limit message, 500 server error message
  - Table styling (4 tests): connect-table-header-cell class on admin, items, roles, teams tables
  - Actions column (6 tests): actions modifier classes (3), no narrow inline width (2), multiple buttons present (2)
  - Admin password reset (10 tests): button visibility, dialog open/close, validation (empty, short, mismatch), success toast
  - Shared components (4 tests): toast region, sidebar nav items, sidebar active state, confirm modal structure
  - Orders page interactions (1 test): create order dialog opens
  - Profile page interactions (3 tests): edit mode toggle, password field reveal, cancel exits edit
  - Admin dialogs (7 tests): CreateUserDialog open/fields/disabled/cancel, EditUserDialog open/fields/cancel
  - First-user registration (3 tests): registration form renders when setup_required, short password validation error, successful registration redirects to dashboard
  - authed_request mutations (3 tests): POST sends body and auth header, PUT sends body and auth header, DELETE sends auth header without body
- Mocking strategy: overrides `window.fetch` via `js_sys::eval` to intercept `gloo-net` HTTP calls; uses `Promise`-based `setTimeout` wrapper for async timing (no `gloo-timers` dependency)
- Run frontend tests: `make test-frontend` or `cd frontend && wasm-pack test --headless --chrome`
- Note: ChromeDriver version must match installed Chrome version

### All Tests

- Run everything: `make test-all` (backend unit + integration + frontend WASM + dependency audit)
- Dependency audit: `make audit` runs `cargo audit`; `make test-all` includes it automatically via `audit-if-available`.

## Required Test Runs

Before committing any changes, **all** applicable test suites must pass:

1. Run `cargo fmt --all` to format all Rust source files (backend + frontend) before staging
2. Run `cargo test` (unit tests — must show 0 failures)
3. Run `make test-integration` (integration tests — must show 0 failures)
4. Run `make test-frontend` (frontend WASM tests — must show 0 failures)

Do not commit if any test suite fails. If only frontend code changed, step 3 may be skipped. If only backend code changed, step 4 may be skipped.

Always run `cargo fmt --all` regardless of which files changed — the formatter must run before `git add`.

When asked to "run all tests", run all three suites (or equivalently `make test-all`).
