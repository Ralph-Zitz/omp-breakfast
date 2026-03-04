# Assessment Findings

Last assessed: 2025-07-21

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

### RBAC — Order Item Handlers Use Wrong Authorization Guard

- [ ] **#302 — `update_order_item` allows any team member to modify other members' order items (privilege escalation)**
  - File: `src/handlers/orders.rs`
  - Problem: Uses `require_team_member` instead of `require_order_owner_or_team_admin`. Any authenticated team member (including Guest role) can update order items belonging to other members.
  - Fix: Change `require_team_member(&state, &claims, team_id).await?` to `require_order_owner_or_team_admin(&state, &claims, team_id, order_id).await?`.
  - Source commands: `rbac-rules`

- [ ] **#303 — `delete_order_item` allows any team member to delete other members' order items (privilege escalation)**
  - File: `src/handlers/orders.rs`
  - Problem: Same as #302 — uses `require_team_member` instead of `require_order_owner_or_team_admin`.
  - Fix: Same pattern as #302.
  - Source commands: `rbac-rules`

### Code Quality — `cargo fmt` Drift in Backend

- [ ] **#304 — `cargo fmt --check` reports formatting diff in `src/middleware/auth.rs`**
  - File: `src/middleware/auth.rs` (line ~174)
  - Problem: Long line needs wrapping per rustfmt rules.
  - Fix: Run `cargo fmt`.
  - Source commands: `practices-audit`

### Code Quality — `cargo fmt` Drift in Frontend

- [ ] **#305 — `cargo fmt --check` reports significant formatting drift in frontend files (~15KB of diffs)**
  - Files: `frontend/src/components/icons.rs` (SVG path data), and other frontend files
  - Problem: Formatting drift accumulated during frontend modular refactor.
  - Fix: Run `cd frontend && cargo fmt`.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md Missing Frontend Modular Architecture

- [ ] **#306 — CLAUDE.md Project Structure tree still shows only `app.rs`, `lib.rs`, `main.rs` under `frontend/src/`**
  - File: `CLAUDE.md` (Project Structure section)
  - Problem: Frontend was refactored into modular architecture (`api.rs`, `components/` with 7 files, `pages/` with 10 files) but CLAUDE.md still describes old single-file structure.
  - Fix: Update Project Structure tree, Component hierarchy, and all `frontend/src/app.rs` path references.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Unfinished Work Section Stale

- [ ] **#307 — 4 of 5 Unfinished Work items are now completed**
  - File: `CLAUDE.md` (Unfinished Work section)
  - Problem: Sidebar navigation, dark/light toggle, toast notifications, and confirmation modals are all implemented.
  - Fix: Remove completed items and update remaining items.
  - Source commands: `cross-ref-check`

### Documentation — Assessment Command Files Reference Stale `app.rs` Path

- [ ] **#308 — 3 command files reference `frontend/src/app.rs` as the frontend source**
  - Files: `.claude/commands/review.md`, `.claude/commands/security-audit.md`, `.claude/commands/test-gaps.md`
  - Problem: Frontend was refactored from monolithic `app.rs` to `api.rs` + `pages/` + `components/` modules.
  - Fix: Update references to `frontend/src/pages/`, `frontend/src/components/`, `frontend/src/api.rs`.
  - Source commands: `cross-ref-check`

### Testing — Zero WASM Tests for 6 New Frontend Pages

- [ ] **#309 — `admin.rs`, `items.rs`, `orders.rs`, `profile.rs`, `roles.rs`, `teams.rs` have no test coverage (~2,800 lines)**
  - File: `frontend/tests/ui_tests.rs`
  - Problem: All 27 existing WASM tests cover only login/dashboard/session flows. The 6 new pages have zero tests.
  - Fix: Add WASM tests for each page: rendering, API interaction mocking, form validation, error states.
  - Source commands: `test-gaps`

## Minor Items

### Security — Swagger UI Exposed in Production

- [ ] **#112 — `/explorer` registered unconditionally regardless of environment**
  - File: `src/routes.rs`
  - Problem: In production, this exposes the complete API schema, aiding attacker reconnaissance.
  - Fix: Conditionally register the Swagger UI scope only when `ENV != production`, or gate behind admin auth.
  - Source commands: `security-audit`

### Documentation — Frontend Test Category Breakdown Wrong

- [ ] **#163 — CLAUDE.md test category breakdown is stale — should reflect 27 tests (was claiming 23)**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Problem: Categories total does not match actual 27 WASM tests. Need to add theme toggle (4 tests), token refresh (2 tests) categories and update existing category counts.
  - Fix: Recount all test categories and update the breakdown to sum to 27.
  - Source commands: `cross-ref-check`

### Frontend — Login Shows "Invalid Credentials" for All Non-2xx Errors

- [ ] **#225 — HTTP 500, 429, and 503 responses all display "Invalid username or password"**
  - File: `frontend/src/pages/login.rs`
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

- [ ] **#246 — CLAUDE.md test counts do not match actual counts**
  - File: `CLAUDE.md` (Testing → Backend section)
  - Problem: Actual counts are 189 unit tests (167 lib + 22 healthcheck), 86 API integration, 90 DB integration, 27 WASM frontend = 392 total. CLAUDE.md has different numbers.
  - Fix: Update all test counts in CLAUDE.md to match actuals.
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

### Security — `revoke_user_token` Returns HTTP 500 for Expired/Malformed Tokens

- [ ] **#298 — `verify_jwt` in `revoke_user_token` propagates `Error::Jwt` → 500 for expired tokens submitted for revocation**
  - File: `src/handlers/users.rs` (line ~138)
  - Problem: When a legitimately-expired (but validly-signed) token is submitted for revocation, `verify_jwt` returns `Error::Jwt` which maps to HTTP 500 "Internal server error". The user gets no actionable information.
  - Fix: Either catch `Error::Jwt` in the handler and return `HttpResponse::BadRequest().json(ErrorResponse { error: "Token is invalid or expired" })`, or use a `Validation` with `validate_exp = false` for the revocation-specific verify call (revoking an expired token is harmless, and signature verification is still performed).
  - Source commands: `security-audit`

### RBAC — `create_order_item` Uses Broad `require_team_member` Guard

- [ ] **#310 — Any team member (including Guest) can create order items on any team order**
  - File: `src/handlers/orders.rs`
  - Problem: Uses `require_team_member` which allows any team member to add items to any order, regardless of ownership.
  - Fix: Consider whether this is intended policy. If not, restrict to order owner + team admin + global admin.
  - Source commands: `rbac-rules`

### RBAC — Policy Table Missing Order Items as Resource

- [ ] **#311 — CLAUDE.md RBAC documentation does not cover `order_items` as a distinct resource**
  - File: `CLAUDE.md` (RBAC sections)
  - Problem: The policy table only documents `teamorders` RBAC. Per-item authorization rules are undocumented.
  - Fix: Add an "Order Items" section to the RBAC documentation with the intended policy.
  - Source commands: `rbac-rules`

### OpenAPI — `create_user` Missing 409 Conflict Response Annotation

- [ ] **#312 — Handler returns 409 on duplicate email but utoipa only documents 201/400/401/403/404**
  - File: `src/handlers/users.rs`
  - Fix: Add `(status = 409, description = "Conflict", body = ErrorResponse)`.
  - Source commands: `openapi-sync`

### Validation — `create_team_order` and `update_team_order` Missing `validate()` Calls

- [ ] **#313 — Unlike other mutation endpoints, these two handlers do not call `validate(&json)?` before DB operations**
  - File: `src/handlers/teams.rs`
  - Problem: Convention is to always call `validate()` before DB calls.
  - Fix: Either add `validate(&json)?` calls, or formally document these endpoints as exceptions.
  - Source commands: `openapi-sync`, `practices-audit`

### Database — `get_member_role` Uses `query()` Not `query_opt()`

- [ ] **#314 — Non-existent membership returns 500 instead of a clean error**
  - File: `src/db/membership.rs`
  - Fix: Use `query_opt()` + `ok_or_else(|| Error::NotFound(...))` per project convention.
  - Source commands: `db-review`

### Database — Missing ORDER BY on `get_user_teams` and `get_team_users`

- [ ] **#315 — Results returned in arbitrary order, which may vary between queries**
  - Files: `src/db/teams.rs` (`get_user_teams`, `get_team_users`)
  - Fix: Add `ORDER BY tname` (or `joined DESC`) to ensure deterministic results.
  - Source commands: `db-review`

### Database — `UserInTeams` Model Missing `descr` Field

- [ ] **#316 — Query SELECTs team name but not description — frontend cannot show team descriptions**
  - Files: `src/db/teams.rs`, `src/models.rs` (`UserInTeams` struct)
  - Fix: Add `teams.descr` to the SELECT clause and `descr: Option<String>` to `UserInTeams`.
  - Source commands: `db-review`, `api-completeness`

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
  - File: `frontend/src/pages/dashboard.rs`
  - Source commands: `review`

### Frontend — Missing `aria-busy` on Submit Button

- [ ] **#127 — No `aria-busy` attribute during login form submission**
  - File: `frontend/src/pages/login.rs`
  - Source commands: `review`

### Frontend — Decorative Icons Lack Accessibility Attributes

- [ ] **#128 — Warning icon and checkmark lack `aria-hidden="true"`**
  - File: `frontend/src/pages/login.rs`
  - Source commands: `review`

### Frontend — Inconsistent `spawn_local` Import

- [ ] **#210 — Session restore uses `wasm_bindgen_futures::spawn_local` while logout uses `leptos::task::spawn_local`**
  - File: `frontend/src/app.rs`
  - Source commands: `review`

### Frontend — Form Has Redundant Double Validation

- [ ] **#211 — `<form>` has both native HTML5 validation and custom JavaScript validation**
  - File: `frontend/src/pages/login.rs`
  - Source commands: `review`

### Frontend — Loading Page Spinner Not Announced to Screen Readers

- [ ] **#231 — Loading spinner container lacks `role="status"` and `aria-live`**
  - File: `frontend/src/pages/loading.rs`
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

### Frontend — Signal-Inside-Reactive-Closure Anti-Pattern in 5 Pages

- [ ] **#317 — `teams.rs`, `orders.rs`, `items.rs`, `roles.rs`, `admin.rs` create signals inside `move || {}` closures**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Problem: Creating `ReadSignal`/`WriteSignal` pairs inside move closures leaks reactive nodes.
  - Fix: Use `StoredValue` or move signal creation outside closures into component scope.
  - Source commands: `review`

### Frontend — Duplicated `role_tag_class()` Function Across 4 Files

- [ ] **#318 — Same role-to-CSS-class mapping repeated in 4 frontend files**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/admin.rs`
  - Fix: Extract to a shared helper in `frontend/src/components/` or a `utils.rs` module.
  - Source commands: `review`

### Frontend — Duplicated `LoadingSpinner` Markup in 5 Pages

- [ ] **#319 — Same loading spinner HTML pattern repeated in 5 page files**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Fix: Extract to a shared `LoadingSpinner` component.
  - Source commands: `review`

### Frontend — `sleep_ms` Uses `js_sys::eval` in Production Code

- [ ] **#320 — `sleep_ms` helper uses `js_sys::eval` to create a Promise-based sleep**
  - File: `frontend/src/pages/login.rs`
  - Problem: `eval` is a code-smell in production (CSP implications, fragility).
  - Source commands: `review`

### Frontend — Uses `String` for UUIDs Everywhere

- [ ] **#321 — No type safety for UUID fields in frontend API types**
  - File: `frontend/src/api.rs`
  - Problem: All ID fields are `String`. A typo or wrong field could silently produce invalid requests.
  - Source commands: `review`

### Testing — Zero Tests for Shared Frontend Components

- [ ] **#322 — `modal.rs`, `toast.rs`, `sidebar.rs`, `card.rs`, `icons.rs`, `theme_toggle.rs` have no WASM tests**
  - Files: `frontend/src/components/`, `frontend/tests/ui_tests.rs`
  - Source commands: `test-gaps`

### Testing — Order-Item RBAC Bug Has Zero Test Coverage

- [ ] **#323 — No integration test verifies that a team member cannot modify another member's order items**
  - Files: `tests/api_tests.rs`, `src/handlers/orders.rs`
  - Problem: The RBAC privilege escalation in #302/#303 was never caught because no negative-path test exists. HIGH PRIORITY.
  - Source commands: `test-gaps`, `rbac-rules`

### Dependencies — `tokio-postgres` Unused `serde_json` Feature

- [ ] **#324 — `with-serde_json-1` feature enabled but no query uses JSON columns**
  - File: `Cargo.toml` (tokio-postgres dependency)
  - Fix: Remove `"with-serde_json-1"` from features list.
  - Source commands: `dependency-check`

### Database — `orders.orders_team_id` May Be Missing NOT NULL

- [ ] **#325 — Advisory: verify that `orders_team_id` FK column has NOT NULL**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs`
  - Source commands: `db-review`

### OpenAPI — Order-Item Endpoint 403 Descriptions Are Imprecise

- [ ] **#326 — `create_order_item`, `update_order_item`, `delete_order_item` utoipa 403 descriptions do not match actual RBAC guards**
  - File: `src/handlers/orders.rs`
  - Fix: Update 403 descriptions to match actual RBAC policy once #302/#303 are fixed.
  - Source commands: `openapi-sync`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 392 tests pass: 189 backend unit (167 lib + 22 healthcheck), 86 API integration, 90 DB integration, 27 WASM. Total: **392 tests, 0 failures**.
- Backend unit test breakdown: config: 6, db/migrate: 34, errors: 16, from_row: 10, handlers/mod: 12, middleware/auth+openapi: 32, models: 12, routes: 19, server: 17, validate: 9, healthcheck: 22 = **189 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. `rsa` 0.10.0 remains at rc.16. Re-evaluate periodically.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` has diffs in `src/middleware/auth.rs` (see #304) and frontend files (see #305).
- CONNECT Design System: `git pull` reports "Already up to date" — no migration needed.
- Frontend was refactored from monolithic `app.rs` (600+ lines) into modular architecture: `api.rs` (377 lines), `pages/` (10 files, ~2,800 lines), `components/` (7 files, ~680 lines). `app.rs` is now 164 lines (routing shell only).
- Frontend consumes 22 of 37 API endpoints (up from 4 at last assessment).
- RBAC enforcement has 2 important privilege escalation vectors in order-item handlers (#302, #303). All other handlers are correct.
- OpenAPI spec has 41 operations; annotation inaccuracies tracked (#244, #245, #271, #276, #277, #287, #312, #313, #326).
- All SQL queries use parameterized prepared statements — zero injection risk.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- 3 resolved findings archived: #71 (monolithic app.rs), #116 (frontend endpoint consumption), #297 (db_tests.rs fmt drift).
- Open items summary: 1 critical (#132 blocked), 8 important, 35 minor, 61 informational. **Total: 105 open items**.
- 188 resolved items in `.claude/resolved-findings.md`.
- Highest finding number: #326.
