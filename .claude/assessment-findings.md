# Assessment Findings

Last assessed: 2025-07-20

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will move completed items to `.claude/resolved-findings.md`, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

### Dependencies — `jsonwebtoken` Pulls Vulnerable and Unnecessary Crypto Crates

- [ ] **#132 — `rust_crypto` feature enables ~15 unused crates including vulnerable `rsa` (RUSTSEC-2023-0071); granular `["hmac", "sha2"]` features are available but do not work**
  - File: `Cargo.toml` (jsonwebtoken dependency)
  - Problem: `features = ["rust_crypto"]` pulls `rsa`, `ed25519-dalek`, `p256`, `p384`, `rand` — none of which are used (only HS256). The `rsa` crate has an unfixable timing side-channel advisory.
  - Attempted fix: Changed `features = ["rust_crypto"]` to `features = ["hmac", "sha2"]`. This compiled but all JWT tests failed at runtime: jsonwebtoken 10.x requires either `rust_crypto` or `aws_lc_rs` to auto-install a `CryptoProvider`. The granular `hmac`/`sha2` features do not register a provider, causing `"Could not automatically determine the process-level CryptoProvider"` errors. Manual `CryptoProvider::install_default()` calls would be needed, which is invasive.
  - Status: **Blocked on upstream.** Requires `jsonwebtoken` to either support granular features with auto-provider registration, or to split the `rust_crypto` feature so HS-only usage doesn't pull RSA/EC crates. Reverted to `features = ["rust_crypto"]`.
  - Mitigation: `cargo audit --ignore RUSTSEC-2023-0071` is used in the Makefile, CI, and assessment commands to acknowledge the advisory while keeping audit runs clean for other vulnerabilities. **This ignore must be re-evaluated periodically** — check whether a new `rsa` release resolves RUSTSEC-2023-0071 or whether `jsonwebtoken` adds HS-only feature support.
  - Source commands: `dependency-check`

## Important Items

### Security — `create_team_order` Accepts Arbitrary User ID (Attribution Spoofing)

- [ ] **#266 — `create_team_order` does not validate that `teamorders_user_id` matches the requesting user**
  - Files: `src/handlers/teams.rs` (`create_team_order` handler), `src/models.rs` (`CreateTeamOrderEntry`)
  - Problem: The handler calls `require_team_member` to verify the requester is in the team, then passes `json.into_inner()` directly to `db::create_team_order`. Any team member can create orders attributed to a different user by sending a different `teamorders_user_id`.
  - Fix: Either validate `json.teamorders_user_id == requesting_user_id()` (reject mismatches with 403), or remove `teamorders_user_id` from the request body and set it server-side from the JWT claims. Admin bypass could be allowed for creating orders on behalf of others.
  - Source commands: `api-completeness`, `security-audit`

### Security — JWT Tokens Lack `iss` and `aud` Claims

- [ ] **#267 — No audience or issuer validation on JWT tokens**
  - Files: `src/models.rs` (`Claims` struct), `src/middleware/auth.rs` (`generate_token_pair`, `verify_jwt`)
  - Problem: The `Claims` struct has `sub`, `exp`, `iat`, `jti`, `token_type` but no `iss` (issuer) or `aud` (audience). `verify_jwt` creates `Validation::new(Algorithm::HS256)` without enforcing issuer or audience. If the same `jwtsecret` is reused across services or leaked, tokens minted elsewhere would be accepted.
  - Fix: Add `iss: String` and `aud: String` to `Claims`. Set them during token generation (e.g., `iss = "omp-breakfast"`, `aud = "omp-breakfast"`). Configure `Validation` to require matching values.
  - Source commands: `security-audit`

### Security — RBAC Inconsistency on Team Order Mutations

- [ ] **#268 — Any team member (including Guest) can update/delete any team order in their team**
  - File: `src/handlers/teams.rs` (`update_team_order`, `delete_team_order`)
  - Problem: `delete_team_order` (single) and `update_team_order` call `require_team_member` — any team member including a Guest can delete or close/reopen any individual order. But `delete_team_orders` (bulk) calls `require_team_admin`. A Guest member can set `closed = true/false` on any order or delete orders they don't own.
  - Fix: Either elevate single-order mutations to `require_team_admin`, or add an owner-check (`order.teamorders_user_id == requesting_user_id()`) with admin/team-admin bypass.
  - Source commands: `security-audit`, `rbac-rules`

### Documentation — `guard_admin_role_assignment` Undocumented in RBAC Policy

- [ ] **#269 — `guard_admin_role_assignment` helper is missing from CLAUDE.md RBAC conventions and rbac-rules.md policy table**
  - Files: `CLAUDE.md` (Key Conventions — RBAC sections), `.claude/commands/rbac-rules.md`
  - Problem: The helper was extracted in resolved finding #216 but never added to the RBAC documentation. The policy table in rbac-rules.md does not have a row for "assigns Admin role → Admin only" guard.
  - Fix: Add a row to the RBAC policy table: "Team Members | Add/Update Role (assigns Admin role) | Admin only | guard_admin_role_assignment". Add the function to CLAUDE.md's Team RBAC and Global Admin RBAC paragraphs.
  - Source commands: `cross-ref-check`, `practices-audit`

## Minor Items

### Code Quality — Dead S3 Config Fields

- [ ] **#59 — `s3_key_id` and `s3_key_secret` are loaded and stored but never used**
  - Files: `src/models.rs` (`State` struct), `src/config.rs` (`ServerConfig` struct), `src/server.rs` (state construction), `config/default.yml`, `config/development.yml`, `config/production.yml`
  - Problem: The `s3_key_id` and `s3_key_secret` fields are defined in `ServerConfig`, loaded from config files, stored in `State`, and propagated through all test helpers, but no handler, middleware, or DB function ever reads them.
  - Fix: Either remove the fields entirely or document the intent in CLAUDE.md's Unfinished Work section.
  - Source commands: `review`, `practices-audit`

### Code Quality — Dead `database.url` Config Field

- [ ] **#68 — `database.url` field in `Settings` is configured but unused**
  - Files: `src/config.rs` (`Database` struct with `#[allow(dead_code)]`), `config/default.yml`, `config/development.yml`
  - Problem: The `Database` struct contains a single `url` field marked `#[allow(dead_code)]`. The DB pool is created from the `pg.*` config fields.
  - Fix: Remove the `Database` struct and its `database` field from `Settings`. Remove `database:` sections from config files.
  - Source commands: `review`, `practices-audit`

### Security — Seed Data Uses Hardcoded Argon2 Salt

- [ ] **#70 — All seed users share the same Argon2 hash with a hardcoded salt**
  - File: `database_seed.sql`
  - Problem: All 5 seed users have identical Argon2id password hashes. While dev-only, this creates risk if accidentally run against production.
  - Fix: Add a prominent `-- WARNING: DO NOT RUN IN PRODUCTION` comment at the top.
  - Source commands: `security-audit`, `db-review`

### Frontend — All Components in Single `app.rs` File

- [ ] **#71 — Frontend `app.rs` is a 600+ line monolith**
  - File: `frontend/src/app.rs`
  - Problem: The entire frontend lives in a single file. As planned pages are built, this will become unmanageable.
  - Fix: Split into module structure when building the next frontend page.
  - Source commands: `review`, `practices-audit`

### Security — No Account Lockout After Failed Auth Attempts

- [ ] **#73 — Failed authentication is rate-limited but no lockout policy exists**
  - Files: `src/routes.rs`, `src/handlers/users.rs`
  - Problem: The `/auth` endpoint has rate limiting but no account-level lockout after N consecutive failures.
  - Fix: Track failed login attempts per email. Lock after threshold (e.g., 5 failures in 15 minutes).
  - Source commands: `security-audit`

### Deployment — Production Config Has Placeholder Hostname

- [ ] **#75 — `config/production.yml` uses `pick.a.proper.hostname` as the PG host**
  - File: `config/production.yml`
  - Problem: Placeholder string with no startup validation catch.
  - Fix: Add a startup check similar to the secret-validation panic.
  - Source commands: `practices-audit`, `review`

### Security — Swagger UI Exposed in Production

- [ ] **#112 — `/explorer` registered unconditionally regardless of environment**
  - File: `src/routes.rs`
  - Problem: In production, this exposes the complete API schema, aiding attacker reconnaissance.
  - Fix: Conditionally register the Swagger UI scope only when `ENV != production`, or gate behind admin auth.
  - Source commands: `security-audit`

### Documentation — Frontend Test Category Breakdown Sums to 21, Not 23

- [ ] **#163 — CLAUDE.md test category breakdown omits 2 token refresh tests**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Problem: 8 categories total 4+3+3+3+2+1+2+3 = 21, but 23 WASM tests exist.
  - Fix: Add "Token refresh (2 tests)" category to the breakdown.
  - Source commands: `cross-ref-check`

### Frontend — Login Shows "Invalid Credentials" for All Non-2xx Errors

- [ ] **#225 — HTTP 500, 429, and 503 responses all display "Invalid username or password"**
  - File: `frontend/src/app.rs`
  - Problem: The login flow's `Ok(_)` catch-all always shows a credentials error.
  - Fix: Match on `response.status()` and provide differentiated messages.
  - Source commands: `api-completeness`, `review`

### Database — `closed` Column Read as `Option<bool>` Despite `NOT NULL` Constraint

- [ ] **#235 — `is_team_order_closed` and `guard_open_order` use `Option<bool>` for a NOT NULL column**
  - File: `src/db/order_items.rs`
  - Problem: The column is `boolean NOT NULL DEFAULT FALSE`, so the `Option<bool>` and `.unwrap_or(false)` are unnecessary.
  - Fix: Change to `row.get::<_, bool>("closed")`.
  - Source commands: `db-review`

### Testing — No API Test for GET Single Team Order by ID

- [ ] **#237 — `GET /api/v1.0/teams/{team_id}/orders/{order_id}` never called in tests**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs`
  - Source commands: `test-gaps`

### Testing — `add_team_member` with FK-Violating IDs Untested

- [ ] **#238 — Adding a member with non-existent `user_id` or `role_id` → error quality untested**
  - Files: `tests/api_tests.rs`, `tests/db_tests.rs`
  - Source commands: `test-gaps`

### Testing — No Frontend Test for Non-401/Non-Network HTTP Errors

- [ ] **#239 — No WASM test mocks 500 or 429 responses for the login flow**
  - File: `frontend/tests/ui_tests.rs`
  - Source commands: `test-gaps`

### Auth — `revoke_user_token` Returns 403 for Missing Authentication

- [ ] **#243 — `revoke_user_token` uses `Error::Forbidden("Authentication required")` — should be `Error::Unauthorized`**
  - File: `src/handlers/users.rs`
  - Fix: Change to `Error::Unauthorized("Authentication required".to_string())`.
  - Source commands: `practices-audit`

### OpenAPI — `get_health` Missing 503 Response Annotation

- [ ] **#244 — `get_health` utoipa annotation only documents 200; handler also returns 503**
  - File: `src/handlers/mod.rs`
  - Fix: Add `(status = 503, description = "Service unavailable — database unreachable", body = StatusResponse)`.
  - Source commands: `openapi-sync`

### OpenAPI — `create_user` Annotates Unreachable 404

- [ ] **#245 — `create_user` utoipa includes `(status = 404)` but handler never returns 404**
  - File: `src/handlers/users.rs`
  - Fix: Remove the `(status = 404, ...)` line from utoipa responses.
  - Source commands: `openapi-sync`

### Documentation — CLAUDE.md Test Counts Stale

- [ ] **#246 — CLAUDE.md says "162 unit tests" and "90 API integration tests" — actual counts are 184 (162 lib + 22 healthcheck) and 86 respectively**
  - File: `CLAUDE.md` (Testing → Backend section)
  - Problem: Unit test count text says "162" but includes "and the healthcheck binary" without adding the 22 healthcheck tests. API test count says 90 but actual is 86.
  - Fix: Update to "184 unit tests (162 library + 22 healthcheck binary)" and "86 API integration tests". Update total from 387 to 383.
  - Source commands: `cross-ref-check`, `test-gaps`

### Validation — `Validate` Derive Still on 4 No-Rule Structs

- [ ] **#253 — `Validate` derive is still present on `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, `UpdateMemberRoleEntry`**
  - File: `src/models.rs`
  - Problem: #224 resolution removed `validate()` calls but left the derive macros — generating dead code.
  - Fix: Remove `Validate` from the derive macros.
  - Source commands: `practices-audit`, `review`

### Database — COALESCE Prevents Clearing `duedate` to NULL

- [ ] **#270 — `update_team_order` uses `COALESCE($2, duedate)` which makes NULL preserve the old value — no way to clear the field**
  - File: `src/db/orders.rs` (UPDATE query)
  - Problem: After #217 applied COALESCE to all three fields for partial-update semantics, sending `null` for `duedate` preserves the existing value instead of clearing it. There's no way for an API consumer to remove a previously-set due date.
  - Fix: Use a sentinel value pattern, add a separate "clear duedate" boolean flag, or use a JSON merge-patch approach.
  - Source commands: `api-completeness`, `db-review`

### OpenAPI — `create_team_order` Missing 409 Annotation

- [ ] **#271 — `create_team_order` utoipa does not document 409 conflict response**
  - File: `src/handlers/teams.rs` (utoipa annotation)
  - Fix: Add `(status = 409, description = "Conflict", body = ErrorResponse)`.
  - Source commands: `api-completeness`

### Documentation — CLAUDE.md Missing `guard_admin_role_assignment` in Function List

- [ ] **#272 — handlers/mod.rs description in CLAUDE.md Project Structure omits `guard_admin_role_assignment`**
  - File: `CLAUDE.md` (Project Structure → `handlers/mod.rs` description)
  - Fix: Add `guard_admin_role_assignment` to the function list.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md API Test Count Wrong

- [ ] **#273 — CLAUDE.md says "90 API integration tests" but actual count is 86**
  - File: `CLAUDE.md` (Testing → Backend section)
  - Fix: Update to "86 API integration tests".
  - Source commands: `cross-ref-check`, `test-gaps`

### Database — `orders.amt` CHECK Allows 0 but API Requires ≥1

- [ ] **#274 — DB constraint `CHECK (amt >= 0)` permits zero-quantity orders; API validation requires `min = 1`**
  - Files: `migrations/V1__initial_schema.sql` (orders table), `src/models.rs`
  - Fix: New migration: `ALTER TABLE orders DROP CONSTRAINT orders_amt_check, ADD CONSTRAINT orders_amt_check CHECK (amt >= 1)`.
  - Source commands: `db-review`

### Performance — Missing Composite Index for Team Orders Query

- [ ] **#275 — `get_team_orders` queries `WHERE teamorders_team_id = $1 ORDER BY created DESC` without a covering index**
  - File: `src/db/orders.rs`
  - Fix: New migration: `CREATE INDEX idx_teamorders_team_created ON teamorders (teamorders_team_id, created DESC)` and drop redundant `idx_teamorders_team`.
  - Source commands: `db-review`

### OpenAPI — `revoke_user_token` Missing 401 Response Annotation

- [ ] **#276 — utoipa annotation doesn't document 401 response for invalid/missing auth**
  - File: `src/handlers/users.rs`
  - Fix: Add `(status = 401, description = "Unauthorized", body = ErrorResponse)`.
  - Source commands: `openapi-sync`

### OpenAPI — `add_team_member` Missing 404 for Invalid Role ID

- [ ] **#277 — utoipa annotation doesn't document 404 when `role_id` doesn't exist**
  - File: `src/handlers/teams.rs`
  - Fix: Add `(status = 404, description = "Role not found", body = ErrorResponse)`.
  - Source commands: `openapi-sync`

### Security — HTTP→HTTPS Redirect Open Redirect via Host Header

- [ ] **#278 — `redirect_to_https` uses unvalidated `Host` header for redirect Location**
  - File: `src/server.rs`
  - Fix: Validate the Host against a configured allowlist or use config value.
  - Source commands: `security-audit`

### Frontend — Logout Revocation Fails With Expired Access Token

- [ ] **#279 — `on_logout` uses potentially-expired access token for revocation requests**
  - File: `frontend/src/app.rs`
  - Problem: If the 15-minute access token has expired, revocation requests fail with 401 (fire-and-forget), leaving the 7-day refresh token valid.
  - Fix: Use `authed_request()` for revocation or refresh the token before revoking.
  - Source commands: `security-audit`

### Config — `server.secret` Production-Checked but Never Used

- [ ] **#280 — `ServerConfig.secret` field is loaded and panic-checked in production but has zero runtime effect**
  - Files: `src/config.rs`, `src/server.rs`
  - Problem: After #189 removed `State.secret`, the field only exists in config. The production check creates a false security signal.
  - Fix: Remove the field and its production check, or document its purpose as a canary.
  - Source commands: `security-audit`

### Security — `update_user` Cache Invalidation Targets Wrong Key

- [ ] **#281 — When email changes, the handler invalidates the NEW email cache key, not the OLD one**
  - File: `src/handlers/users.rs`
  - Problem: Old email's cache entry persists until TTL (5 min), allowing auth with the old email.
  - Fix: Fetch the user's current email before the update, then invalidate both old and new keys.
  - Source commands: `review`

### Code Quality — `update_user` Has Inconsistent RBAC/Validate Ordering

- [ ] **#282 — `update_user` is the only mutation handler that does RBAC before validate (9 others do validate first)**
  - File: `src/handlers/users.rs`
  - Fix: Standardize ordering across all mutation handlers.
  - Source commands: `review`

### Code Quality — `delete_user` Premature Cache Invalidation

- [ ] **#283 — Handler invalidates auth cache before the DB delete succeeds**
  - File: `src/handlers/users.rs`
  - Fix: Move invalidation after the successful delete, matching `delete_user_by_email`.
  - Source commands: `review`

### Performance — `refresh_validator` Redundantly Re-decodes JWT

- [ ] **#284 — Middleware decodes JWT but doesn't pass claims to handler, which re-decodes**
  - Files: `src/middleware/auth.rs`, `src/handlers/users.rs`
  - Fix: Insert claims into `req.extensions_mut()` like `jwt_validator` does.
  - Source commands: `review`

### Code Quality — `cargo fmt` Drift in `db_tests.rs`

- [ ] **#297 — `cargo fmt --check` reports formatting diff in `db_tests.rs`**
  - File: `tests/db_tests.rs` (line ~2433)
  - Problem: A multi-line `assert!()` should be single-line per rustfmt rules. One-liner fix: `cargo fmt`.
  - Source commands: `practices-audit`

### Security — `revoke_user_token` Returns HTTP 500 for Expired/Malformed Tokens

- [ ] **#298 — `verify_jwt` in `revoke_user_token` propagates `Error::Jwt` → 500 for expired tokens submitted for revocation**
  - File: `src/handlers/users.rs` (line ~138)
  - Problem: When a legitimately-expired (but validly-signed) token is submitted for revocation, `verify_jwt` returns `Error::Jwt` which maps to HTTP 500 "Internal server error". The user gets no actionable information.
  - Fix: Either catch `Error::Jwt` in the handler and return `HttpResponse::BadRequest().json(ErrorResponse { error: "Token is invalid or expired" })`, or use a `Validation` with `validate_exp = false` for the revocation-specific verify call (revoking an expired token is harmless, and signature verification is still performed).
  - Source commands: `security-audit`

## Informational Items

### Documentation — Test Count Maintenance Burden

- [ ] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md`
  - Source command: `practices-audit`
  - Action: Inherent maintenance cost. The assessment process updates counts each time it runs.

### API Design — No Pagination on List Endpoints

- [ ] **#61 — List endpoints return all records without pagination**
  - Files: `src/db/`, `src/handlers/`
  - Source commands: `review`, `api-completeness`
  - Action: Add `LIMIT`/`OFFSET` when data growth warrants it.

### Deployment — No `.env.example` File for Onboarding

- [ ] **#76 — No `.env.example` or env documentation for new developers**
  - Source commands: `practices-audit`

### Frontend — Consumes Only 4 of 41 Endpoints

- [ ] **#116 — Frontend only uses auth (3) + user-detail (1) endpoints**
  - File: `frontend/src/app.rs`
  - Source commands: `api-completeness`
  - Action: Documented in CLAUDE.md Frontend Roadmap.

### Deployment — Dev Config in Production Docker Image

- [ ] **#118 — `development.yml` copied into production image unnecessarily**
  - File: `Dockerfile.breakfast`
  - Source commands: `security-audit`

### Security — Rate Limiter Uses IP-Based Key Extraction

- [ ] **#119 — Behind a reverse proxy, all requests share one IP**
  - File: `src/routes.rs`
  - Source commands: `security-audit`

### Security — Auth Cache Staleness Window

- [ ] **#120 — 5-minute cache TTL allows stale credentials after password change**
  - File: `src/middleware/auth.rs`
  - Source commands: `security-audit`

### Dependencies — `native-tls` Compiled Alongside `rustls`

- [ ] **#121 — `refinery` unconditionally enables `postgres-native-tls`**
  - Source commands: `dependency-check`

### Dependencies — Low-Activity `tracing-bunyan-formatter`

- [ ] **#123 — `tracing-bunyan-formatter` has infrequent releases**
  - Source commands: `dependency-check`

### Testing — Additional Coverage Gaps

- [ ] **#124 — FK cascade and `fix_migration_history` DB interaction lack tests**
  - Source commands: `test-gaps`

### Frontend — `Page::Dashboard` Clones Data on Every Signal Read

- [ ] **#126 — Dashboard state stored in enum variant, cloned on every re-render**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Missing `aria-busy` on Submit Button

- [ ] **#127 — No `aria-busy` attribute during login form submission**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Decorative Icons Lack Accessibility Attributes

- [ ] **#128 — Warning icon and checkmark lack `aria-hidden="true"`**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Inconsistent `spawn_local` Import

- [ ] **#210 — Session restore uses `wasm_bindgen_futures::spawn_local` while logout uses `leptos::task::spawn_local`**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Form Has Redundant Double Validation

- [ ] **#211 — `<form>` has both native HTML5 validation and custom JavaScript validation**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Loading Page Spinner Not Announced to Screen Readers

- [ ] **#231 — Loading spinner container lacks `role="status"` and `aria-live`**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Code Quality — `ErrorResponse::Display` Fallback Doesn't Escape JSON

- [ ] **#232 — If `serde_json::to_string` fails, the fallback `format!` produces invalid JSON**
  - File: `src/errors.rs`
  - Source commands: `review`

### Frontend — Redundant `session_storage()` Calls in Logout Handler

- [ ] **#233 — `session_storage()` called 3 times in the `on_logout` closure**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Code Quality — `from_row.rs` Error Classification Uses Fragile String Matching

- [ ] **#234 — `map_err` helper checks for `"column"` or `"not found"` in error messages**
  - File: `src/from_row.rs`
  - Source commands: `review`

### Documentation — Command Files Reference Stale Migration Range

- [ ] **#250 — `api-completeness.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/api-completeness.md`
  - Source commands: `cross-ref-check`

- [ ] **#251 — `db-review.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/db-review.md`
  - Source commands: `cross-ref-check`

### Documentation — `database.sql` Stale vs V3–V5

- [ ] **#252 — `database.sql` deprecated script doesn't reflect V3–V5 changes**
  - File: `database.sql`
  - Source commands: `cross-ref-check`

### Code Quality — `from_row_ref` Boilerplate Reducible by Macro

- [ ] **#254 — 9 `FromRow` implementations total ~200 lines of repetitive `try_get`/`map_err` per column**
  - File: `src/from_row.rs`
  - Source commands: `review`

### Code Quality — Duplicated Row-Mapping Pattern Across 6 DB List Functions

- [ ] **#255 — Identical `filter_map` + `warn` block in 6 list functions**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Source commands: `review`

### Deployment — `HTTP_REDIRECT_PORT` Hardcoded to 80

- [ ] **#256 — HTTP→HTTPS redirect listener binds to port 80 unconditionally**
  - File: `src/server.rs`
  - Source commands: `review`

### Dependencies — `password-hash` Direct Dependency for Feature Activation Only

- [ ] **#257 — `password-hash` is a direct dependency only to enable `getrandom` feature**
  - File: `Cargo.toml`
  - Source commands: `dependency-check`

### Security — Missing `Permissions-Policy` Header

- [ ] **#258 — `DefaultHeaders` does not include `Permissions-Policy`**
  - File: `src/server.rs`
  - Source commands: `security-audit`

### Deployment — Docker Compose `breakfast` Service Lacks Resource Limits

- [ ] **#259 — No `deploy.resources.limits` for CPU or memory**
  - File: `docker-compose.yml`
  - Source commands: `security-audit`

### Documentation — `database_seed.sql` Header Only Mentions V1

- [ ] **#260 — Seed data file header references only V1 schema**
  - File: `database_seed.sql`
  - Source commands: `cross-ref-check`

### Testing — No Test for Partial `update_team_order` (COALESCE Preservation)

- [ ] **#261 — No test passes `None` for some update fields and verifies existing values are preserved**
  - File: `tests/db_tests.rs`
  - Source commands: `test-gaps`

### Testing — No Test for `create_team_order` with FK-Violating `team_id`

- [ ] **#262 — No test creates a team order with non-existent `team_id` to verify FK error handling**
  - Files: `tests/db_tests.rs`, `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — No Explicit Refresh Token Revocation → Refresh Rejection Test

- [ ] **#263 — No test explicitly revokes a refresh token then verifies `/auth/refresh` returns 401**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — No Test for Empty Order Items List Response

- [ ] **#264 — No test verifies `GET .../items` returns `200 []` for an order with zero items**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `guard_admin_role_assignment` Non-Existent `role_id` Path Untested

- [ ] **#265 — No test calls `add_team_member` or `update_member_role` with a non-existent `role_id`**
  - File: `src/handlers/mod.rs`
  - Source commands: `test-gaps`

### Security — Token Responses Lack `Cache-Control: no-store`

- [ ] **#247 — `/auth` and `/auth/refresh` responses contain JWT tokens but no `Cache-Control` header**
  - Files: `src/server.rs`, `src/handlers/users.rs`
  - Source commands: `security-audit`

### Security — Missing `Referrer-Policy` Header

- [ ] **#248 — `DefaultHeaders` does not include `Referrer-Policy`**
  - File: `src/server.rs`
  - Source commands: `security-audit`

### Deployment — Docker Compose Exposes PostgreSQL on All Interfaces

- [ ] **#249 — `docker-compose.yml` maps port 5432 to `0.0.0.0` by default**
  - File: `docker-compose.yml`
  - Source commands: `security-audit`

### Database — Text Columns Lack DB-Level Length Constraints

- [ ] **#285 — Text columns have API-level max-length validation but no `VARCHAR(N)` or `CHECK` at the database layer**
  - Files: `migrations/V1__initial_schema.sql`
  - Source commands: `db-review`
  - Action: Informational — API is the sole entry point.

### Error Handling — `create_order_item` Doesn't Map Trigger Exception Clearly

- [ ] **#286 — PostgreSQL `P0001` (raise_exception from trigger) maps to generic DB error (500)**
  - File: `src/db/order_items.rs`
  - Source commands: `db-review`
  - Action: Informational — the handler already checks before the INSERT; trigger only fires under race conditions.

### OpenAPI — `auth_user` 401 Response Missing Body Type

- [ ] **#287 — `auth_user` utoipa has `(status = 401)` but no `body = ErrorResponse`**
  - File: `src/handlers/users.rs`
  - Source commands: `openapi-sync`

### Dead Code — `is_team_order_closed` Never Called From Handlers

- [ ] **#288 — `is_team_order_closed` is public API but only used in integration tests**
  - File: `src/db/order_items.rs`
  - Source commands: `review`
  - Action: Mark `pub(crate)` or `#[cfg(test)]`.

### Testing — Member-Cannot-Manage-Members Negative Path Untested

- [ ] **#289 — No test where a user with "Member" role tries to POST/DELETE/PUT on team members**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs`
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Member-Cannot-Bulk-Delete-Orders Negative Path Untested

- [ ] **#290 — `delete_team_orders` requires `require_team_admin` but only admin bypass is tested**
  - File: `tests/api_tests.rs`
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Non-Member Cannot Update/Delete Single Team Order Untested

- [ ] **#291 — `non_member_cannot_create_team_order` tests only POST; PUT and DELETE have no non-member test**
  - File: `tests/api_tests.rs`
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Auth Cache FIFO Eviction at Capacity Not Tested

- [ ] **#292 — No test saturates the cache past 1000 entries to verify eviction fires correctly**
  - File: `src/middleware/auth.rs`
  - Source commands: `test-gaps`

### Testing — In-Memory Token Blacklist Cleanup Not Tested

- [ ] **#293 — `DashMap::retain()` cleanup path has no test**
  - File: `src/server.rs`
  - Source commands: `test-gaps`

### Testing — Location Header Verified on Only 1 of 7 Create Endpoints

- [ ] **#294 — `create_item_returns_location_header` exists but no equivalent for 6 other create endpoints**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — GET Orders for Nonexistent Team Untested

- [ ] **#295 — No test calls `GET /teams/{nonexistent}/orders` to verify 200 empty vs 404**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — Delete-Not-Found API Paths Untested for 5 Entities

- [ ] **#296 — No API test calls DELETE with a nonexistent ID for items, roles, team orders, order items, or members**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — No API Test for Revoking an Expired Token

- [ ] **#299 — No test submits a legitimately-expired (but validly-signed) token for revocation**
  - File: `tests/api_tests.rs`
  - Problem: Would currently return 500 (see #298). After #298 is fixed, should assert 400.
  - Source commands: `test-gaps`

### Testing — No API-Level Test for UPDATE with Nonexistent ID → 404

- [ ] **#300 — DB-level tests exist but no API integration test verifies HTTP 404 for PUT with nonexistent UUID across 6 update endpoints**
  - File: `tests/api_tests.rs`
  - Problem: Missing tests for: `PUT /users/{nonexistent}`, `PUT /teams/{nonexistent}`, `PUT /roles/{nonexistent}`, `PUT /items/{nonexistent}`, `PUT /teams/{tid}/orders/{nonexistent}`, `PUT /teams/{tid}/orders/{oid}/items/{nonexistent}`.
  - Source commands: `test-gaps`

### API Design — `get_user_teams` Query Does Not Return `team_id`

- [ ] **#301 — `UserInTeams` model and query lack `team_id`, preventing frontend navigation from team list to team detail**
  - Files: `src/db/teams.rs` (line ~15–25), `src/models.rs` (`UserInTeams` struct)
  - Problem: The query SELECTs `tname, title, firstname, lastname, joined, role_changed` but not `teams.team_id`. A frontend consumer cannot navigate from a user's team list to a team detail page.
  - Fix: Add `teams.team_id` to the SELECT clause and `team_id: Uuid` to the `UserInTeams` struct.
  - Source commands: `db-review`, `api-completeness`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 383 tests pass: 184 backend unit (162 lib + 22 healthcheck), 86 API integration, 90 DB integration, 23 WASM. Total: **383 tests, 0 failures**.
- Backend unit test breakdown: config: 7, db/migrate: 34, errors: 16, from_row: 10, handlers/mod: 11, middleware/auth: 13, middleware/openapi: 14, models: 12, routes: 19, server: 17, validate: 9, healthcheck: 22 = **184 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` 0.9.10 via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. Re-evaluate periodically.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` has one minor diff in `tests/db_tests.rs` (see #297).
- CONNECT Design System: `git pull` reports "Already up to date" — no migration needed.
- RBAC enforcement is correct across all handlers per the policy table.
- OpenAPI spec is synchronized with routes (41 operations), with annotation inaccuracies tracked (#244, #245, #271, #276, #277, #287).
- All SQL queries use parameterized prepared statements — zero injection risk.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- No regressions detected against 176 resolved findings.
- Open items summary: 1 critical (#132 blocked), 4 important (#266–#269), 36 minor, 48 informational. **Total: 89 open items**.
- 176 resolved items in `.claude/resolved-findings.md`.
- Highest finding number: #301.
