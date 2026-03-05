# Assessment Findings

Last assessed: 2026-03-06

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

## Minor Items

### Documentation — CLAUDE.md Backend Test Counts Stale

- [ ] **#404 — CLAUDE.md states 193 unit, 87 API, 96 DB tests; actual counts are 195 unit, 109 API, 101 DB**
  - File: `CLAUDE.md` (Testing section)
  - Source commands: `cross-ref-check`

### Documentation — README Test Counts Stale

- [ ] **#405 — README.md states 193 unit, 87 API, 92 DB; actual counts are 195, 109, 101**
  - File: `README.md`
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md `db/users.rs` Function List Incomplete

- [ ] **#406 — `get_password_hash` missing from the parenthetical function list**
  - File: `CLAUDE.md` (Project Structure → `db/users.rs`)
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Structure Tree Missing Root Files

- [ ] **#407 — `NEW-UI-COMPONENTS.md` and `LICENSE` exist on disk but not in project structure tree**
  - File: `CLAUDE.md` (Project Structure)
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Security Headers Omits `Permissions-Policy`

- [ ] **#415 — `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()` is set in `DefaultHeaders` but not documented**
  - File: `CLAUDE.md` (Security headers bullet), `src/server.rs` (line ~444)
  - Source commands: `practices-audit`

### Database — Redundant Indexes Duplicate UNIQUE Constraint Auto-Indexes

- [ ] **#408 — `idx_users_email` and `idx_teams_name` duplicate the implicit unique indexes from UNIQUE constraints**
  - File: `migrations/V1__initial_schema.sql` (lines ~25, ~38)
  - Fix: Add migration to `DROP INDEX IF EXISTS idx_users_email; DROP INDEX IF EXISTS idx_teams_name;`
  - Source commands: `db-review`

### Database — Pagination Count and Data Queries Not Transactionally Consistent

- [ ] **#409 — `SELECT COUNT(*)` and `SELECT ... LIMIT/OFFSET` run as separate statements; total can be stale relative to items**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Fix: Wrap in explicit transaction or use `COUNT(*) OVER()` window function.
  - Source commands: `db-review`

### Database — `get_order_items` ORDER BY UUID Gives Non-Meaningful Sort

- [ ] **#410 — `ORDER BY orders_item_id` sorts by item UUID primary key, not by when the item was added or by name**
  - File: `src/db/order_items.rs` (line ~84)
  - Fix: Change to `ORDER BY created` or `ORDER BY items.descr` via JOIN.
  - Source commands: `db-review`

### Dependencies — `tracing-bunyan-formatter` Effectively Unmaintained

- [ ] **#411 — v0.3.10 (last release Feb 2024) causes `tracing-log` v0.1/v0.2 duplication and pulls stale transitive deps**
  - File: `Cargo.toml`
  - Fix: Replace with custom JSON layer via `tracing-subscriber::fmt::layer().json()`.
  - Source commands: `dependency-check`

### OpenAPI — `create_order_item` Missing 404 Response

- [ ] **#412 — `guard_open_order` returns 404 when team order doesn't exist, but utoipa annotation omits 404**
  - File: `src/handlers/orders.rs` (lines ~68–82)
  - Fix: Add `(status = 404, description = "Team order or item not found", body = ErrorResponse)`.
  - Source commands: `openapi-sync`

### OpenAPI — Member Management 403 Descriptions Omit Admin-Role Guard

- [ ] **#413 — `add_team_member` and `update_member_role` 403 descriptions say only "team admin role required" but omit `guard_admin_role_assignment` scenario**
  - File: `src/handlers/teams.rs` (lines ~358–372, ~431–445)
  - Fix: Update to "Forbidden — team admin role required, or only global admins can assign the Admin role".
  - Source commands: `openapi-sync`

### OpenAPI — `create_team_order` Missing 422 Response

- [ ] **#414 — Handler calls `validate(&json)?` but utoipa annotation omits 422**
  - File: `src/handlers/teams.rs` (line ~228)
  - Fix: Add `(status = 422, description = "Validation error", body = ErrorResponse)`.
  - Source commands: `rbac-rules`

### Security — `Error::ActixAuth` Leaks Raw Actix Error Messages

- [ ] **#416 — `ActixAuth` variant returns `e.to_string()` directly in 401 response body, potentially exposing internal framework details**
  - File: `src/errors.rs` (lines ~131–134)
  - Fix: Return generic `"Authentication failed"` instead of raw actix error string.
  - Source commands: `security-audit`

### Security — No `Cache-Control` on Authenticated GET Endpoints

- [ ] **#417 — Authenticated GET responses lack `Cache-Control: no-store` — browsers/proxies may cache sensitive data**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/roles.rs`, `src/handlers/items.rs`, `src/handlers/orders.rs`
  - Fix: Add `Cache-Control: no-store, private` via `DefaultHeaders` for the `/api/v1.0` scope.
  - Source commands: `security-audit`

### Security — No Guard That `jwtsecret` ≠ `secret`

- [ ] **#418 — Production startup guards reject default values individually but don't check if both are set to the same custom value**
  - File: `src/server.rs` (lines ~297–316)
  - Fix: Add `if settings.server.secret == settings.server.jwtsecret { panic!("...") }`.
  - Source commands: `security-audit`

### Security — Default Config Plaintext Secrets in Docker Image

- [ ] **#419 — `default.yml` with `secret: "Very Secret"` and `password: actix` is copied into the final Docker image**
  - File: `Dockerfile.breakfast` (line ~81), `config/default.yml`
  - Fix: Copy only `production.yml` into final image, or strip secrets from baked `default.yml`.
  - Source commands: `security-audit`

### Frontend — Missing Edit UI for Teams, Items, and Roles

- [ ] **#420 — `PUT /teams/{id}`, `PUT /items/{id}`, `PUT /roles/{id}` exist but no frontend edit forms**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`
  - Source commands: `api-completeness`

### Frontend — No Team Member Management UI

- [ ] **#421 — Backend POST/DELETE/PUT on team members fully implemented; frontend shows read-only member table only**
  - File: `frontend/src/pages/teams.rs`
  - Source commands: `api-completeness`

### Frontend — No Order Update/Close UI or Order Item Quantity Edit

- [ ] **#422 — `PUT /teams/{id}/orders/{oid}` (close/reopen, due date) and `PUT .../items/{iid}` (quantity) exist but no frontend UI**
  - File: `frontend/src/pages/orders.rs`
  - Source commands: `api-completeness`

### Frontend — No Pagination Controls

- [ ] **#423 — All list endpoints return paginated responses but no page has next/previous/page controls; lists truncated at 50**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Source commands: `api-completeness`

### Frontend — No Admin Edit-User UI

- [ ] **#424 — AdminPage shows user list with create/delete but no edit form; only ProfilePage supports self-edit**
  - File: `frontend/src/pages/admin.rs`
  - Source commands: `api-completeness`

### Frontend — Create User Gated Admin-Only in UI but Backend Allows Team Admin

- [ ] **#425 — `require_admin_or_team_admin` allows team admins to create users, but Admin page is only visible to global admins**
  - File: `frontend/src/pages/admin.rs`
  - Problem: Team admins have no UI path to create users.
  - Source commands: `api-completeness`

### Frontend — Profile Save Duplicates `build_user_context()` Logic

- [ ] **#426 — After PUT, profile page manually fetches user + teams + checks admin, duplicating `build_user_context()` from api.rs**
  - File: `frontend/src/pages/profile.rs` (lines ~69–101)
  - Fix: Call `build_user_context()` instead.
  - Source commands: `review`

### Frontend — Profile Save Discards PUT Response, Makes 2 Extra GETs

- [ ] **#427 — Successful PUT response body is not read; code makes separate GET for user and GET for teams**
  - File: `frontend/src/pages/profile.rs` (lines ~76–78)
  - Fix: Deserialize PUT response for user data; only teams fetch needed.
  - Source commands: `review`

### Frontend — No Client-Side Email Validation on Profile Edit

- [ ] **#428 — Invalid email accepted client-side, rejected server-side with generic toast**
  - File: `frontend/src/pages/profile.rs` (lines ~239–253)
  - Fix: Add basic email format check to disabled expression.
  - Source commands: `review`

### Testing — Team Admin Bulk-Delete Orders Positive Path Untested

- [ ] **#429 — Admin bypass tested, member denied tested, but no test where Team Admin bulk-deletes orders on own team**
  - File: `tests/api_tests.rs`
  - Source commands: `rbac-rules`

### Testing — Team Admin Update/Delete Another Member's Order Untested

- [ ] **#430 — No test where Team Admin (non-owner) updates or deletes an order created by a regular member**
  - File: `tests/api_tests.rs`
  - Source commands: `rbac-rules`

### Testing — Order Owner Update/Delete Own Order Positive Path Untested

- [ ] **#431 — No test where a regular member (order creator) updates or deletes their own order and gets 200**
  - File: `tests/api_tests.rs`
  - Source commands: `rbac-rules`

### Testing — Duplicate Team Name Conflict Not Tested via API

- [ ] **#432 — No API test creates a team with an existing name and asserts 409**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — Negative Price Rejection Not Tested via API

- [ ] **#433 — No API test sends a negative price to `POST /items` and asserts 422**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `PaginationParams::sanitize()` Clamping Untested

- [ ] **#434 — No test sends `limit=200` or `offset=-5` and verifies clamped pagination metadata**
  - File: `src/models.rs` (lines ~31–38), `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — Self-Delete User by Email Untested

- [ ] **#435 — No API test verifies a non-admin user can delete their own account by email**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `create_team` Duplicate Name Not Tested at DB Level

- [ ] **#436 — No DB test attempts to create a team with an existing name (UNIQUE constraint)**
  - File: `tests/db_tests.rs`
  - Source commands: `test-gaps`

### Testing — `create_role` Duplicate Title Not Tested at DB Level

- [ ] **#437 — No DB test for creating a role with a duplicate title**
  - File: `tests/db_tests.rs`
  - Source commands: `test-gaps`

## Informational Items

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
  - Files: `frontend/src/api.rs` (line ~372), `frontend/src/components/toast.rs` (line ~75)
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

### Frontend — Sidebar Uses `user.get()` Which Clones Full `UserContext` on Each Render

- [ ] **#360 — `Sidebar` calls `user.get()` inside reactive closures, cloning the entire `UserContext` (including `teams: Vec<UserInTeams>`) on every re-render**
  - Files: `frontend/src/components/sidebar.rs` (lines ~87 and ~119)
  - Problem: Two locations: (1) `user.get().map(|u| u.is_admin).unwrap_or(false)` clones the full struct just to read one `bool`; (2) `user.get().map(|u| { let initials = ...; ... })` clones to render the sidebar user card. This is the same pattern fixed for `dashboard.rs` in #126 but was not carried through to `sidebar.rs`.
  - Fix: Replace `user.get()` with `user.with(|u| ...)` at both locations (identical pattern to the #126 fix).
  - Source commands: `review`

### Frontend — `authed_request` Collapses All Errors to `Option`

- [ ] **#364 — `authed_request()` returns `Option<Response>`, discarding HTTP error codes and network errors**
  - File: `frontend/src/api.rs` (lines ~266–296)
  - Problem: Callers cannot distinguish 403 (forbidden) from 500 (server error) from network failure. All treated identically as `None`.
  - Source commands: `review`

### API Completeness — Frontend `UserInTeams` Missing `team_id` and `descr` Fields

- [ ] **#365 — Frontend `UserInTeams` struct lacks `team_id` and `descr` that the backend now provides**
  - Files: `frontend/src/api.rs` (line ~95), `src/models.rs` (line ~227)
  - Problem: Backend #301 fix added `team_id: Uuid` and `descr: Option<String>` to the backend model and query, but the frontend struct was not updated. Extra JSON fields are silently dropped during deserialization.
  - Fix: Add `pub team_id: String` and `pub descr: Option<String>` to the frontend `UserInTeams` struct.
  - Source commands: `api-completeness`

### API Completeness — Frontend `ItemEntry.price` Typed as `String`

- [ ] **#366 — Frontend `ItemEntry` uses `pub price: String` instead of a numeric type**
  - File: `frontend/src/api.rs`
  - Problem: Backend returns `numeric(10,2)` as a JSON number; frontend deserializes as `String` which works but loses type safety for display and arithmetic.
  - Source commands: `api-completeness`

### Frontend — Create Dialogs Don't Reset Form State on Cancel

- [ ] **#367 — Closing a create dialog without submitting leaves stale values in form fields**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Problem: When user opens "New Team" dialog, types partial data, then cancels, reopening the dialog shows the previously typed values.
  - Fix: Reset form signals in the cancel/close handler.
  - Source commands: `review`

### Frontend — `OrderDetail` Add-Item Form Doesn't Reset on Order Change

- [ ] **#368 — Selecting a different order retains the previously selected item and quantity in the add-item form**
  - File: `frontend/src/pages/orders.rs`
  - Source commands: `review`

### Frontend — Fetch JSON Deserialization Errors Silently Swallowed in 5 Pages

- [ ] **#369 — `.json::<T>().await.unwrap_or_default()` hides deserialization failures**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Problem: If the backend response schema changes, the frontend silently shows empty data instead of reporting an error.
  - Source commands: `review`

### Documentation — `db-review.md` Factually Incorrect Description of `init_dev_db.sh`

- [ ] **#370 — `.claude/commands/db-review.md` describes `init_dev_db.sh` as initialising the development database; it actually initialises the Docker Postgres entrypoint**
  - File: `.claude/commands/db-review.md`
  - Source commands: `cross-ref-check`

### Documentation — `README.md` Make Targets Table Missing 3 Targets

- [ ] **#371 — README lists Make targets but omits `check`, `fmt`, and `audit-install`**
  - File: `README.md`
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Key Conventions Omits `BREAKFAST_` Env Var Prefix

- [ ] **#372 — Config env var override prefix `BREAKFAST_` (set in `config/default.yml`) not documented in Key Conventions**
  - File: `CLAUDE.md`
  - Source commands: `cross-ref-check`

### Database — `token_blacklist.revoked_at` Lacks NOT NULL Constraint

- [ ] **#373 — `revoked_at TIMESTAMPTZ DEFAULT NOW()` has no NOT NULL; a manual INSERT could omit it**
  - File: `migrations/V1__initial_schema.sql`
  - Source commands: `db-review`

### Database — `idx_teamorders_id_due` Index Unused by Any Query

- [ ] **#374 — Covering index on `(orders_team_id, due)` is never used; all order queries filter by `team_id` alone or by primary key**
  - File: `migrations/V6__order_constraint_and_index.sql`
  - Source commands: `db-review`

### Code Quality — Identical Create/Update Model Pairs in `models.rs`

- [ ] **#375 — `CreateTeamEntry`/`UpdateTeamEntry`, `CreateRoleEntry`/`UpdateRoleEntry`, `CreateItemEntry`/`UpdateItemEntry` have identical fields**
  - File: `src/models.rs`
  - Problem: 3 pairs of structs are field-identical. Could be unified or type-aliased to reduce boilerplate.
  - Source commands: `review`

### Code Quality — `#[derive(Validate)]` with No Validation Attributes on 4 Structs

- [ ] **#376 — `UpdateTeamEntry`, `UpdateRoleEntry`, `UpdateItemEntry`, `UpdateTeamOrderEntry` derive `Validate` but have no `#[validate(...)]` field attributes**
  - File: `src/models.rs`
  - Problem: The `validate()` call does nothing — it always succeeds. Either add field-level validation or remove the derive.
  - Source commands: `review`

### Code Quality — `healthcheck.rs` Builds Unused `root_store` Variable

- [ ] **#377 — `root_store` is created then shadowed or never read in the healthcheck binary**
  - File: `src/bin/healthcheck.rs`
  - Source commands: `review`

### Code Quality — `db_tls_connector` Panics Instead of Returning Result

- [ ] **#378 — `db_tls_connector()` in `server.rs` uses `.expect()` on certificate loading, panicking at runtime if certs are missing**
  - File: `src/server.rs`
  - Problem: A missing cert file causes a panic with no structured error. Should return `Result` and let the caller handle it.
  - Source commands: `review`

### Security — Password Fields Lack `autocomplete="new-password"`

- [ ] **#379 — Profile page password input missing `autocomplete` attribute**
  - File: `frontend/src/pages/profile.rs` (line ~207)
  - Problem: Without `autocomplete="new-password"`, password managers may not offer to save the new password.
  - Source commands: `security-audit`

### Security — Argon2id `Params::default()` Below OWASP Minimum

- [ ] **#380 — Default Argon2id parameters (19 MiB, 2 iterations, 1 lane) are below OWASP recommendation (46 MiB, 1 iteration, 1 lane)**
  - File: `src/middleware/auth.rs`
  - Problem: The `Params::default()` values are the `argon2` crate defaults, not the OWASP-recommended profile. For an internal app this is low risk but worth noting.
  - Source commands: `security-audit`

### Security — JWT Validator Performs DB Lookup on Every Request

- [ ] **#381 — `jwt_validator` calls `db::get_user_by_email` on every authenticated request after cache miss**
  - File: `src/middleware/auth.rs`
  - Problem: Informational. The auth cache mitigates this for warm paths. Cold requests hit the DB. Not a bug, just a performance observation.
  - Source commands: `security-audit`

### Security — No Rate Limiting on Password Change Endpoint

- [ ] **#382 — `PUT /api/v1.0/users/{id}` has no rate limiter for password changes**
  - File: `src/routes.rs`
  - Problem: An attacker with a valid session could brute-force test many passwords via the update endpoint. Low risk because the endpoint requires authentication.
  - Source commands: `security-audit`

### Security — `delete_user_by_email` Email Existence Oracle

- [ ] **#383 — DELETE endpoint returns 404 vs 204, revealing whether an email exists in the system**
  - File: `src/handlers/users.rs`
  - Problem: Low risk — endpoint is admin-gated. But the response difference is observable.
  - Source commands: `security-audit`

### OpenAPI — `delete_user_by_email` Missing 422 Response

- [ ] **#384 — utoipa annotations for `delete_user_by_email` omit the 422 (validation error) response**
  - File: `src/handlers/users.rs`
  - Source commands: `openapi-sync`

### OpenAPI — Auth Endpoints Missing 429 Response

- [ ] **#385 — `auth_user` and `refresh_token` utoipa annotations omit 429 (rate-limited / account locked) response**
  - File: `src/handlers/users.rs`
  - Source commands: `openapi-sync`

### Dependencies — OpenTelemetry Stack Enables Unused `logs` and `metrics` Features

- [ ] **#386 — `opentelemetry` and `opentelemetry_sdk` have `logs` feature enabled; no log exporter is configured**
  - File: `Cargo.toml`
  - Problem: Compiles unused modules. Removing `logs` feature reduces compile time slightly.
  - Source commands: `dependency-check`

### Testing — Token Refresh After User Deletion Untested

- [ ] **#387 — No test refreshes a token after the user has been deleted from the database**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — Admin Assigning Admin Role Has No Positive API Test

- [ ] **#388 — `guard_admin_role_assignment` allows Admin to assign Admin role, but no test exercises this success path**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `auth_user` Cache Miss Path Untested

- [ ] **#389 — No test verifies the code path when the auth cache has no entry for a user (first login or after TTL expiry)**
  - File: `src/middleware/auth.rs`
  - Source commands: `test-gaps`

### Testing — `delete_user_by_email` Invalid Email Format Not Tested

- [ ] **#390 — No API test sends a malformed email string to verify 422 response**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `update_user` Email Change Dual Cache Invalidation Untested

- [ ] **#391 — No test changes a user's email and verifies both old and new cache keys are invalidated**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `GET /teams/{nonexistent}/users` Behavior Untested

- [ ] **#392 — No test verifies whether the endpoint returns 200 `[]` or 404 for a non-existent team**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `check_team_access` for "Team Admin" Role Missing from DB Tests

- [ ] **#393 — DB tests cover Admin bypass and Member access but not Team Admin role specifically**
  - File: `tests/db_tests.rs`
  - Source commands: `test-gaps`

### Testing — Health Endpoint 503 Response Never Tested

- [ ] **#394 — No integration test verifies that `/health` returns HTTP 503 when the database is unreachable**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `refresh_token` `DateTime::from_timestamp` Fallback Untested

- [ ] **#395 — The `DateTime::from_timestamp(exp, 0).unwrap_or_default()` fallback in `refresh_token` handler is never tested**
  - File: `src/handlers/users.rs`
  - Source commands: `test-gaps`

### Testing — Order Entry `amt` Range Validation Untested

- [ ] **#396 — `CreateOrderEntry` and `UpdateOrderEntry` have `#[validate(range(min=1, max=10000))]` on `amt` but no test verifies boundary values**
  - File: `src/models.rs`
  - Source commands: `test-gaps`

### Database — `SET timezone` in V1 Is Session-Scoped Dead Code

- [ ] **#438 — `SET timezone = 'Europe/Copenhagen'` only affects the migration connection session, not application connections**
  - File: `migrations/V1__initial_schema.sql` (line ~10)
  - Source commands: `db-review`

### Database — Seed Data `teamorders` INSERT Not Idempotent

- [ ] **#439 — `ON CONFLICT DO NOTHING` never fires because PK is auto-generated UUID; re-running seed creates duplicates**
  - File: `database_seed.sql` (lines ~178–187)
  - Source commands: `db-review`

### Database — FK Constraint Violations Return Generic 409 Message

- [ ] **#440 — All foreign key violations (23503) map to same opaque message regardless of which relationship is violated**
  - File: `src/errors.rs` (lines ~110–114)
  - Fix: Extract constraint name from DB error for more specific messages.
  - Source commands: `db-review`

### Database — No DB-Level Aggregate Query for Order Totals

- [ ] **#441 — No `get_order_total()` function; frontend must fetch all items and compute totals client-side**
  - File: `src/db/order_items.rs` (absent function)
  - Source commands: `db-review`

### Dependencies — `rustls` `tls12` Feature May Be Unnecessary

- [ ] **#442 — Internal app could enforce TLS 1.3 only by removing `tls12` feature**
  - File: `Cargo.toml` (rustls dependency)
  - Source commands: `dependency-check`

### Dependencies — Three Versions of `getrandom` Compiled

- [ ] **#443 — `getrandom` v0.2, v0.3, and v0.4 all compiled due to ecosystem version split**
  - File: `Cargo.toml` (transitive)
  - Source commands: `dependency-check`

### Dependencies — `refinery` Pulls `toml` 0.8 Alongside `config`'s 0.9

- [ ] **#444 — Duplicates the TOML parser; will resolve when `refinery` upgrades upstream**
  - File: `Cargo.toml` (transitive)
  - Source commands: `dependency-check`

### Dependencies — `opentelemetry-stdout` Used Unconditionally

- [ ] **#445 — Trace spans go to stdout in both dev and production; may conflict with Bunyan JSON logging in prod**
  - File: `Cargo.toml`, `src/server.rs` (line ~13)
  - Source commands: `dependency-check`

### OpenAPI — `update_user` 403 Description Omits Password Verification Failure

- [ ] **#446 — A correct JWT with wrong `current_password` returns an undocumented 403 with different error message**
  - File: `src/handlers/users.rs` (lines ~273–289)
  - Source commands: `openapi-sync`

### Security — JWT HS256 With No Key Rotation Mechanism

- [ ] **#447 — No `kid` claim or multi-key support; compromised secret requires full restart**
  - File: `src/middleware/auth.rs` (lines ~65–70)
  - Source commands: `security-audit`

### Security — `local_storage()` Helper Needs Doc Warning

- [ ] **#448 — Public helper exists alongside `session_storage()`; could invite token misuse by future developers**
  - File: `frontend/src/api.rs` (lines ~187–191)
  - Fix: Add doc-comment warning against storing tokens in localStorage.
  - Source commands: `security-audit`

### Security — Docker Containers Lack Hardening Options

- [ ] **#449 — No `read_only: true`, `security_opt: ["no-new-privileges:true"]`, or `cap_drop: ["ALL"]`**
  - File: `docker-compose.yml`
  - Source commands: `security-audit`

### Security — CORS `allowed_origin` Uses Bind Address

- [ ] **#450 — `allowed_origin(&format!("https://{}:{}", host, port))` produces non-matching origin string; CORS is effectively non-functional**
  - File: `src/server.rs` (lines ~430–435)
  - Source commands: `security-audit`

### Security — `.git` Directory Copied Into Docker Builder Stage

- [ ] **#451 — Full git history in builder image cache; used only for `git_version!()`**
  - File: `Dockerfile.breakfast` (line ~40)
  - Fix: Pass git version as build arg instead.
  - Source commands: `security-audit`

### Frontend — Inconsistent Async Spawning API

- [ ] **#452 — `LogoutButton` uses `leptos::task::spawn_local` while all others use `wasm_bindgen_futures::spawn_local`**
  - File: `frontend/src/components/sidebar.rs` (line ~172)
  - Source commands: `review`

### Code Quality — Auth Cache Eviction O(n)

- [ ] **#453 — `evict_oldest_if_full` iterates all 1000 entries to find oldest; fine at current scale**
  - File: `src/middleware/auth.rs`
  - Source commands: `review`

### Code Quality — `GovernorConfigBuilder::finish().unwrap()` in Production Path

- [ ] **#454 — Should use `.expect("valid rate limiter config")` for better panic message**
  - File: `src/routes.rs` (lines ~22–24)
  - Source commands: `review`

### Code Quality — `format!()` on String Literals

- [ ] **#455 — `format!("Delete User")` etc. allocate unnecessarily; use `.to_string()` instead**
  - Files: `frontend/src/pages/admin.rs` (line ~169), `frontend/src/pages/roles.rs` (line ~164), `frontend/src/pages/items.rs` (line ~165)
  - Source commands: `review`

### Code Quality — OrdersPage File Exceeds 700 Lines

- [ ] **#456 — Contains `OrdersPage`, `OrderDetail`, `CreateOrderDialog`, `LoadingSpinner` — hard to navigate**
  - File: `frontend/src/pages/orders.rs`
  - Fix: Extract `OrderDetail` and `CreateOrderDialog` into components or submodule.
  - Source commands: `review`

### API Completeness — `OrderItemEntry` vs Backend `OrderEntry` Naming Inconsistency

- [ ] **#457 — Frontend renames the struct for clarity but creates naming mismatch with backend**
  - File: `frontend/src/api.rs` (lines ~96–104)
  - Source commands: `api-completeness`

### API Completeness — Bulk Team Order Delete Endpoint Not Consumed

- [ ] **#458 — `DELETE /api/v1.0/teams/{team_id}/orders` exists but has no frontend UI trigger**
  - File: `src/routes.rs`
  - Source commands: `api-completeness`

### API Completeness — Delete-User-by-Email Endpoint Not Consumed

- [ ] **#459 — AdminPage deletes by user_id only; the by-email endpoint is unreachable from UI**
  - File: `src/routes.rs`
  - Source commands: `api-completeness`

### API Completeness — Single-Resource GET Endpoints Not Consumed (×5)

- [ ] **#460 — Frontend always fetches via list endpoints; `GET /teams/{id}`, `GET /items/{id}`, `GET /roles/{id}`, single order, single order item all unused**
  - File: `src/routes.rs`
  - Source commands: `api-completeness`

### Testing — Revoke Already-Revoked Token Idempotency Untested

- [ ] **#461 — No API test calls `POST /auth/revoke` twice with the same token**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `Cache-Control: no-store` Header on Auth Responses Untested

- [ ] **#462 — Both `auth_user` and `refresh_token` set the header but no test asserts its presence**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `ErrorResponse::Display` Fallback Branch Untested

- [ ] **#463 — The `serde_json::to_string` failure fallback in `Display` impl has no test**
  - File: `src/errors.rs` (lines ~51–59)
  - Source commands: `test-gaps`

### Testing — `ActixJson` Catch-All Error Branch Untested

- [ ] **#464 — The `_ =>` branch for generic `JsonPayloadError` (overflow, EOF) returns 400 but is untargeted by tests**
  - File: `src/errors.rs` (lines ~200–205)
  - Source commands: `test-gaps`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 437 tests pass: 195 backend unit (173 lib + 22 healthcheck), 103 API integration, 98 DB integration, 41 WASM. Total: **437 tests, 0 failures**.
- Backend unit test breakdown: config: 6, db/migrate: 34, errors: 16, from_row: 10, handlers/mod: 12, middleware/auth+openapi: 34, models: 16, routes: 19, server: 17, validate: 9, healthcheck: 22 = **195 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. `rsa` 0.10.0 remains at rc.16. Re-evaluate periodically.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- CONNECT Design System: `git pull` reports "Already up to date" — no migration needed.
- Frontend was refactored from monolithic `app.rs` (600+ lines) into modular architecture: `api.rs` (377 lines), `pages/` (10 files, ~2,800 lines), `components/` (7 files, ~680 lines). `app.rs` is now 164 lines (routing shell only).
- Frontend consumes 22 of 37 API endpoints.
- RBAC enforcement: no violations found. All 30 handlers enforce correct guards per CLAUDE.md policy.
- OpenAPI spec has 41 operations; annotation gaps tracked (#326, #384, #385, #412, #413, #414, #446).
- All SQL queries use parameterized prepared statements — zero injection risk.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 2 critical (#132 blocked, #397), 6 important, 34 minor, 90 informational. **Total: 132 open items**.
- 68 new findings in this assessment: #397–#464. 0 regressions found.
- 296 resolved items in `.claude/resolved-findings.md`.
- Highest finding number: #464.
