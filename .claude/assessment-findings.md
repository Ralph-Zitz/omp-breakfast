# Assessment Findings

Last assessed: 2026-03-04

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

*No open important items.*

## Minor Items

*No open minor items.*

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

- [x] **#126 — Dashboard state stored in enum variant, cloned on every re-render**
  - File: `frontend/src/pages/dashboard.rs`
  - Source commands: `review`

### Frontend — Missing `aria-busy` on Submit Button

- [x] **#127 — No `aria-busy` attribute during login form submission**
  - File: `frontend/src/pages/login.rs`
  - Source commands: `review`

### Frontend — Decorative Icons Lack Accessibility Attributes

- [x] **#128 — Warning icon and checkmark lack `aria-hidden="true"`**
  - File: `frontend/src/pages/login.rs`
  - Source commands: `review`
  - Note: Already resolved — both icons already had `aria-hidden="true"` at the time of review.

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
  - Files: `frontend/src/pages/dashboard.rs`, `frontend/src/pages/teams.rs`, `frontend/src/pages/profile.rs`, `frontend/src/pages/roles.rs`
  - Problem: `role_tag_class()` is duplicated with identical logic. `dashboard.rs`/`teams.rs` return `String`; `profile.rs`/`roles.rs` return `&'static str` — inconsistent signatures.
  - Fix: Extract to a shared helper in `frontend/src/components/` or a `utils.rs` module; prefer `&'static str` return type.
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

### Security — Swagger UI Gated by Negation Rather Than Explicit Opt-In

- [ ] **#336 — Swagger UI at `/explorer` available in all non-production environments (staging, preprod, etc.)**
  - File: `src/routes.rs` (lines ~33–35)
  - Problem: Gate is `ENV != "production"`, so staging/preprod environments expose full API docs. Consider explicit opt-in (e.g., `ENABLE_SWAGGER=true`).
  - Source commands: `security-audit`

### Security — Refresh Token Rotation Doesn't Revoke Old Access Token

- [ ] **#337 — When refresh token is used to obtain a new pair, the old access token remains valid up to 15 minutes**
  - File: `src/handlers/users.rs` (lines ~121–129)
  - Problem: Only the old refresh token is revoked. An attacker with both tokens could still use the access token.
  - Source commands: `security-audit`

### Security — HSTS Header Missing `preload` Directive

- [ ] **#338 — HSTS value is `max-age=31536000; includeSubDomains` but lacks `preload`**
  - File: `src/server.rs` (line ~443)
  - Problem: Without `preload`, the first visit is vulnerable to MITM downgrade.
  - Source commands: `security-audit`

### Security — Account Lockout State In-Memory Only

- [ ] **#339 — Login attempt tracking stored in `DashMap`, not shared across instances**
  - File: `src/middleware/auth.rs` (lines ~189–213)
  - Problem: In multi-instance deployment, attacker can distribute brute-force attempts across instances.
  - Source commands: `security-audit`

### Frontend — No Client-Side `maxlength` on Form Inputs

- [ ] **#340 — Frontend input fields lack `maxlength` attributes matching backend validation rules**
  - Files: `frontend/src/pages/login.rs`, `frontend/src/pages/admin.rs`, `frontend/src/pages/profile.rs`
  - Problem: Arbitrarily long strings transmitted to server before backend validator catches them.
  - Fix: Add `maxlength=50` on name fields, `maxlength=128` on password, `maxlength=255` on team/role names.
  - Source commands: `security-audit`

### Testing — `verify_jwt_for_revocation` Has Zero Dedicated Unit Tests

- [ ] **#349 — Security-sensitive function that skips expiry validation has no test verifying expired-but-valid tokens are accepted**
  - File: `src/middleware/auth.rs` (lines ~124–136)
  - Problem: This function intentionally sets `validate_exp = false`. No test confirms that an expired token with a valid signature passes, or that a tampered token still fails.
  - Source commands: `test-gaps`

### Testing — `basic_validator` Malformed Password Hash Path Untested

- [ ] **#350 — When DB stores a corrupted/non-Argon2 hash, `PasswordHash::new()` fails and returns 500 — no test**
  - File: `src/middleware/auth.rs` (lines ~484–498)
  - Source commands: `test-gaps`

### Testing — `jwt_validator` Rejects Refresh Token — No Explicit Test

- [ ] **#351 — The `if c.claims.token_type != TokenType::Access` branch returns 401 but is never directly tested**
  - File: `src/middleware/auth.rs` (lines ~230–248)
  - Problem: The reverse (refresh endpoint rejects access token) is tested, but no test submits a refresh token to a JWT-protected API endpoint.
  - Source commands: `test-gaps`

### Testing — `validate_non_negative_price` Has No Unit Tests

- [ ] **#352 — Custom validator for item price never directly tested (negative, zero, positive cases)**
  - File: `src/models.rs` (lines ~301–312)
  - Source commands: `test-gaps`

### Testing — No Boundary Tests for `CreateUserEntry` Name Fields

- [ ] **#353 — firstname/lastname max=50 boundary untested (50 chars should pass, 51 should fail)**
  - File: `src/models.rs` (lines ~159–185)
  - Source commands: `test-gaps`

### Testing — No Boundary Tests for Team/Role/Item Model Field Lengths

- [ ] **#354 — `tname` max=255, `descr` max=1000, role `title` max=255, item `descr` max=255 — all untested at boundary**
  - File: `src/models.rs`
  - Source commands: `test-gaps`

### Testing — Non-Owner Member Cannot Update/Delete Team Order — Untested

- [ ] **#355 — A team member who didn't create the order, and is not a team admin, tries PUT/DELETE — no test**
  - File: `tests/api_tests.rs`
  - Problem: Differs from #291 (non-member). This concerns a member who is not the order creator.
  - Source commands: `test-gaps`

### Testing — `ActixJson` Deserialize Error Branch Untested

- [ ] **#356 — `JsonPayloadError::Deserialize` with `.data()` → 422 path has no test (only parse error is tested)**
  - File: `src/errors.rs` (lines ~82–118)
  - Problem: Sending valid JSON with wrong field types (type mismatch) hits a different `JsonPayloadError` variant than malformed JSON.
  - Source commands: `test-gaps`

### Testing — Frontend Orders Page Interactive Flows Untested

- [ ] **#357 — Add-item, remove-item, create/delete order interactions have no WASM tests**
  - Files: `frontend/src/pages/orders.rs`, `frontend/tests/ui_tests.rs`
  - Problem: The 721-line orders page has extensive interactive logic, all untested.
  - Source commands: `test-gaps`

### Testing — Frontend Profile Page Password Change Flow Untested

- [ ] **#358 — Edit mode, password validation, and save logic have no WASM tests**
  - Files: `frontend/src/pages/profile.rs`, `frontend/tests/ui_tests.rs`
  - Source commands: `test-gaps`

### Testing — `DbMapper::Conversion` Error Variant Returns 500 — Never Tested

- [ ] **#359 — Only `ColumnNotFound` sub-variant is tested; `Conversion` has its own log-and-respond branch with zero coverage**
  - File: `src/errors.rs` (lines ~124–140)
  - Source commands: `test-gaps`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 416 tests pass: 193 backend unit (171 lib + 22 healthcheck), 87 API integration, 92 DB integration, 41 WASM, 3 doc tests. Total: **416 tests, 0 failures**.
- Backend unit test breakdown: config: 6, db/migrate: 34, errors: 16, from_row: 10, handlers/mod: 12, middleware/auth+openapi: 32, models: 16, routes: 19, server: 17, validate: 9, healthcheck: 22 = **193 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. `rsa` 0.10.0 remains at rc.16. Re-evaluate periodically.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- CONNECT Design System: `git pull` reports "Already up to date" — no migration needed.
- Frontend was refactored from monolithic `app.rs` (600+ lines) into modular architecture: `api.rs` (377 lines), `pages/` (10 files, ~2,800 lines), `components/` (7 files, ~680 lines). `app.rs` is now 164 lines (routing shell only).
- Frontend consumes 22 of 37 API endpoints (up from 4 at last assessment).
- RBAC enforcement: no violations found. All 30 handlers enforce correct guards per CLAUDE.md policy.
- OpenAPI spec has 41 operations; remaining annotation inaccuracies tracked (#287, #326).
- All SQL queries use parameterized prepared statements — zero injection risk.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 1 critical (#132 blocked), 0 important, 0 minor, 74 informational. **Total: 75 open items**.
- 33 new findings in this assessment: #327–#359. 17 resolved in this session (#327–#335, #341–#348).
- 251 resolved items in `.claude/resolved-findings.md`.
- Highest finding number: #359.
