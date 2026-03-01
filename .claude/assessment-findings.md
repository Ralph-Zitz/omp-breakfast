# Assessment Findings

Last assessed: 2026-03-01 (updated — #77/#78 new; all prior open items verified)

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will preserve completed items, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

_No critical items remaining._

## Important Items

_No important items remaining._

## Minor Items

### Code Quality — Dead S3 Config Fields

- [ ] **#59 — `s3_key_id` and `s3_key_secret` are loaded and stored but never used**
  - Files: `src/models.rs` lines 46–47 (`State` struct), `src/config.rs` lines 13–14 (`ServerConfig` struct), `src/server.rs` lines 224–225 (state construction), `config/default.yml` lines 6–7, `config/development.yml` lines 5–6, `config/production.yml` lines 6–7
  - Problem: The `s3_key_id` and `s3_key_secret` fields are defined in `ServerConfig`, loaded from config files, stored in `State`, and propagated through all test helpers (`routes.rs`, `server.rs`, `middleware/auth.rs`), but no handler, middleware, or DB function ever reads them. They add confusion and unnecessary config surface area.
  - Fix: Either remove the fields entirely from `ServerConfig`, `State`, all config files, and all test helpers — or, if S3 integration is planned, document the intent in CLAUDE.md's Unfinished Work section.
  - Source commands: `review`, `practices-audit`

### Code Quality — Dead `database.url` Config Field

- [ ] **#68 — `database.url` field in `Settings` is configured but unused**
  - Files: `src/config.rs` lines 19–22 (`Database` struct with `#[allow(dead_code)]`), `config/default.yml` lines 10–11, `config/development.yml` lines 1–2
  - Problem: The `Database` struct contains a single `url` field marked `#[allow(dead_code)]`. The DB pool is created from the `pg.*` config fields (`deadpool_postgres::Config`), not from `database.url`. The only reference to `database.url` is in a config unit test assertion (`config.rs` line 150). The field and its config entries serve no purpose and could confuse developers.
  - Fix: Remove the `Database` struct and its `database` field from `Settings`. Remove `database:` sections from `config/default.yml` and `config/development.yml`. Update the config test to remove the `database.url` assertion.
  - Source commands: `review`, `practices-audit`

### Database — UUID Version Mismatch Between Schema and Application

- [ ] **#69 — Schema defaults to UUID v4 but Rust code generates UUID v7**
  - Files: `database.sql` lines 25, 41, 53, 62, 88 (`uuid_generate_v4()`), `src/handlers/mod.rs` line 151 and test helpers (`Uuid::now_v7()`)
  - Problem: The `database.sql` schema sets `DEFAULT uuid_generate_v4()` (random UUIDs) on all primary key columns, but the Rust application generates `Uuid::now_v7()` (time-ordered UUIDs). Since server-generated IDs are passed via `INSERT ... VALUES` and returned via `RETURNING`, the DB default is never triggered in practice. However, the inconsistency is confusing and would cause mixed UUID versions if rows were ever inserted directly via SQL.
  - Fix: Update `database.sql` to use `uuid_generate_v7()` (available in PostgreSQL 17+) or `gen_random_uuid()` as the default — or document the intentional mismatch. If using PostgreSQL < 17, keep `uuid_generate_v4()` and add a comment noting that the application overrides it with v7.
  - Source commands: `db-review`, `review`

### Security — Seed Data Uses Hardcoded Argon2 Salt

- [ ] **#70 — All seed users share the same Argon2 hash with a hardcoded salt**
  - File: `database.sql` lines 215–227 (INSERT statements for 5 seed users)
  - Problem: All 5 seed users have identical Argon2id password hashes using the salt `dGVzdHNhbHQxMjM0NTY` (base64 for `testsalt123456`). The password for all users (including admin) is `"Very Secret"` — the same as the default JWT secret in `config/default.yml`. While documented as dev-only and mitigated by the production startup panic check on secrets, this creates risk if the seed script is accidentally run against a production database.
  - Fix: Add a prominent `-- WARNING: DO NOT RUN IN PRODUCTION` comment at the top of the seed data section. Consider generating unique hashes with random salts for each seed user. Alternatively, separate seed data into a `database-seed.sql` file that is explicitly excluded from production deployment scripts.
  - Source commands: `security-audit`, `db-review`

### Frontend — All Components in Single `app.rs` File

- [ ] **#71 — Frontend `app.rs` is a 597-line monolith**
  - File: `frontend/src/app.rs` (597 lines — all components, auth logic, API calls, types)
  - Problem: The entire frontend — all components (`App`, `LoginPage`, `DashboardPage`, `LoadingPage`, and 6 sub-components), auth helpers (`try_refresh_token`, `authed_get`, `revoke_token_server_side`), JWT decoding, and type definitions — lives in a single file. As the 6 planned pages (Team Management, Orders, Items, Profile, Admin, Roles) are built, this file will become unmanageable.
  - Fix: When building the next frontend page, split into a module structure: `app/mod.rs` (App + router), `app/auth.rs` (token logic), `app/types.rs` (shared types), `app/pages/login.rs`, `app/pages/dashboard.rs`, `app/pages/loading.rs`, `app/components/` (reusable sub-components). This is tracked as part of the Frontend Roadmap in CLAUDE.md.
  - Source commands: `review`, `practices-audit`

### Security — No Account Lockout After Failed Auth Attempts

- [ ] **#73 — Failed authentication is rate-limited but no lockout policy exists**
  - Files: `src/routes.rs` (rate limiter config on `/auth`), `src/handlers/users.rs` (`auth_user` handler)
  - Problem: The `/auth` endpoint has rate limiting (6s per request, burst 10) via `actix-governor`, but there is no account-level lockout after N consecutive failures. An attacker can continue brute-forcing at the rate limit's pace indefinitely. For an internal app this is low-risk, but it's a gap against password spraying.
  - Fix: Track failed login attempts per email in the database or in-memory cache. Lock the account (or add exponential backoff) after a threshold (e.g., 5 failures in 15 minutes). Add an admin endpoint or background task to unlock accounts.
  - Source commands: `security-audit`

### Deployment — Production Config Has Placeholder Hostname

- [ ] **#75 — `config/production.yml` uses `pick.a.proper.hostname` as the PG host**
  - File: `config/production.yml` line 2
  - Problem: The production config uses a placeholder string `pick.a.proper.hostname` for the PostgreSQL host. If deployed without overriding via environment variable (`BREAKFAST_PG_HOST`), the application will fail to connect to the database with a DNS resolution error. There is no startup validation that catches this.
  - Fix: Add a startup check in `server.rs` (similar to the existing secret-validation panic) that verifies `pg.host` is not `pick.a.proper.hostname` when `ENV=production`. Alternatively, remove the placeholder and require the env var to be set.
  - Source commands: `practices-audit`, `review`

### Documentation — CLAUDE.md Test Count Breakdowns Are Stale

- [ ] **#77 — Test count breakdowns in CLAUDE.md and assessment-findings.md are incorrect**
  - Files: `CLAUDE.md` lines 256–264 (Testing section), `.claude/assessment-findings.md` Notes section
  - Problem: The `from_row` test count is listed as 11 but the actual count is 10 (verified by enumerating `#[test]` attributes in `src/from_row.rs` lines 315–425). This causes the per-module breakdown to sum to 137, contradicting the stated total of 136. Additionally, the WASM test count is listed as 22 but the actual count is 23 — the `test_authed_get_double_failure_falls_back_to_login` test added in completed item #74 was not reflected in the count.
  - Fix: Update `CLAUDE.md` Testing section: change `from_row` from 11 to 10 in the breakdown, and change "22 WASM tests" to 23. Update the Notes section of this findings file to match. The backend total of 136 is correct.
  - Source commands: `practices-audit`

## Informational Items

### Dependencies — Unfixable RSA Advisory

- [ ] **#55 — `rsa` 0.9.10 has an unfixable timing side-channel advisory (RUSTSEC-2023-0071)**
  - Problem: The `rsa` crate (transitive dependency via `jsonwebtoken` 10.3.0) is affected by the Marvin Attack (CVSS 5.9, medium severity), a timing side-channel that could allow key recovery during RSA decryption. No patched version is available upstream. Since this project uses HMAC-based JWT signing (not RSA key exchange), the practical risk is negligible. This item will remain open until `jsonwebtoken` updates its `rsa` dependency or a patched `rsa` release is published.
  - Source command: `dependency-check` (cargo audit)
  - Action: No code changes possible — waiting on upstream fix. Monitor `jsonwebtoken` releases.

### Documentation — Test Count Maintenance Burden

- [ ] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md` lines 256–258, 264
  - Problem: CLAUDE.md hard-codes specific test counts (136 unit, 65 API integration, 86 DB, 23 WASM). These go stale every time a test is added or removed. The counts were verified as of 2026-03-01, but they will drift again with the next code change that adds tests.
  - Source command: `practices-audit`
  - Action: No code change needed. This is an inherent maintenance cost of documenting exact counts. The assessment process updates them each time it runs.

### API Design — No Pagination on List Endpoints

- [ ] **#61 — List endpoints return all records without pagination**
  - Files: `src/db/` (`users::get_users`, `teams::get_teams`, `roles::get_roles`, `items::get_items`, `orders::get_team_orders`, `order_items::get_order_items`), `src/handlers/` (corresponding GET collection handlers)
  - Problem: All list/collection endpoints (`GET /api/v1.0/users`, `GET /api/v1.0/teams`, `GET /api/v1.0/items`, `GET /api/v1.0/roles`, `GET /api/v1.0/teams/{id}/orders`, `GET /api/v1.0/teams/{id}/orders/{id}/items`) return all rows from the database. This works at current scale (internal team app) but would become a performance problem with growth. Standard REST pagination (e.g., `?page=1&limit=20` or cursor-based) is not implemented.
  - Source commands: `review`, `api-completeness`
  - Action: No immediate change needed for current usage. When implementing pagination, add `limit`/`offset` parameters to DB functions, query parameter extraction in handlers, and pagination metadata in responses. Update OpenAPI annotations to document query parameters.

### Deployment — No `.env.example` File for Onboarding

- [ ] **#76 — No `.env.example` or env documentation for new developers**
  - Problem: The project uses environment variables for configuration overrides (`BREAKFAST_*` prefix, `ENV` variable), but there is no `.env.example` file or dedicated environment variable documentation. New developers must read `CLAUDE.md`, `config/*.yml`, and `docker-compose.yml` to discover which variables are available. The `CLAUDE.md` Config section mentions the layering strategy but doesn't enumerate available env vars.
  - Source commands: `practices-audit`
  - Action: Create a `.env.example` file listing all available environment variables with comments explaining their purpose, default values, and which config field they override. Reference it in the README.

## Completed Items

Items moved here after being resolved:

### Deployment — Database Migration Tool Adopted

- [x] **#66 — Schema managed via destructive `DROP TABLE` DDL script**
  - Files: `Cargo.toml` (added `refinery`), `migrations/V1__initial_schema.sql` (new), `src/db/migrate.rs` (new), `src/db/mod.rs`, `src/server.rs`, `database.sql`
  - Resolution: Adopted `refinery` 0.8 with `tokio-postgres` feature for versioned database migrations. Created `migrations/V1__initial_schema.sql` capturing the full schema with `IF NOT EXISTS` / `OR REPLACE` for idempotency. Created `src/db/migrate.rs` module using `embed_migrations!` macro. Added migration execution at server startup (after pool creation, before accepting requests) — the server refuses to start if migrations fail. Added prominent `WARNING: DEVELOPMENT AND TESTING ONLY` header to `database.sql`. Dev/test workflow is unchanged: `database.sql` (with DROPs + seed data) is still used by docker-compose for clean-slate setup; the app's migrations are idempotent against an already-created schema. Production deployments now use incremental migrations — future schema changes go into `V2__*.sql`, `V3__*.sql`, etc.
  - Source commands: `db-review`, `security-audit`

### Security — In-Memory Token Blacklist Eviction

- [x] **#67 — `token_blacklist` in-memory DashMap has no eviction or size limit**
  - Files: `src/models.rs` (State struct), `src/middleware/auth.rs` (revoke_token, is_token_revoked, test helpers), `src/server.rs` (spawn_token_cleanup_task)
  - Resolution: Changed `token_blacklist` DashMap value type from `bool` to `DateTime<Utc>` to store each revoked token's expiry time. Updated `revoke_token()` to store the actual `expires_at` value. Updated `is_token_revoked()` DB-fallback cache population to use a conservative expiry estimate (max refresh token lifetime of 7 days). Rewrote `spawn_token_cleanup_task` to accept `Data<State>` instead of just `Pool`, and added `state.token_blacklist.retain(|_, expires_at| *expires_at > now)` to evict expired entries from the in-memory map on each cleanup cycle (every hour). The cleanup task now logs both `db_deleted` and `memory_evicted` counts plus `memory_remaining` size. Updated test helper `revoke_token_in_memory` to insert a `DateTime<Utc>` value.
  - Source commands: `security-audit`, `review`

### Documentation — Command Files Reference Stale `src/db.rs` Path

- [x] **#78 — Three `.claude/commands/*.md` files reference `src/db.rs` which is now `src/db/`**
  - Files: `.claude/commands/db-review.md`, `.claude/commands/api-completeness.md`, `.claude/commands/test-gaps.md`
  - Resolution: Updated all three command files to reference `src/db/` (the module directory) instead of the old monolithic `src/db.rs` file. Updated all instances: heading, inline references, scope sections.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md CSP Policy Not Documented

- [x] **#57 — CLAUDE.md Key Conventions should document the CSP header on static files**
  - File: `CLAUDE.md` line 118 (Key Conventions section)
  - Resolution: Added a new bullet to Key Conventions documenting the full `Content-Security-Policy` header value and explaining why `'unsafe-inline'` in `script-src` is required for Trunk's inline WASM bootstrap `<script type="module">`. Notes that removing it causes a white-screen failure in Chrome.
  - Source commands: `practices-audit`, `security-audit`

### Frontend — Loading Page Spinner CSS Missing

- [x] **#58 — `LoadingPage` component references undefined CSS classes**
  - File: `frontend/style/main.css`
  - Resolution: Added CSS rules for `.loading-page` (background), `.loading-card` (flex column centered layout with padding), `.loading-spinner` (40×40px animated spinner using `var(--color-border)` and `var(--color-primary)` with the existing `spin` keyframes), and `.loading-text` (muted text styling). All 22 WASM frontend tests pass.
  - Source commands: `review`, `practices-audit`

### Security — actix-files CVE (Verified Patched)

- [x] **#56 — `actix-files` had 2 known CVEs (GHSA-8v2v-wjwg-vx6r, GHSA-gcqf-3g44-vc9p)**
  - Problem: `actix-files` versions prior to 0.6.10 are vulnerable to information exposure when serving a non-existing folder (GHSA-8v2v-wjwg-vx6r, medium severity) and panic on empty `Range` header (GHSA-gcqf-3g44-vc9p, medium severity).
  - Resolution: Verified that `Cargo.lock` already pins `actix-files` at version 0.6.10 (the fixed version). The `Cargo.toml` spec `"0.6"` (equivalent to `>=0.6.0, <0.7.0`) resolved to the patched version via `cargo update` in a prior session. No code changes required.
  - Source commands: `dependency-check`, `security-audit`

### Security — Password Hashing at User Creation

- [x] **#40 — `create_user` stores plaintext password instead of Argon2 hash**
  - File: `src/db.rs` lines 68–93
  - Resolution: Already fixed in a prior session — `db::create_user` now hashes the password with `Argon2::default()` + `SaltString::generate(&mut OsRng)` before inserting, matching the pattern in `db::update_user`. Verified by the new `create_user_then_authenticate_round_trip` integration test (#44).
  - Source commands: `db-review`, `security-audit`

### Documentation — CLAUDE.md Stale After Recent Changes

- [x] **#41 — Test counts in CLAUDE.md are stale**
  - File: `CLAUDE.md`
  - Resolution: Updated all test count references — 79 unit tests, 65 API integration tests, 22 WASM tests. Counts now match actual test output.
  - Source command: `practices-audit`

- [x] **#42 — `Error::Unauthorized` variant not documented in CLAUDE.md**
  - File: `CLAUDE.md` Key Conventions section
  - Resolution: Added bullet: "`Error::Unauthorized` variant maps to HTTP 401 for authentication failures" after the existing `Error::Forbidden` bullet.
  - Source command: `practices-audit`

- [x] **#43 — Unfinished Work section does not reflect frontend token revocation**
  - File: `CLAUDE.md` Unfinished Work + Frontend Architecture sections
  - Resolution: Updated Unfinished Work to mention "token revocation" alongside "token refresh". Updated Auth flow description in Frontend Architecture to note: "On logout, both access and refresh tokens are revoked server-side via `POST /auth/revoke` (fire-and-forget)."
  - Source commands: `practices-audit`, `api-completeness`

### Test Gaps

- [x] **#44 — No integration test for create-user → authenticate round-trip**
  - File: `tests/api_tests.rs` — `create_user_then_authenticate_round_trip`
  - Resolution: Added integration test that: (1) creates a user via `POST /api/v1.0/users`, (2) authenticates via `POST /auth` with Basic Auth, (3) asserts 200 with valid token pair, (4) uses the token to fetch the user's own profile, (5) cleans up. All 65 API integration tests pass.
  - Source command: `test-gaps`

### Backend — Error Response Consistency

- [x] **#15 — `auth_user` returns bare string instead of `ErrorResponse`**
  - File: `src/handlers/users.rs`
  - Resolution: Replaced `Ok(HttpResponse::Unauthorized().json("Unauthorized"))` with `Err(Error::Unauthorized("Unauthorized".to_string()))`, routing through the centralized `ResponseError` impl with proper `warn!()` logging.
  - Source command: `review`

- [x] **#16 — `refresh_token` handler bypasses centralized error handling**
  - File: `src/handlers/users.rs`
  - Resolution: Added `Error::Unauthorized(String)` variant to `errors::Error` enum (maps to HTTP 401 with `warn!()` logging). Replaced both `Ok(HttpResponse::Unauthorized().json(ErrorResponse { ... }))` returns (invalid token type and revoked token cases) with `Err(Error::Unauthorized(...))`. Added unit test `unauthorized_error_returns_401`.
  - Source command: `review`

### Frontend — Token Revocation on Logout

- [x] **#1 — Frontend logout does not revoke tokens server-side**
  - File: `frontend/src/app.rs` — `on_logout` closure in `DashboardPage`
  - Resolution: Added `revoke_token_server_side` async helper that POSTs to `/auth/revoke` with the bearer token and token-to-revoke body. Updated `on_logout` to grab both tokens before clearing `sessionStorage`, then fire-and-forget revocation of both access and refresh tokens via `leptos::task::spawn_local`. Storage is cleared immediately so logout always succeeds regardless of network outcome.
  - Source commands: `api-completeness`, `security-audit`

### Database — Inconsistent Row Mapping Pattern

- [x] **#6 — `get_team_users` uses `.map()` instead of `filter_map` + `warn!()`**
  - File: `src/db.rs` — `get_team_users` function
  - Resolution: Changed `.map(|row| UsersInTeam { ... })` with infallible `row.get()` to `.filter_map()` with `row.try_get()` tuple matching. Failed rows now log `warn!("Failed to map users-in-team row — skipping")` and are skipped, consistent with all other list queries.
  - Source commands: `db-review`, `practices-audit`

- [x] **#7 — `get_user_teams` has the same `.map()` issue**
  - File: `src/db.rs` — `get_user_teams` function
  - Resolution: Same approach as #6 — changed to `filter_map` with `try_get()` and `warn!()` on failure, matching the project convention.
  - Source commands: `db-review`, `practices-audit`

### Architecture — Defence-in-Depth Notes

- [x] **#49 — RBAC, OpenAPI sync, and dependency health all verified correct**
  - Resolution: `cargo-audit` is now installed and integrated. Ran `cargo update` to resolve 4 of 5 vulnerabilities (aes-gcm, bytes, ring, time) and 6 of 7 warnings (adler, paste, pest/pest_derive/pest_generator/pest_meta yanked versions). Migrated from unmaintained `rustls-pemfile` to `rustls-pki-types` `PemObject` trait (removed direct dependency). Only 1 unfixable advisory remains: `rsa` 0.9.10 via `jsonwebtoken` (RUSTSEC-2023-0071, no upstream fix available — tracked as #55). RBAC and OpenAPI sync remain correct.
  - Source commands: `rbac-rules`, `openapi-sync`, `dependency-check`

### Test Gaps (Earlier Round)

- [x] **#37 — No integration test for closed-order enforcement**
  - File: `tests/api_tests.rs`
  - Resolution: Already resolved — tests `closed_order_rejects_add_item`, `closed_order_rejects_update_item`, `closed_order_rejects_delete_item`, and `reopened_order_allows_item_mutations` were already present in the codebase at the time the finding was recorded. Confirmed present and passing (64/64 API integration tests pass).
  - Source command: `test-gaps`

- [x] **#38 — No integration test for `delete_user_by_email` RBAC fallback**
  - File: `tests/api_tests.rs`
  - Resolution: Added two integration tests: `non_admin_delete_by_email_nonexistent_returns_403` (verifies non-admin gets 403 for nonexistent email, preventing info leakage) and `admin_delete_by_email_nonexistent_returns_404` (verifies admin gets proper 404). Both pass against the test database.
  - Source command: `test-gaps`

- [x] **#39 — No WASM test for `authed_get` token refresh retry**
  - File: `frontend/tests/ui_tests.rs`
  - Resolution: Added `test_authed_get_retries_after_401_with_token_refresh` with a stateful fetch mock that tracks `/api/v1.0/users/` call count — returns 401 on first call, 200 on retry. Mock also handles `POST /auth/refresh` returning new tokens. Test verifies: dashboard renders with refreshed user details, `sessionStorage` contains updated tokens, and the user endpoint was called exactly twice (initial 401 + retry 200). All 22 WASM tests pass in headless Chrome.
  - Source command: `test-gaps`

### Backend — Redundant Token-Type Check

- [x] **#45 — `refresh_token` handler duplicates token-type check already enforced by middleware**
  - File: `src/handlers/users.rs` lines 93–98
  - Resolution: Kept the check as defence-in-depth and added a comment explaining that `refresh_validator` middleware already rejects non-refresh tokens before the handler is reached, so this check is a safety net. All 79 unit tests and 65 API integration tests pass.
  - Source commands: `review`, `security-audit`

### Frontend — Clippy Warning in Test File

- [x] **#46 — Useless `format!` in frontend test `ui_tests.rs`**
  - File: `frontend/tests/ui_tests.rs` line 842
  - Resolution: Replaced `format!(r#"..."#)` with `r#"..."#.to_string()` and un-escaped the double braces (`{{`/`}}`) to single braces (`{`/`}`) since format string escaping is no longer needed.
  - Source command: `review`

### Testing — Flaky DB Test

- [x] **#47 — `cleanup_expired_tokens_removes_old_entries` is flaky under parallel test execution**
  - File: `tests/db_tests.rs` lines 1987–2007
  - Resolution: Changed expiry from 1 hour ago to 1 day ago to avoid timing edge cases. Removed the `assert!(deleted >= 1)` global count assertion that was affected by parallel tests. Now only asserts on the specific JTI being removed after cleanup — the meaningful check. All 86 DB integration tests pass.
  - Source command: `test-gaps`

### Security — Missing CSP Headers for Static Files

- [x] **#48 — No Content-Security-Policy header on static file responses**
  - File: `src/server.rs` lines 293–303
  - Resolution: Wrapped the `actix-files` static file service in a `web::scope("")` with `DefaultHeaders` middleware that sets `Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'`. Policy allows WASM execution, inline scripts (required by Trunk's bootstrap), inline styles, and data URIs for images while restricting everything else to same-origin. Initially deployed without `'unsafe-inline'` in `script-src`, which caused a white-screen bug (fixed in commit `5ae1f07`). All tests pass.
  - Source commands: `security-audit`

### Security — Credentials Logged via `#[instrument]`

- [x] **#50 — `#[instrument]` on auth handlers doesn't skip credential parameters**
  - File: `src/handlers/users.rs` lines 65, 85, 129
  - Resolution: Updated all three `#[instrument]` annotations to skip credential parameters:
    - `auth_user`: `#[instrument(skip(basic, state), level = "debug")]`
    - `refresh_token`: `#[instrument(skip(credentials, state), level = "debug")]`
    - `revoke_user_token`: `#[instrument(skip(state, json), level = "debug")]`
  - Source commands: `security-audit`, `review`

### Documentation — CLAUDE.md `handlers/mod.rs` Description Incomplete

- [x] **#51 — `handlers/mod.rs` description in CLAUDE.md omits newer RBAC helpers**
  - File: `CLAUDE.md` line 62
  - Resolution: Updated the Project Structure description to list all current RBAC helpers: `require_admin, require_admin_or_team_admin, require_team_admin, require_team_member, require_self_or_admin, require_self_or_admin_or_team_admin, requesting_user_id`.
  - Source command: `practices-audit`

### Database — Missing DROP TABLE for token_blacklist

- [x] **#52 — `database.sql` missing `DROP TABLE IF EXISTS token_blacklist`**
  - File: `database.sql` line 16
  - Resolution: Added `DROP TABLE IF EXISTS token_blacklist;` after the existing `DROP TABLE IF EXISTS items;` block, before `CREATE EXTENSION`. Verified by `make test-integration` which re-runs the schema script on a fresh database (output shows `NOTICE: table "token_blacklist" does not exist, skipping`).
  - Source command: `db-review`

### Code Quality — Unused `require_self_or_admin` Helper

- [x] **#53 — `require_self_or_admin` helper is retained but never called**
  - File: `src/handlers/mod.rs` line 92
  - Resolution: Added `#[deprecated(note = "Use require_self_or_admin_or_team_admin instead")]` attribute to make the intent explicit. The function is kept for potential future use but callers will now see a deprecation warning. CLAUDE.md already documents the legacy status.
  - Source command: `review`

### Dependencies — `tokio-pg-mapper` Is Archived

- [x] **#60 — `tokio-pg-mapper` crate is unmaintained/archived**
  - File: `Cargo.toml` line 26, `src/models.rs`, `src/db.rs`, `src/errors.rs`
  - Resolution: Removed `tokio-pg-mapper` dependency entirely. Created a custom `FromRow` trait and `FromRowError` enum in `src/from_row.rs` with manual implementations for all 15 model structs that previously used `#[derive(PostgresMapper)]`. Updated `src/db.rs` to import `crate::from_row::FromRow` instead of `tokio_pg_mapper::FromTokioPostgresRow`. Updated `src/errors.rs` to use `crate::from_row::FromRowError` in the `DbMapper` variant and match arms. Removed all `#[pg_mapper(table = "...")]` attributes and `PostgresMapper` derive from `src/models.rs`. Updated `CLAUDE.md` tech stack description. All 79 unit tests, 65 API integration tests, and 86 DB integration tests pass.
  - Source command: `dependency-check`

### Deployment — Docker Image Tags Verified Valid (False Positives)

- [x] **#62 — `postgres:18.3` Docker image tag does not exist (FALSE POSITIVE)**
  - Files: `docker-compose.yml` line 24 (`postgres-setup` image), `Dockerfile.postgres` line 2 (`FROM postgres:18.3`)
  - Resolution: Verified via Docker Hub API that `postgres:18.3` exists and is actively pulled (last pushed 2026-02-26). PostgreSQL 18 has been released. The assessment was based on stale training data. No changes needed.
  - Source commands: `dependency-check`, `review`

- [x] **#63 — `rust:1.93.1` Docker image tag may not exist (FALSE POSITIVE)**
  - File: `Dockerfile.breakfast` lines 10, 17, 23, 52 (all four `FROM rust:1.93.1` stages)
  - Resolution: Verified via Docker Hub API that `rust:1.93.1` exists and is actively pulled (last pushed 2026-02-25). The local `rustc --version` also reports `1.93.1`. The assessment was based on stale training data. No changes needed.
  - Source commands: `dependency-check`, `review`

### Code Quality — Monolithic `src/db.rs` Refactored

- [x] **#64 — `src/db.rs` is 1,144+ lines covering all domain areas**
  - File: `src/db.rs` (entire file — 20+ public functions across users, teams, roles, items, orders, memberships, auth, and token cleanup)
  - Resolution: Split the monolithic `src/db.rs` into a `src/db/` module directory with 9 domain-specific files: `db/mod.rs` (re-exports all public functions), `db/health.rs` (check_db), `db/users.rs` (7 user functions), `db/teams.rs` (7 team functions), `db/roles.rs` (5 role functions), `db/items.rs` (5 item functions), `db/orders.rs` (6 team order functions), `db/order_items.rs` (6 order item functions), `db/membership.rs` (7 membership/RBAC functions), `db/tokens.rs` (3 token blacklist functions). All call sites continue using `db::function_name` via re-exports in `mod.rs`. All 79 unit tests, 65 API integration tests, and 86 DB integration tests pass.
  - Source commands: `review`, `practices-audit`

### Dependencies — `flurry` Replaced with `dashmap`

- [x] **#65 — `flurry` 0.5.2 is unmaintained (last release 2023)**
  - Files: `Cargo.toml`, `src/models.rs`, `src/server.rs`, `src/middleware/auth.rs`, `src/handlers/users.rs`, `src/routes.rs`, `tests/api_tests.rs`
  - Resolution: Replaced `flurry` 0.5.2 with `dashmap` 6.1.0 (actively maintained). Updated `Cargo.toml` dependency, changed `State` struct fields from `flurry::HashMap` to `dashmap::DashMap`, removed all `.pin()` calls throughout the codebase (DashMap uses direct method calls without pinning), adjusted `get()` usage for DashMap's `Ref` return type, updated all test helpers. The `basic_validator` cache eviction logic was refactored to drop the `Ref` guard before calling `remove()` to avoid deadlocks. All 79 unit tests, 65 API integration tests, and 86 DB integration tests pass.
  - Source commands: `dependency-check`, `review`

### Security — HTTPS Redirect Implemented

- [x] **#72 — HTTP requests are not redirected to HTTPS**
  - File: `src/server.rs` (lines 26, 63–126)
  - Resolution: Added `redirect_to_https` handler and `spawn_http_redirect_server` function that binds an HTTP listener on port 80 and returns `301 Moved Permanently` redirecting to the HTTPS equivalent URL. The redirect preserves path, query string, and handles both standard (443) and non-standard HTTPS ports. If port 80 binding fails (e.g., insufficient privileges), a warning is logged and the main HTTPS server continues unaffected. Added 4 unit tests: `redirect_handler_returns_301_with_location`, `redirect_handler_omits_port_for_443`, `redirect_handler_preserves_root_path`, `redirect_handler_uses_localhost_when_no_host_header`. All 136 backend tests pass.
  - Source commands: `security-audit`

### Testing — Missing Test Coverage Areas Addressed

- [x] **#74 — Several areas lack dedicated test coverage**
  - Files: `src/from_row.rs` (10 unit tests added), `src/middleware/openapi.rs` (14 tests added), `src/bin/healthcheck.rs` (22 tests added), `src/server.rs` (6 CORS tests added), `frontend/tests/ui_tests.rs` (double-failure test added)
  - Resolution: All five sub-items resolved:
    1. `from_row.rs` — 10 unit tests covering `FromRowError` display/debug, `map_err` conversion, trait object safety, and type implementation verification.
    2. `middleware/openapi.rs` — 14 tests validating OpenAPI spec version, all endpoint paths (health, auth, users, teams, team orders, order items, items, roles), operation count (41), all 27 registered schemas, both security schemes (bearer + basic), and JSON round-trip.
    3. `bin/healthcheck.rs` — 22 tests covering `is_healthy_response` (HTTP 1.0/1.1 200 OK, 503, 500, 404, empty, garbage, partial, non-200 2xx), `status_line` extraction, `NoVerifier` (cert acceptance, supported signature schemes — RSA, ECDSA, EdDSA — count and membership, debug format), and TLS config/connection construction.
    4. CORS configuration — 6 tests in `server.rs`: `cors_allows_same_origin`, `cors_rejects_disallowed_origin`, `cors_allows_configured_methods`, `cors_rejects_disallowed_method`, `cors_allows_configured_headers`, `cors_max_age_is_3600`.
    5. Frontend double-failure — `test_authed_get_double_failure_falls_back_to_login` test added in `frontend/tests/ui_tests.rs`.
  - Source commands: `test-gaps`

## Notes

- All 136 backend unit tests pass (114 lib + 22 healthcheck); 65 API integration tests pass; 86 DB integration tests pass; 23 WASM tests pass.
- Clippy is clean on both backend and frontend.
- `cargo-audit` reports 1 unfixable vulnerability (`rsa` 0.9.10 via `jsonwebtoken`, RUSTSEC-2023-0071) and 0 warnings. All other advisories resolved via `cargo update` and the `rustls-pemfile` → `rustls-pki-types` migration.
- `actix-files` CVE (GHSA-8v2v-wjwg-vx6r, GHSA-gcqf-3g44-vc9p) verified patched — `Cargo.lock` resolves to 0.6.10 (fixed version).
- No direct dependency CVEs found via CVE validation of all 24 direct dependencies.
- CSP white-screen bug fixed in commit `5ae1f07`: added `'unsafe-inline'` to `script-src` so Trunk's inline WASM bootstrap script is not blocked by Chrome.
- All 34 completed items (#1, #6, #7, #15, #16, #37, #38, #39, #40, #41, #42, #43, #44, #45, #46, #47, #48, #49, #50, #51, #52, #53, #56, #57, #58, #60, #62, #63, #64, #65, #66, #67, #72, #74, #78) confirmed in place. No regressions.
- Open items summary: 0 critical, 0 important, 7 minor (#59, #68, #69, #70, #71, #73, #75, #77), 4 informational (#54, #55, #61, #76).
- All 10 assessment commands verified: `api-completeness`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment`.
- RBAC enforcement is correct across all handlers per the policy table in `rbac-rules.md`.
- OpenAPI spec (`middleware/openapi.rs`) is fully synchronized with `routes.rs` — all 41 handler paths present, all request/response schemas registered.
- CLAUDE.md conventions are followed consistently: error handling pattern, `#[instrument]` annotations, validation before DB calls, logging severity levels.
- CSP header documented in CLAUDE.md Key Conventions (completed item #57) and enforced in `server.rs` (completed item #48).
- Test counts verified on 2026-03-01: 136 backend unit (config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, healthcheck: 22), 65 API integration, 86 DB integration, 23 WASM.
- `flurry` replaced with `dashmap` 6.1.0 (#65) — all `.pin()` patterns removed, DashMap uses direct method calls.
- `src/db.rs` split into `src/db/` module with 9 domain files (#64) — all call sites unchanged via `pub use` re-exports in `mod.rs`.
- Command files (`db-review.md`, `api-completeness.md`, `test-gaps.md`) updated to reference `src/db/` module directory (#78).
- `refinery` 0.8 adopted for database migrations (#66). Initial migration `V1__initial_schema.sql` created. Migrations run at server startup. `database.sql` retained for dev/test with warning header.
- In-memory token blacklist now stores `DateTime<Utc>` expiry times (#67). `spawn_token_cleanup_task` evicts expired entries via `DashMap::retain()` every hour.
