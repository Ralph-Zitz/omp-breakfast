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

*No open important items.*

## Minor Items

*No open minor items.*

## Informational Items

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

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 421 tests pass: 193 backend unit (171 lib + 22 healthcheck), 90 API integration, 98 DB integration, 41 WASM. Total: **422 tests, 0 failures**.
- Backend unit test breakdown: config: 6, db/migrate: 34, errors: 16, from_row: 10, handlers/mod: 12, middleware/auth+openapi: 32, models: 16, routes: 19, server: 17, validate: 9, healthcheck: 22 = **193 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. `rsa` 0.10.0 remains at rc.16. Re-evaluate periodically.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- CONNECT Design System: `git pull` reports "Already up to date" — no migration needed.
- Frontend was refactored from monolithic `app.rs` (600+ lines) into modular architecture: `api.rs` (377 lines), `pages/` (10 files, ~2,800 lines), `components/` (7 files, ~680 lines). `app.rs` is now 164 lines (routing shell only).
- Frontend consumes 22 of 37 API endpoints (up from 4 at last assessment).
- RBAC enforcement: no violations found. All 30 handlers enforce correct guards per CLAUDE.md policy.
- OpenAPI spec has 41 operations; remaining annotation inaccuracies tracked (#287, #326, #384, #385).
- All SQL queries use parameterized prepared statements — zero injection risk.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 1 critical (#132 blocked), 0 important, 0 minor, 93 informational. **Total: 94 open items**.
- 36 new findings in this assessment: #361–#396. 1 regression found (#361, regresses resolved #1). 1 item archived: #301 (backend fix complete).
- 255 resolved items in `.claude/resolved-findings.md`.
- Highest finding number: #396.
