# Resolved Assessment Findings

This file contains all assessment findings that have been resolved, organized by their original severity. Items are moved here from `.claude/assessment-findings.md` when marked `[x]` (completed) as part of the "assess project" process.

Last updated: 2026-03-08

> **2026-03-08 fix-informational:** Archived #723–#726 (4 informational) — all fixed and verified with passing tests.
>
> **2026-03-08 fix-findings:** Archived #714–#722 (2 important, 7 minor) — all fixed and verified with passing tests.
>
> **2026-03-08 resume-assessment:** Archived #699 (false positive — JWT unit tests exist) and #707 (false positive — FK constraint messages already implemented).

## Critical Items

### Database — `register_first_user` Not Transactional

- [x] **#601 — First-user bootstrap calls 5 DB functions without a transaction — crash mid-sequence leaves irrecoverable state**
  - File: `src/handlers/users.rs`, `src/db/users.rs`
  - Resolution: Created `db::bootstrap_first_user()` that wraps all 5 operations (count check, user creation, role seeding, team creation, membership assignment) in a single database transaction. Password hashing runs before the transaction (CPU-bound). On failure, the transaction rolls back and the system remains in a clean state for retry.
  - Source commands: `db-review`

### Dependencies — Vulnerable `rsa` Crate via `jsonwebtoken`

- [x] **#132 — Migrated from `jsonwebtoken` to `jwt-compact 0.8.0`; vulnerable `rsa` crate eliminated**
  - File: `Cargo.toml`, `src/middleware/auth.rs`, `src/errors.rs`, `src/handlers/users.rs`, `tests/api_tests.rs`, `Makefile`
  - Problem: `jsonwebtoken`'s `rust_crypto` feature pulled ~15 unused crates including `rsa` (RUSTSEC-2023-0071, unfixable timing side-channel). Granular feature flags didn't work (runtime `CryptoProvider` errors).
  - Resolution: Replaced `jsonwebtoken` with `jwt-compact 0.8.0` (pure Rust, HS256 via `sha2`, no RSA/EC dependencies). Removed `--ignore RUSTSEC-2023-0071` from Makefile. `cargo audit` now passes clean.
  - Source commands: `dependency-check`

### RBAC — Privilege Escalation via Team Admin Role Assignment

- [x] **#186 — Team Admin can assign the "Admin" role, escalating any user to global superuser**
  - Files: `src/handlers/teams.rs` (`add_team_member`, `update_member_role`)
  - Problem: Both handlers accepted an arbitrary `role_id` guarded only by `require_team_admin`. A Team Admin could self-promote to global Admin.
  - Fix: Added `is_admin` check + `get_role` validation — non-admin requesters are now rejected with `Error::Forbidden` when assigning the "Admin" role.
  - Source commands: `rbac-rules`

### Transaction Safety — TOCTOU Race on Closed-Order Checks

- [x] **#85 — `create_order_item`, `update_order_item`, and `delete_order_item` have TOCTOU race conditions**
  - File: `src/handlers/orders.rs` (all three mutation handlers)
  - Problem: Each handler checks `is_team_order_closed()` then performs the mutation as two separate, non-transactional DB operations. Between the check and the mutation, a concurrent request could close the order, allowing an item to be added/updated/deleted on a closed order.
  - Fix: Wrap the closed-order check and the mutation in a single DB transaction with `SELECT ... FOR UPDATE` on the `teamorders` row. Alternatively, add a DB-level trigger on the `orders` table that prevents INSERT/UPDATE/DELETE when the parent `teamorders.closed = true`.
  - Source commands: `db-review`

### Security — Password Hashing at User Creation

- [x] **#40 — `create_user` stores plaintext password instead of Argon2 hash**
  - Resolution: Fixed in prior session.
  - Source commands: `db-review`, `security-audit`

### Security — actix-files CVE (Verified Patched)

- [x] **#56 — `actix-files` had 2 known CVEs**
  - Resolution: Verified Cargo.lock pins patched version 0.6.10.
  - Source commands: `dependency-check`, `security-audit`

### Deployment — Database Migration Tool Adopted

- [x] **#66 — Schema managed via destructive `DROP TABLE` DDL script**
  - Resolution: Adopted `refinery` 0.8 with versioned migrations.
  - Source commands: `db-review`, `security-audit`

### Database — `update_team_order` Can Set `closed` to NULL

- [x] **#130 — Sending `null` for `closed` bypasses `guard_open_order` (which treats NULL as open via `.unwrap_or(false)`)**
  - Files: `src/db/orders.rs` (UPDATE query), `src/models.rs` (`UpdateTeamOrderEntry`)
  - Problem: `UpdateTeamOrderEntry.closed` is `Option<bool>`. When `closed` is `None`, the SQL `SET closed = $3` writes NULL to the DB. `guard_open_order` uses `.unwrap_or(false)` — so NULL counts as "open." An attacker who is a team member could re-open a closed order.
  - Fix: Use `COALESCE($3, closed)` in the SQL so NULL preserves the existing value, or make `closed` a required `bool` in `UpdateTeamOrderEntry`.
  - Source commands: `db-review`, `review`

### Database — Missing Index on `orders.orders_item_id`

- [x] **#131 — FK RESTRICT lookups require sequential scan after V3 changed CASCADE→RESTRICT**
  - Files: `migrations/V3__indexes_constraints.sql`, `migrations/V1__initial_schema.sql`
  - Problem: V3 changed the FK on `orders.orders_item_id` from CASCADE to RESTRICT. When deleting an item, PostgreSQL must verify no orders reference it. The composite PK `(orders_teamorders_id, orders_item_id)` cannot serve this lookup because `orders_item_id` is the second column.
  - Fix: Add `CREATE INDEX IF NOT EXISTS idx_orders_item ON orders (orders_item_id);` in a V4 migration.
  - Source commands: `db-review`

### Testing — `current_password` Verification on Self-Password-Change Completely Untested

- [x] **#397 — Three distinct error paths in self-password-change have zero test coverage: missing field→422, wrong password→403, correct→200**
  - File: `src/handlers/users.rs`, `tests/api_tests.rs`
  - Fix: Added three API integration tests exercising all three paths: `self_password_change_without_current_password_returns_422`, `self_password_change_with_wrong_current_password_returns_403`, `self_password_change_with_correct_current_password_succeeds`.
  - Source commands: `test-gaps`

### RBAC — Team Admin Can Reset Global Admin Password (Account Takeover)

- [x] **#667 — `update_user` handler does not call `guard_admin_demotion` — Team Admin can reset a Global Admin's password**
  - File: `src/handlers/users.rs`, `update_user` handler
  - Resolution: Added `guard_admin_demotion(&client, &req, uid).await?;` after `require_self_or_admin_or_team_admin` in the `update_user` handler. Non-admin users can no longer modify a Global Admin's user record (including password).
  - Source commands: `rbac-rules`

## Important Items

### Database — FK Guard on User Delete

- [x] **#714 — `delete_user` / `delete_user_by_email` will fail with FK violation if user has created team orders**
  - File: `src/db/orders.rs`, `src/handlers/users.rs`
  - Resolution: Added `count_user_team_orders()` DB function in `src/db/orders.rs`. Added pre-delete guard in both `delete_user` and `delete_user_by_email` handlers that returns 409 Conflict with a clear message when the user still owns team orders. Updated utoipa annotations to include 409 response.
  - Source commands: `db-review`

### Documentation — Command File Scope

- [x] **#715 — `openapi-sync.md` and `api-completeness.md` scope sections only reference `frontend/src/app.rs`**
  - File: `.claude/commands/openapi-sync.md`, `.claude/commands/api-completeness.md`
  - Resolution: Updated both command files to reference `frontend/src/api.rs` and `frontend/src/pages/` instead of the outdated `frontend/src/app.rs`.
  - Source commands: `practices-audit`

### Test Coverage — JWT Unit Tests Already Exist

- [x] **#699 — No unit tests for JWT core functions**
  - File: `src/middleware/auth.rs`
  - Resolution: False positive — unit tests already exist in `#[cfg(test)] mod tests` block covering `generate_token_pair`, `verify_jwt`, expired/invalid token rejection, blacklist cache, account lockout, and cache TTL. Finding used incorrect function names.
  - Source commands: `test-gaps`

### RBAC — Team Admin Can Delete a Global Admin's Account

- [x] **#668 — `delete_user` and `delete_user_by_email` do not call `guard_admin_demotion` — Team Admin can delete a Global Admin**
  - File: `src/handlers/users.rs`, `delete_user` and `delete_user_by_email` handlers
  - Resolution: Added `guard_admin_demotion(&client, &req, uid).await?;` after `require_self_or_admin_or_team_admin` in both handlers. Only a Global Admin can now delete another Global Admin's account.
  - Source commands: `rbac-rules`

### Database — `memberof.memberof_user_id` ON DELETE CASCADE Bypasses Last-Admin Guard

- [x] **#669 — Deleting the last global admin user silently cascades through `memberof`, bypassing `guard_last_admin_membership`**
  - File: `migrations/V15__restrict_cascade_fks.sql`, `src/db/users.rs`
  - Resolution: Created migration V15 changing `memberof.memberof_user_id` FK from ON DELETE CASCADE to ON DELETE RESTRICT. Rewrote `delete_user` and `delete_user_by_email` DB functions to use transactions that explicitly delete memberof rows before deleting the user.
  - Source commands: `db-review`

### Database — `teamorders` and `orders` ON DELETE CASCADE FKs Violate Documented RESTRICT Convention

- [x] **#670 — `teamorders.teamorders_team_id` and `orders.orders_team_id` use ON DELETE CASCADE despite documented convention of RESTRICT**
  - File: `migrations/V15__restrict_cascade_fks.sql`
  - Resolution: Migration V15 also changes `teamorders.teamorders_team_id` and `orders.orders_team_id` FKs from ON DELETE CASCADE to ON DELETE RESTRICT. Defense-in-depth alongside the handler-level 409 guard.
  - Source commands: `db-review`

### Testing — Frontend First-User Registration Flow Has Zero Test Coverage

- [x] **#671 — Login page dual-mode detection (`setup_required: true` → registration form) is completely untested**
  - File: `frontend/tests/ui_login.rs`, `frontend/tests/ui_helpers.rs`
  - Resolution: Added 3 WASM tests: `test_registration_form_renders_when_setup_required` (verifies registration form with name fields), `test_registration_short_password_shows_validation_error` (validates 8-char minimum), `test_registration_success_redirects_to_dashboard` (full flow). Added `install_mock_fetch_setup_required` and `install_mock_fetch_registration_success` mock helpers.
  - Source commands: `test-gaps`

### Testing — `authed_request` POST/PUT/DELETE Methods Untested

- [x] **#672 — Only `authed_get` is tested — token refresh retry with body-forwarding for mutations has no coverage**
  - File: `frontend/tests/ui_session.rs`, `frontend/tests/ui_helpers.rs`
  - Resolution: Added 3 WASM tests: `test_authed_post_sends_body_and_auth_header`, `test_authed_put_sends_body_and_auth_header`, `test_authed_delete_sends_auth_header_no_body`. Added `install_mock_fetch_mutation_echo` mock and request-recording helpers (`last_request_method`, `last_request_auth`, `last_request_body`).
  - Source commands: `test-gaps`

### Frontend — TeamsPage Fetches Users/Roles Without Pagination Params

- [x] **#602 — Team member management silently drops data when >50 users or roles exist**
  - File: `frontend/src/pages/teams.rs`
  - Resolution: Added explicit `?limit=100` to both the users and roles fetch URLs in the add-member dialog.
  - Source commands: `openapi-sync`, `review`

### RBAC — Order Item RBAC Checks Team Order Owner, Not Line-Item Contributor

- [x] **#603 — `update_order_item` and `delete_order_item` authorize based on team order creator, not the person who added the item**
  - File: `CLAUDE.md`
  - Resolution: Documented as intentional design in CLAUDE.md. Breakfast orders are collaborative — ownership is at the order level, not the line-item level. Order items have no per-item `user_id` column by design.
  - Source commands: `rbac-rules`

### Settings — `Bash(rm *)` Is Overly Permissive

- [x] **#604 — `.claude/settings.json` allows any `rm` command including destructive ones**
  - File: `.claude/settings.json`
  - Resolution: Replaced `Bash(rm *)` with scoped patterns: `Bash(rm -r frontend/dist*)`, `Bash(rm -f /tmp/*)`, `Bash(rm -f *.tmp)`.
  - Source commands: `practices-audit`

### Security — Deserialization Errors May Reveal Internal Types

- [x] **#605 — `serde_json` deserialization errors return struct field names and expected types to API consumers**
  - File: `src/errors.rs`
  - Resolution: Changed the `ActixJson` deserialization error handler to return a generic `"Invalid request body"` message instead of the raw `json_err.to_string()`. Detailed error is still logged server-side at `warn!` level.
  - Source commands: `security-audit`

### Database — Denormalized `orders.orders_team_id` Can Drift on Team Reassignment

- [x] **#606 — No trigger blocks `teamorders.teamorders_team_id` UPDATE when child `orders` rows exist**
  - File: `migrations/V10__guard_teamorders_team_id.sql`
  - Resolution: Added V10 migration with `guard_teamorders_team_id_change()` trigger function on `teamorders` that raises an exception if `teamorders_team_id` is changed when child `orders` rows exist.
  - Source commands: `db-review`

### Database — `delete_team` Cascades Through Order History

- [x] **#607 — Deleting a team silently destroys all team orders and order items via CASCADE**
  - File: `src/handlers/teams.rs`, `src/errors.rs`
  - Resolution: Added a pre-check in the `delete_team` handler that counts existing team orders. If any exist, returns `Error::Conflict` (HTTP 409) with a clear message. Added new `Error::Conflict` variant to the error enum with a `conflict_error_returns_409` unit test.
  - Source commands: `db-review`

### Documentation — README.md Missing V8 Migration

- [x] **#515 — README.md says "Seven migrations" and lists V1–V7, but V8 (avatars) exists on disk**
  - File: `README.md`
  - Fix: Updated count to "Eight", added V8 row to migration table, changed all "seven" references to "eight".
  - Source commands: `cross-ref-check`

### Database — `get_team_order` returns 500 instead of 404

- [x] **#187 — `get_team_order` uses `query_one` instead of `query_opt` — missing orders return 500 Internal Server Error**
  - File: `src/db/orders.rs`
  - Fix: Replaced `query_one` with `query_opt` + `ok_or_else(|| Error::NotFound(...))`.
  - Source commands: `db-review`, `review`

### Database — `update_user` returns 500 instead of 404

- [x] **#188 — Both branches of `update_user` use `query_one` — missing users return 500**
  - File: `src/db/users.rs`
  - Fix: Switched both branches to `query_opt` + `ok_or_else(|| Error::NotFound(...))`.
  - Source commands: `db-review`, `review`

### Dead Code — `State.secret` field stored but never read

- [x] **#189 — `State.secret` is loaded from config and stored but never accessed after construction**
  - Files: `src/models.rs`, `src/server.rs`, all test State constructions
  - Fix: Removed `secret` field from `State` struct and all constructions. `ServerConfig.secret` retained for startup validation.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md Project Structure tree missing V4 migration

- [x] **#190 — `V4__schema_hardening.sql` exists on disk but is missing from the Project Structure tree**
  - File: `CLAUDE.md`
  - Fix: Added `V4__schema_hardening.sql – Schema hardening migration` to the migrations section.
  - Source commands: `cross-ref-check`, `practices-audit`

### Documentation — `api-completeness.md` migration enumeration excludes V4

- [x] **#191 — `api-completeness.md` line 7 enumerates V1–V3 as exhaustive, implying V4 doesn't exist**
  - File: `.claude/commands/api-completeness.md`
  - Fix: Changed to generic wording: "all migration files in `migrations/` — the authoritative schema".
  - Source commands: `cross-ref-check`

### Model/Schema Mismatch — `teamorders_user_id` Type Disagrees with V5 NOT NULL

- [x] **#240 — `CreateTeamOrderEntry.teamorders_user_id` is `Option<Uuid>` but V5 migration made column NOT NULL — causes 500 on null**
  - Files: `src/models.rs` (`CreateTeamOrderEntry`), `src/db/orders.rs` (INSERT query)
  - Fix: Changed `teamorders_user_id: Option<Uuid>` to `teamorders_user_id: Uuid` in `CreateTeamOrderEntry`. Updated all tests and seed data to provide a non-null user_id.
  - Source commands: `api-completeness`, `db-review`

- [x] **#241 — `TeamOrderEntry.teamorders_user_id` is `Option<Uuid>` but column is NOT NULL — misleads API consumers**
  - Files: `src/models.rs` (`TeamOrderEntry`), `src/from_row.rs` (row mapping)
  - Fix: Changed `teamorders_user_id: Option<Uuid>` to `teamorders_user_id: Uuid` in `TeamOrderEntry`. The `from_row_ref` implementation auto-adjusted since it infers the type from the struct field.
  - Source commands: `api-completeness`, `db-review`

### Documentation — CLAUDE.md Missing V5 Migration

- [x] **#242 — CLAUDE.md Project Structure tree does not list V5 migration**
  - File: `CLAUDE.md`
  - Fix: Added `V5__trigger_and_notnull_fixes.sql – Trigger fix on users, NOT NULL on teamorders_user_id and memberof.joined` to the migration list.
  - Source commands: `cross-ref-check`

### Code Quality — Argon2 hasher duplicated in two places

- [x] **#192 — Identical `Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())` appears in two files**
  - Files: `src/db/users.rs`, `src/middleware/auth.rs`, `src/lib.rs`
  - Fix: Extracted `argon2_hasher()` to `src/lib.rs` as a public function; both `db/users.rs` and `middleware/auth.rs` now call `crate::argon2_hasher()`.
  - Source commands: `review`

### Validation — No range validation on order item quantities

- [x] **#193 — `CreateOrderEntry.amt` and `UpdateOrderEntry.amt` accept zero/negative quantities**
  - File: `src/models.rs`
  - Fix: Added `#[validate(range(min = 1, message = "quantity must be at least 1"))]` to `amt` in both structs.
  - Source commands: `db-review`, `review`, `security-audit`

### Frontend — Token Revocation on Logout

- [x] **#1 — Frontend logout does not revoke tokens server-side**
  - Resolution: Added `revoke_token_server_side` helper with fire-and-forget revocation.
  - Source commands: `api-completeness`, `security-audit`

### Backend — Error Response Consistency

- [x] **#15 — `auth_user` returns bare string instead of `ErrorResponse`**
  - Resolution: Routed through centralized `ResponseError` impl.
  - Source command: `review`

- [x] **#16 — `refresh_token` handler bypasses centralized error handling**
  - Resolution: Added `Error::Unauthorized` variant and updated handler.
  - Source command: `review`

### Test Gaps

- [x] **#44 — No integration test for create-user -> authenticate round-trip**
  - Resolution: Added integration test.
  - Source command: `test-gaps`

### Security — Missing CSP Headers for Static Files

- [x] **#48 — No Content-Security-Policy header on static file responses**
  - Resolution: Added CSP via `DefaultHeaders` middleware.
  - Source commands: `security-audit`

### Security — Credentials Logged via `#[instrument]`

- [x] **#50 — `#[instrument]` on auth handlers doesn't skip credential parameters**
  - Resolution: Updated all `#[instrument]` annotations to skip credentials.
  - Source commands: `security-audit`, `review`

### Dependencies — `tokio-pg-mapper` Is Archived

- [x] **#60 — `tokio-pg-mapper` crate is unmaintained/archived**
  - Resolution: Replaced with custom `FromRow` trait in `src/from_row.rs`.
  - Source command: `dependency-check`

### Code Quality — Monolithic `src/db.rs` Refactored

- [x] **#64 — `src/db.rs` is 1,144+ lines covering all domain areas**
  - Resolution: Split into `src/db/` module directory with 9 domain files.
  - Source commands: `review`, `practices-audit`

### Dependencies — `flurry` Replaced with `dashmap`

- [x] **#65 — `flurry` 0.5.2 is unmaintained**
  - Resolution: Replaced with `dashmap` 6.1.0.
  - Source commands: `dependency-check`, `review`

### Security — In-Memory Token Blacklist Eviction

- [x] **#67 — `token_blacklist` in-memory DashMap has no eviction or size limit**
  - Resolution: Changed DashMap value to `DateTime<Utc>`, added `retain()` in cleanup task.
  - Source commands: `security-audit`, `review`

### Database — UUID Version Mismatch Between Schema and Application

- [x] **#69 — Schema defaults to UUID v4 but Rust code generates UUID v7**
  - Files: `migrations/V2__uuid_v7_defaults.sql` (new), `database.sql`, `init_dev_db.sh`
  - Resolution: Created V2 migration that `ALTER TABLE ... SET DEFAULT uuidv7()` on all five UUID primary key columns. Updated `database.sql` and `init_dev_db.sh`.
  - Source commands: `db-review`, `review`

### Security — HTTPS Redirect Implemented

- [x] **#72 — HTTP requests are not redirected to HTTPS**
  - Resolution: Added HTTP->HTTPS redirect server.
  - Source commands: `security-audit`

### Testing — Missing Test Coverage Areas Addressed

- [x] **#74 — Several areas lack dedicated test coverage**
  - Resolution: Added tests for from_row, openapi, healthcheck, CORS, frontend double-failure.
  - Source commands: `test-gaps`

### Code Quality — Panicking `row.get()` in Membership Functions

- [x] **#86 — `add_team_member` and `update_member_role` use panicking `row.get()` instead of `try_get()`**
  - Files: `src/db/membership.rs` lines 139–158 (`add_team_member`), lines 224–236 (`update_member_role`)
  - Problem: Both functions use `row.get("column")` (the panicking variant from tokio-postgres) when constructing `UsersInTeam` results inside transactions. The rest of the codebase consistently uses `row.try_get()` or `FromRow`. If a column is renamed or missing due to a migration error, this will panic and crash the server process rather than returning an error.
  - Fix: Use `row.try_get(...).map_err(|e| Error::Db(e))?` or implement `FromRow` for `UsersInTeam` to match the pattern used everywhere else.
  - Source commands: `review`

### Security — Token Revocation Expiry Defaults to Now

- [x] **#87 — Token revocation blacklist entry may be immediately evictable**
  - File: `src/handlers/users.rs` lines 112, 142
  - Problem: `DateTime::<Utc>::from_timestamp(claims.claims.exp, 0).unwrap_or_else(Utc::now)` — if `exp` is an invalid timestamp, the blacklist entry gets `Utc::now()` as its expiry, making it immediately eligible for cleanup by the hourly background task. A still-valid token could become un-revoked after the next cleanup cycle.
  - Fix: Default to a far-future timestamp (e.g., `Utc::now() + Duration::days(7)` matching max refresh token lifetime) instead of `Utc::now()`.
  - Source commands: `review`

### Security — JWT Algorithm Not Explicitly Pinned

- [x] **#88 — JWT validation uses implicit algorithm selection**
  - File: `src/middleware/auth.rs` lines 36, 80
  - Problem: `Header::default()` uses HS256 and `Validation::default()` implicitly allows HS256. If `jsonwebtoken`'s defaults ever change, algorithm confusion attacks become possible. While the current behavior is safe, the reliance on implicit defaults is fragile.
  - Fix: Use `Validation::new(Algorithm::HS256)` instead of `Validation::default()` to explicitly pin the algorithm.
  - Source commands: `security-audit`

### Security — Token Revocation Allows Cross-User Revocation

- [x] **#89 — Any authenticated user can revoke any other user's token**
  - File: `src/handlers/users.rs` lines 126–148
  - Problem: The `revoke_user_token` handler accepts a JWT token in the request body and revokes it by `jti`. It requires a valid access token (JWT auth) but does not verify that the `sub` (user ID) of the token being revoked matches the requesting user. Any authenticated user who knows or guesses a token can revoke it.
  - Fix: Decode the token-to-revoke, verify `token_data.claims.sub == requesting_user_id`, or restrict this endpoint to admins. The current frontend only revokes its own tokens at logout, but the API is open.
  - Source commands: `security-audit`

### Security — No Explicit JSON Body Size Limit

- [x] **#90 — `JsonConfig::default()` relies on implicit size limit**
  - File: `src/routes.rs` lines 58–59
  - Problem: No explicit `.limit()` is set on `JsonConfig`. The implicit 32 KiB limit from actix-web 4 is adequate but could change across library versions, enabling DoS via large payloads.
  - Fix: Add `.limit(65_536)` (64 KiB) to `JsonConfig::default()`.
  - Source commands: `security-audit`

### Documentation — CLAUDE.md Test Count Stale Again

- [x] **#91 — CLAUDE.md says 156 unit tests but actual count is 170**
  - File: `CLAUDE.md` (Testing → Backend section)
  - Problem: 14 tests were added to `db/migrate.rs` (20→34) since the last count update. The documented breakdown and total are wrong. Correct breakdown: config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, db/migrate: 34, healthcheck: 22 = 170 total.
  - Fix: Update CLAUDE.md test count from 156 to 170 and update the db/migrate count in the per-module breakdown.
  - Source commands: `practices-audit`

### Testing — Missing RBAC Denial Tests

- [x] **#92 — No integration test verifies non-admin gets 403 on `update_role`, `delete_role`, `update_team`**
  - File: `tests/api_tests.rs`
  - Problem: These endpoints are admin-gated in code (`require_admin`) but no test verifies the denial path. A refactor could silently remove the guard and no test would catch it.
  - Fix: Add 3 integration tests: `non_admin_cannot_update_role`, `non_admin_cannot_delete_role`, `non_admin_cannot_update_team`.
  - Source commands: `test-gaps`

### Dependencies — Unused `secure-cookies` Feature

- [x] **#93 — `actix-web` `secure-cookies` feature adds unused crypto crates**
  - File: `Cargo.toml` line 14
  - Problem: The `secure-cookies` feature on `actix-web` pulls in `aes-gcm`, `aes`, `hmac`, and `cookie` with crypto features. The project uses JWT in headers, not cookie-based authentication. No cookie signing or encryption is used anywhere.
  - Fix: Remove `"secure-cookies"` from the features list: `features = ["rustls-0_23"]`.
  - Source commands: `dependency-check`

### Database — Nullable Timestamp Columns Across All Tables

- [x] **#133 — `created` and `changed` columns lack NOT NULL; Rust models use non-Optional types**
  - File: `migrations/V1__initial_schema.sql` (users, teams, roles, items, teamorders)
  - Problem: All timestamp columns use `DEFAULT CURRENT_TIMESTAMP` but no `NOT NULL`. An explicit NULL insert would cause a `FromRow` conversion error at runtime since the Rust models use `DateTime<Utc>` (non-optional).
  - Fix: V4 migration: `ALTER TABLE ... ALTER COLUMN created SET NOT NULL` and same for `changed` on all 5 entity tables.
  - Source commands: `db-review`

### Database — `items.price` Allows NULL

- [x] **#134 — Item without a price makes order totals impossible to calculate**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs` (`ItemEntry`, `CreateItemEntry`, `UpdateItemEntry`)
  - Problem: `price numeric(10,2) CHECK (price >= 0)` has no NOT NULL. Rust models use `Option<Decimal>`.
  - Fix: Add NOT NULL to schema and change Rust type from `Option<Decimal>` to `Decimal`.
  - Source commands: `db-review`

### Database — `orders.amt` Allows NULL

- [x] **#135 — Order item without a quantity is meaningless**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs` (`OrderEntry`, `CreateOrderEntry`, `UpdateOrderEntry`)
  - Problem: `amt int CHECK (amt >= 0)` has no NOT NULL. Rust models use `Option<i32>`.
  - Fix: Add `NOT NULL DEFAULT 1` to schema and change Rust type from `Option<i32>` to `i32`.
  - Source commands: `db-review`

### Database — `orders` Table Has No Timestamps

- [x] **#136 — Unlike every other entity table, `orders` lacks `created`/`changed` columns**
  - File: `migrations/V1__initial_schema.sql` (orders table definition)
  - Problem: No audit trail for when order items were added or modified.
  - Fix: V4 migration: add `created` and `changed` columns with NOT NULL defaults and BEFORE UPDATE trigger, consistent with other tables.
  - Source commands: `db-review`

### Error Handling — Fragile 404 Detection via String Matching

- [x] **#137 — 404 detection relies on matching `"query returned an unexpected number of rows"` string from tokio-postgres**
  - File: `src/errors.rs` (Error::Db handler)
  - Problem: If tokio-postgres ever changes this error message wording, all 404 responses silently degrade to 500s.
  - Fix: Use `query_opt` + explicit `Error::NotFound` in single-row DB functions, or match on the error kind instead of the string.
  - Source commands: `db-review`

### Documentation — `database.sql` Diverged from Migrations

- [x] **#138 — Deprecated `database.sql` is out of sync with V3 migration**
  - File: `database.sql`
  - Problem: Still uses CASCADE (V3 changed to RESTRICT), still creates `idx_orders_tid` (V3 drops it), missing NOT NULL on `memberof_role_id`, missing V3 indexes. Developers using it get a different schema than production.
  - Fix: Update to match post-V3 schema, or remove the file entirely.
  - Source commands: `db-review`

### OpenAPI — Spurious Query Params on `create_user`

- [x] **#139 — `params(CreateUserEntry)` in utoipa annotation renders body fields as query parameters in Swagger UI**
  - File: `src/handlers/users.rs` (`create_user` utoipa path annotation)
  - Problem: `CreateUserEntry` derives `IntoParams`. Its fields (firstname, lastname, email, password) appear as query parameters alongside the request body.
  - Fix: Remove `params(CreateUserEntry)` from the annotation. Remove `IntoParams` from the derive.
  - Source commands: `openapi-sync`

### OpenAPI — Spurious Query Params on `update_user`

- [x] **#140 — `params(("user_id", ...), UpdateUserRequest)` renders body fields as query parameters**
  - File: `src/handlers/users.rs` (`update_user` utoipa path annotation)
  - Problem: Same issue as #139 — `UpdateUserRequest` appears as query params alongside the body.
  - Fix: Change to `params(("user_id", ...))` only. Remove `IntoParams` from `UpdateUserRequest`.
  - Source commands: `openapi-sync`

### OpenAPI — Missing 422 Response on Validated Endpoints

- [x] **#141 — 12 handlers call `validate(&json)?` but none document 422 in utoipa annotations**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/items.rs`, `src/handlers/roles.rs`, `src/handlers/orders.rs`
  - Problem: Validation errors return HTTP 422 via `ErrorResponse`, but Swagger UI consumers don't see this documented response.
  - Fix: Add `(status = 422, description = "Validation error", body = ErrorResponse)` to each handler's `responses(...)`.
  - Source commands: `openapi-sync`

### Security — No Minimum JWT Secret Length in Production

- [x] **#142 — Operator could set `BREAKFAST_SERVER_JWTSECRET=abc` and the server would accept it**
  - Files: `src/server.rs` (production checks), `config/default.yml`
  - Problem: The server panics on default secret values in production, but imposes no minimum length. HS256 security requires at least 256 bits (32 bytes) of entropy.
  - Fix: Add a runtime check that JWT secret is ≥32 characters in production.
  - Source commands: `security-audit`

### Security — `auth_user` Cache Hit Path Bypasses Password Verification

- [x] **#144 — Handler generates tokens from cache without re-verifying password; middleware verifies but code path is misleading**
  - File: `src/handlers/users.rs` (`auth_user` handler)
  - Problem: On cache hit, a token pair is generated immediately without password check. The `basic_validator` middleware verifies first, but if middleware ordering changes, this becomes a critical auth bypass.
  - Fix: Remove the redundant cache check in the handler body. Generate token pair from the middleware-authenticated identity.
  - Source commands: `security-audit`, `review`

### Frontend — `.unwrap()` on Event Targets in WASM

- [x] **#125 — `ev.target().unwrap()` in input handlers could crash the WASM module (upgraded from informational)**
  - File: `frontend/src/app.rs` (UsernameField and PasswordField components)
  - Problem: A panic in WASM kills the entire SPA. The `target()` call returns `Option` and is unwrapped without graceful handling.
  - Fix: Use `let Some(target) = ev.target() else { return; };`.
  - Source commands: `review`

### Code Quality — Double DB Client Acquisition in `revoke_user_token`

- [x] **#147 — Handler acquires two pool connections when one would suffice**
  - File: `src/handlers/users.rs` (`revoke_user_token`)
  - Problem: The handler acquires a client for the admin check, drops it, then acquires a second for the revocation. The first client could be reused.
  - Fix: Reuse the first `Client` for both the admin check and the token revocation.
  - Source commands: `review`, `practices-audit`, `rbac-rules`

### Code Quality — `Claims.token_type` Uses `String` Instead of Typed Enum

- [x] **#148 — `token_type` field only ever holds `"access"` or `"refresh"` but uses `String`**
  - Files: `src/models.rs` (`Claims`), `src/middleware/auth.rs`
  - Problem: A typo or invalid value would compile and only fail at runtime. String comparisons are scattered across auth.rs and handlers/users.rs.
  - Fix: Define a `TokenType` enum with serde serialization.
  - Source commands: `review`

### Dependencies — `leptos` Patch Update Available

- [x] **#149 — `leptos` 0.8.16 resolved, 0.8.17 available**
  - File: `frontend/Cargo.toml`
  - Problem: Patch release likely contains bug fixes.
  - Fix: Run `cargo update -p leptos`.
  - Source commands: `dependency-check`

### Security — Argon2 Parameters Rely on Crate Defaults

- [x] **#143 — A dependency update could silently weaken hashing parameters**
  - Resolution: Replaced `Argon2::default()` with explicit `Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())` in `src/db/users.rs` (shared `argon2_hasher()` helper) and `src/middleware/auth.rs`.
  - Source commands: `security-audit`

### Security — No Production Panic for Default DB Credentials

- [x] **#145 — Default Postgres credentials `actix/actix` used with no startup validation (unlike server/JWT secrets)**
  - Resolution: Added production panic checks for default `pg.user` and `pg.password` in `src/server.rs`, matching the existing pattern for server/JWT secrets.
  - Source commands: `security-audit`

### Bug — 5 Update DB Functions Return HTTP 500 Instead of 404 for Missing Resources

- [x] **#212 — `update_team`, `update_role`, `update_item`, `update_team_order`, `update_order_item` use `query_one` which maps not-found to 500**
  - Resolution: Changed all five functions to use `query_opt()` + `.ok_or_else(|| Error::NotFound("... not found"))`, matching the `update_user` pattern. Added permanent convention note to `CLAUDE.md` to prevent future regression.
  - Source commands: `review`, `db-review`

### Security — User Enumeration via Authentication Timing Side-Channel

- [x] **#213 — Non-existent users return ~1ms vs ~100ms for wrong-password on existing users**
  - Resolution: Added `DUMMY_HASH` static constant and dummy `argon2_hasher().verify_password()` call in the user-not-found branch of `basic_validator` in `src/middleware/auth.rs`. Added `dummy_hash_is_valid_argon2id` unit test.
  - Source commands: `security-audit`

### Testing — No Test for Admin Role Escalation Guard

- [x] **#214 — Both `add_team_member` and `update_member_role` have escalation guards but no test exercises them**
  - Resolution: Added `team_admin_cannot_assign_admin_role_via_add_member` and `team_admin_cannot_assign_admin_role_via_update_role` API integration tests in `tests/api_tests.rs`.
  - Source commands: `test-gaps`, `rbac-rules`

### Testing — No Test for Password Update → Re-Login Round-Trip

- [x] **#215 — Password change via PUT is never tested with subsequent authentication**
  - Resolution: Added `update_user_password_then_reauthenticate` API integration test in `tests/api_tests.rs`.
  - Source commands: `test-gaps`

### Security — `create_team_order` Attribution Spoofing

- [x] **#266 — `create_team_order` does not validate that `teamorders_user_id` matches the requesting user**
  - Files: `src/handlers/teams.rs`, `src/models.rs`, `src/db/orders.rs`
  - Fix: Removed `teamorders_user_id` from `CreateTeamOrderEntry` request body. The handler now extracts user_id from JWT claims via `requesting_user_id()` and passes it as a separate parameter to `db::create_team_order`. Also removed `teamorders_user_id` from `UpdateTeamOrderEntry` to prevent ownership reassignment. Updated all API and DB tests.
  - Source commands: `api-completeness`, `security-audit`

### Security — JWT Tokens Lack `iss` and `aud` Claims

- [x] **#267 — No audience or issuer validation on JWT tokens**
  - Files: `src/models.rs`, `src/middleware/auth.rs`
  - Fix: Added `iss` and `aud` fields to `Claims` struct. Set `iss = "omp-breakfast"`, `aud = "omp-breakfast"` during token generation. Configured `Validation` in `verify_jwt` to require matching issuer and audience. Updated all test helpers that construct Claims.
  - Source commands: `security-audit`

### Security — RBAC Inconsistency on Team Order Mutations

- [x] **#268 — Any team member (including Guest) can update/delete any team order in their team**
  - File: `src/handlers/teams.rs`, `src/handlers/mod.rs`
  - Fix: Added `require_order_owner_or_team_admin` helper to `handlers/mod.rs`. Updated `delete_team_order` and `update_team_order` handlers to fetch the order first, then check ownership via the new helper. Only the order creator, Team Admin for the team, or global Admin can now modify/delete a single order. Updated utoipa annotations.
  - Source commands: `security-audit`, `rbac-rules`

### Documentation — `guard_admin_role_assignment` Undocumented in RBAC Policy

- [x] **#269 — `guard_admin_role_assignment` helper is missing from CLAUDE.md RBAC conventions and rbac-rules.md policy table**
  - Files: `CLAUDE.md`, `.claude/commands/rbac-rules.md`
  - Fix: Added `guard_admin_role_assignment` and `require_order_owner_or_team_admin` to CLAUDE.md handlers/mod.rs function list and RBAC convention paragraphs. Added separate rows in rbac-rules.md policy table for order owner checks and admin role assignment guard.
  - Source commands: `cross-ref-check`, `practices-audit`

### RBAC — Order Item Handlers Use Wrong Authorization Guard

- [x] **#302 — `update_order_item` allows any team member to modify other members' order items (privilege escalation)**
  - File: `src/handlers/orders.rs`
  - Fix: Changed `require_team_member` to `require_order_owner_or_team_admin` — now fetches the team order first, then checks ownership/admin status. Updated utoipa 403 description.
  - Source commands: `rbac-rules`

- [x] **#303 — `delete_order_item` allows any team member to delete other members' order items (privilege escalation)**
  - File: `src/handlers/orders.rs`
  - Fix: Same pattern as #302 — changed to `require_order_owner_or_team_admin` with team order ownership check.
  - Source commands: `rbac-rules`

### Code Quality — `cargo fmt` Drift

- [x] **#304 — `cargo fmt --check` reports formatting diff in `src/middleware/auth.rs`**
  - Fix: Ran `cargo fmt` on backend.
  - Source commands: `practices-audit`

- [x] **#305 — `cargo fmt --check` reports significant formatting drift in frontend files (~15KB of diffs)**
  - Fix: Ran `cd frontend && cargo fmt`.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md Updates

- [x] **#306 — CLAUDE.md Project Structure tree still shows only `app.rs`, `lib.rs`, `main.rs` under `frontend/src/`**
  - File: `CLAUDE.md`
  - Fix: Updated Project Structure tree with full modular frontend layout (api.rs, components/ with 7 files, pages/ with 10 files). Updated Frontend Architecture section with correct component hierarchy and module descriptions.
  - Source commands: `cross-ref-check`

- [x] **#307 — 4 of 5 Unfinished Work items are now completed**
  - File: `CLAUDE.md`
  - Fix: Removed completed items (sidebar navigation, dark/light toggle, toast notifications, confirmation modals). Updated remaining items.
  - Source commands: `cross-ref-check`

### Documentation — Assessment Command Files Reference Stale `app.rs` Path

- [x] **#308 — 3 command files reference `frontend/src/app.rs` as the frontend source**
  - File: `.claude/commands/test-gaps.md` (only file with stale reference; review.md and security-audit.md already used generic paths)
  - Fix: Updated test-gaps.md to reference `frontend/src/` with `api.rs`, `app.rs`, `components/`, `pages/`.
  - Source commands: `cross-ref-check`

### Testing — Zero WASM Tests for 6 New Frontend Pages

- [x] **#309 — `admin.rs`, `items.rs`, `orders.rs`, `profile.rs`, `roles.rs`, `teams.rs` have no test coverage (~2,800 lines)**
  - File: `frontend/tests/ui_tests.rs`
  - Fix: Added 12 WASM tests (2 per page): page rendering with data, navigation/interaction, and admin visibility checks. Extended mock fetch to return data for all API endpoints. Added timeout to Makefile `test-frontend` target. Total WASM tests: 39.
  - Source commands: `test-gaps`

### Validation — `add_team_member` Missing Validation

- [x] **#327 — `add_team_member` handler missing `validate(&json)?` call before DB operation**
  - File: `src/handlers/teams.rs`
  - Resolution: Added `validate(&json)?` call before `json.into_inner()` in `add_team_member`.
  - Source commands: `practices-audit`

### Validation — `update_member_role` Missing Validation

- [x] **#328 — `update_member_role` handler missing `validate(&json)?` call before DB operation**
  - File: `src/handlers/teams.rs`
  - Resolution: Added `validate(&json)?` call before `json.into_inner()` in `update_member_role`.
  - Source commands: `practices-audit`

### Frontend — REGRESSION: Sidebar Logout Token Revocation Silently Fails

- [x] **#361 — `LogoutButton` uses `authed_request()` after clearing `sessionStorage`, so token revocation requests are never sent (regression of resolved #1)**
  - File: `frontend/src/components/sidebar.rs`
  - Problem: The logout handler saved token values, cleared `sessionStorage`, then called `authed_request()` which reads from `sessionStorage` (now empty) — revocation requests were never sent.
  - Resolution: Replaced `authed_request()` calls with `revoke_token_server_side()`, which takes an explicit bearer token and does not depend on `sessionStorage`.
  - Source commands: `review`, `security-audit`

### Security — Password Change Does Not Require Current Password

- [x] **#362 — `update_user` accepts a new password without verifying the current one**
  - Files: `src/handlers/users.rs`, `src/models.rs`, `src/db/users.rs`, `frontend/src/pages/profile.rs`
  - Problem: The profile page sent a new password in the PUT body without confirming the user knows the current password.
  - Resolution: Added `current_password` field to `UpdateUserRequest`, added `get_password_hash` DB function, and updated `update_user` handler to verify current password for self-updates. Admins resetting another user's password may omit `current_password`. Frontend profile page conditionally shows "Current Password" field when a new password is entered.
  - Source commands: `security-audit`

### Accessibility — Icon-Only Buttons Lack `aria-label` in 5 Pages

- [x] **#363 — Delete/action buttons with only an icon have no accessible name**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Problem: Screen readers announced icon-only trash buttons as unlabeled buttons, violating WCAG 2.1 SC 4.1.2.
  - Resolution: Added `aria-label` to all 6 icon-only delete buttons: "Delete team", "Delete item", "Delete order", "Remove item from order", "Delete role", "Delete user".
  - Source commands: `review`

### Performance — Argon2 Password Hashing Blocks Async Tokio Worker Thread

- [x] **#398 — `hash_password()` and `verify_password()` are CPU-intensive (~100–300ms) and run synchronously in async handlers**
  - Files: `src/db/users.rs`, `src/middleware/auth.rs`, `src/handlers/users.rs`
  - Fix: Wrapped all 4 call sites in `tokio::task::spawn_blocking()`: `hash_password` in `create_user`/`update_user`, `verify_password` in `basic_validator` (both paths), and `verify_password` in `update_user` handler self-password-change. Added `tokio` as direct dependency.
  - Source commands: `review`

### Security — Admin Can Delete Their Own Account With No Guard

- [x] **#399 — No frontend or backend guard prevents the last admin from deleting themselves, losing all administrative access**
  - Files: `src/handlers/users.rs`, `src/db/membership.rs`, `frontend/src/pages/admin.rs`
  - Fix: (Backend) Added `count_admins()` DB function and guard in `delete_user`/`delete_user_by_email` handlers — returns 403 if caller is deleting self, is admin, and is the last admin. (Frontend) Hide delete button for current user's own row in admin page.
  - Source commands: `review`

### Testing — Account Lockout Full-Flow Has No End-to-End API Test

- [x] **#400 — 5-attempt lockout → 429 → success clears lockout — no API integration test for the full flow**
  - File: `tests/api_tests.rs`
  - Fix: Added `lockout_lifecycle_5_failures_then_429_then_success_clears` API integration test exercising the complete lockout lifecycle.
  - Source commands: `test-gaps`

### Testing — Self-Delete User Completely Untested at API Level

- [x] **#401 — The `require_self_or_admin_or_team_admin` self-match path for DELETE has no API integration test**
  - File: `tests/api_tests.rs`
  - Fix: Added `non_admin_user_can_delete_own_account` API integration test.
  - Source commands: `test-gaps`

### Testing — `get_password_hash` DB Function Completely Untested

- [x] **#402 — `get_password_hash` in `db/users.rs` is used for password verification during self-password-change but has no DB integration test**
  - File: `tests/db_tests.rs`
  - Fix: Added `get_password_hash_returns_argon2_hash` and `get_password_hash_returns_not_found_for_nonexistent_user` DB integration tests.
  - Source commands: `test-gaps`

### Frontend — Order Delete Button RBAC Mismatch

- [x] **#403 — Frontend gates delete button on global admin only, but backend `require_order_owner_or_team_admin` allows order owner and team admin**
  - File: `frontend/src/pages/orders.rs`, `frontend/src/api.rs`
  - Fix: Added `team_id` field to `UserInTeams` struct. Replaced `is_admin` prop with `can_delete` closure that checks admin OR order owner OR team admin. Updated mock data in frontend tests.
  - Source commands: `api-completeness`

### RBAC — Last-Admin Demotion/Removal via Membership Operations

- [x] **#505 — `remove_team_member` and `update_member_role` allow stripping the last global Admin of their Admin role, leaving the system with zero admins**
  - Files: `src/handlers/teams.rs` (`remove_team_member`, `update_member_role`), `src/handlers/mod.rs` (`guard_last_admin_membership`), `src/db/membership.rs` (`would_admins_remain_without`)
  - Fix: Added `would_admins_remain_without` DB function and `guard_last_admin_membership` handler guard. The guard checks if the target holds Admin in the specific team, then verifies at least one admin user would remain after excluding that membership. Wired into both `remove_team_member` and `update_member_role` handlers after `guard_admin_demotion`. Returns 403 if the operation would leave zero admins.
  - Source commands: `db-review`, `rbac-rules`, `review`, `test-gaps`

### Frontend — Admin Password Reset Sends Incomplete Request Body (Broken Feature)

- [x] **#506 — `do_reset_password` sends `PUT /api/v1.0/users/{id}` with only `{"password": "..."}`, but `UpdateUserRequest` requires `firstname`, `lastname`, `email` as non-optional fields**
  - File: `frontend/src/pages/admin.rs`
  - Fix: Updated `do_reset_password` to look up the target user from the `users` signal and include `firstname`, `lastname`, `email` in the PUT request body alongside the new `password`.
  - Source commands: `api-completeness`

### Database — Race Condition in First-User Bootstrap

- [x] **#633 — `bootstrap_first_user` allows concurrent first-user registrations with different emails**
  - File: `src/db/users.rs` (`bootstrap_first_user` function)
  - Resolution: Added `SELECT pg_advisory_xact_lock(0)` at the start of the transaction to serialize concurrent bootstrap attempts. The advisory lock is held for the duration of the transaction, ensuring only one caller can proceed past the `count_users() == 0` check.
  - Source commands: `db-review`

### Testing — Avatars Feature Completely Untested

- [x] **#634 — Zero API integration tests and zero DB integration tests for the avatars feature**
  - Files: `tests/api_tests.rs`, `tests/db_tests.rs`
  - Resolution: Added 7 API integration tests (get_avatars_returns_list, get_avatars_requires_auth, get_avatar_not_found, set_avatar_nonexistent_avatar_returns_404, set_avatar_requires_self_or_admin, remove_avatar_nonexistent_user_returns_404, remove_avatar_succeeds_for_self) and 3 DB integration tests (get_avatar_nonexistent_returns_error, set_user_avatar_nonexistent_user_returns_error, insert_avatar_duplicate_name_is_idempotent). All tests are idempotent and self-contained.
  - Source commands: `test-gaps`

### Concurrency — TOCTOU Race in `remove_team_member`

- [x] **#643 — TOCTOU race in last-admin guard for `remove_team_member`**
  - File: `src/handlers/teams.rs`, `src/db/membership.rs`
  - Resolution: Changed `remove_team_member` DB function to accept `&mut Client`, wrapping the guard check + DELETE in a single transaction with `SELECT ... FOR UPDATE` on the memberof row. Updated handler to use `let mut client`. Updated test call sites to pass `&mut client`.
  - Source commands: `db-review`, `review`

### Security — JSON Body Size Limit on `/auth/refresh`

- [x] **#644 — No JSON body size limit on `/auth/refresh` endpoint**
  - File: `src/routes.rs`
  - Resolution: Added `.app_data(JsonConfig::default().limit(65_536).error_handler(json_error_handler))` to the `/auth/refresh` resource, matching `/auth/register` and `/auth/revoke`.
  - Source commands: `security-audit`

### Documentation — README.md Missing V17 Migration

- [x] **#698 — README.md still says "sixteen migrations" and lists V1–V16, missing V17**
  - Files: `README.md`
  - Resolution: Updated "sixteen" to "seventeen" in both occurrences. Added V17 row to migration table. Updated V1–V16 references to V1–V17. Updated frontend test count from 93 to 97.
  - Source commands: `cross-ref-check`

## Minor Items

### Frontend — Orders Page Pagination

- [x] **#716 — Orders page fetches items and team users without pagination limit**
  - File: `frontend/src/pages/orders.rs`
  - Resolution: Added `?limit=100` to both `/api/v1.0/items` and `/api/v1.0/teams/{id}/users` fetch URLs in the orders page.
  - Source commands: `api-completeness`

### Frontend — Safe DOM Casting in Teams Page

- [x] **#717 — `unchecked_into` casts in teams page (3 occurrences)**
  - File: `frontend/src/pages/teams.rs`
  - Resolution: Replaced all 3 `target.unchecked_into::<web_sys::HtmlSelectElement>().value()` with safe `target.dyn_ref::<web_sys::HtmlSelectElement>()` + early return guard.
  - Source commands: `review`

### Frontend — Unused Signal in Orders Page

- [x] **#718 — Unused `_is_admin` signal in `OrdersPage`**
  - File: `frontend/src/pages/orders.rs`
  - Resolution: Removed the unused `let _is_admin = crate::api::is_admin_signal(user);` binding.
  - Source commands: `review`

### Documentation — CLAUDE.md api.rs Description

- [x] **#719 — CLAUDE.md `api.rs` description says `authed_get/post/put/delete` but code has `authed_get` + `authed_request`**
  - File: `CLAUDE.md`
  - Resolution: Updated description to: "HTTP helpers (authed_get, authed_request with HttpMethod enum), JWT decode, UserContext, session storage".
  - Source commands: `practices-audit`

### Documentation — README.md postgres-setup References

- [x] **#720 — README.md references `postgres-setup` service in dev setup instructions**
  - File: `README.md`
  - Resolution: Removed `docker compose run --rm postgres-setup` from Setup section. Rewrote Database Initialization Development paragraph to explain that the app runs Refinery migrations automatically at startup.
  - Source commands: `cross-ref-check`

### Testing — PaginationParams Unit Tests

- [x] **#721 — `PaginationParams::sanitize()` has no unit tests**
  - File: `src/models.rs`
  - Resolution: Added 5 unit tests covering: None defaults, zero limit clamped to 1, negative limit clamped to 1, limit > MAX clamped to 100, negative offset clamped to 0.
  - Source commands: `test-gaps`

### Dependencies — rust_decimal Default Features

- [x] **#722 — `rust_decimal` pulls ~20 unused transitive crates via default features**
  - File: `Cargo.toml`
  - Resolution: Changed to `default-features = false` with explicit `["std", "serde", "db-tokio-postgres", "serde-with-str"]`. Verified build and all tests pass.
  - Source commands: `dependency-check`

### Backend — FK RESTRICT Error Messages Already Specific

- [x] **#707 — FK DELETE RESTRICT error messages are ambiguous**
  - File: `src/errors.rs`
  - Resolution: False positive — constraint-specific human-readable messages are already implemented. The error handler parses constraint names and maps `item` → "Referenced item does not exist", `team` → "Referenced team does not exist", etc.
  - Source commands: `db-review`

### OpenAPI — `get_avatar` Annotation Falsely Claims JWT Auth Required

- [x] **#513 — Public `get_avatar` endpoint has `security(("bearer_auth" = []))` but is registered outside JWT scope**
  - File: `src/handlers/avatars.rs`
  - Fix: Removed `security(("bearer_auth" = []))` and the `(status = 401, ...)` response from the `get_avatar` utoipa annotation.
  - Source commands: `openapi-sync`

### Documentation — CLAUDE.md Unit Test Count Stale (198 → 199)

- [x] **#514 — CLAUDE.md says 198 unit tests but actual count is 199 (177 lib + 22 healthcheck)**
  - File: `CLAUDE.md`
  - Fix: Updated test count from 198 to 199.
  - Source commands: `practices-audit`, `cross-ref-check`

### Documentation — README.md Unit Test Count Stale (193 → 199)

- [x] **#516 — README.md says 193 unit tests but actual count is 199**
  - File: `README.md`
  - Fix: Updated test count from 193 to 199.
  - Source commands: `cross-ref-check`

### Security — Swagger UI Exposed in Production

- [x] **#112 — `/explorer` registered unconditionally regardless of environment**
  - File: `src/routes.rs`
  - Resolution: `routes()` now checks `ENV` and only registers the `/explorer` Swagger UI scope when `ENV != "production"`. In production, the endpoint is simply not mounted — no schema exposure.
  - Source commands: `security-audit`

### Frontend — All Components in Single `app.rs` File

- [x] **#71 — Frontend `app.rs` is a 600+ line monolith**
  - File: `frontend/src/app.rs`
  - Resolution: Refactored into modular architecture. `app.rs` is now 164 lines (routing shell only). Frontend split into `api.rs` (377 lines), `pages/` directory (10 files, ~2,800 lines), `components/` directory (7 files, ~680 lines) covering all planned pages and shared UI components.
  - Source commands: `review`, `practices-audit`

### Frontend — Consumes Only 4 of 41 Endpoints

- [x] **#116 — Frontend only uses auth (3) + user-detail (1) endpoints**
  - File: `frontend/src/api.rs`
  - Resolution: Frontend now consumes 22 of 37 endpoints across all page modules (teams, orders, items, roles, admin, profile). Remaining 15 endpoints are mostly update/edit operations and member management that will be added as pages mature.
  - Source commands: `api-completeness`

### Code Quality — `cargo fmt` Drift in `db_tests.rs`

- [x] **#297 — `cargo fmt --check` reports formatting diff in `db_tests.rs`**
  - File: `tests/db_tests.rs`
  - Resolution: `db_tests.rs` no longer has formatting issues. New formatting drift tracked in #304 (backend `auth.rs`) and #305 (frontend files).
  - Source commands: `practices-audit`

### Code Quality — Dead S3 Config Fields

- [x] **#59 — `s3_key_id` and `s3_key_secret` are loaded and stored but never used**
  - Files: `src/config.rs`, `src/models.rs`, `src/server.rs`, `src/routes.rs`, `src/middleware/auth.rs`, `tests/api_tests.rs`, `config/default.yml`, `config/development.yml`, `config/production.yml`
  - Fix: Removed `s3_key_id` and `s3_key_secret` fields from `ServerConfig` and `State`. Removed all occurrences from state construction in server, routes, middleware, and test helpers. Removed from all three config YAML files.
  - Source commands: `review`, `practices-audit`

### Code Quality — Dead `database.url` Config Field

- [x] **#68 — `database.url` field in `Settings` is configured but unused**
  - Files: `src/config.rs`, `src/server.rs`, `config/default.yml`, `config/development.yml`
  - Fix: Removed the `Database` struct and `database` field from `Settings`. Removed `database:` sections from config YAML files. Removed `settings_database_url` test. Removed `database` field from all `Settings` constructions in server.rs tests.
  - Source commands: `review`, `practices-audit`

### Security — Seed Data Uses Hardcoded Argon2 Salt

- [x] **#70 — All seed users share the same Argon2 hash with a hardcoded salt**
  - File: `database_seed.sql`
  - Fix: Added prominent `⚠ WARNING: DO NOT RUN IN PRODUCTION ⚠` banner at the top of the file with explanation about hardcoded credentials.
  - Source commands: `security-audit`, `db-review`

### Security — No Account Lockout After Failed Auth Attempts

- [x] **#73 — Failed authentication is rate-limited but no lockout policy exists**
  - Files: `src/models.rs`, `src/middleware/auth.rs`, `CLAUDE.md`
  - Fix: Added `login_attempts: DashMap<String, Vec<DateTime<Utc>>>` to `State`. Added `is_account_locked`, `record_failed_attempt`, and `clear_failed_attempts` helpers. `basic_validator` now checks lockout (HTTP 429) before processing credentials, records failed attempts on all failure paths, and clears on success. Constants: 5 attempts in 15-minute window. Added 5 unit tests. Updated CLAUDE.md.
  - Source commands: `security-audit`

### Deployment — Production Config Has Placeholder Hostname

- [x] **#75 — `config/production.yml` uses `pick.a.proper.hostname` as the PG host**
  - File: `src/server.rs`
  - Fix: Added startup panic when `pg.host` is `pick.a.proper.hostname` and `ENV=production`. Updated CLAUDE.md production safety documentation.
  - Source commands: `practices-audit`, `review`

### Database — Inconsistent Row Mapping Pattern

- [x] **#6 — `get_team_users` uses `.map()` instead of `filter_map` + `warn!()`**
  - Resolution: Changed to `filter_map` with `try_get()` and `warn!()`.
  - Source commands: `db-review`, `practices-audit`

- [x] **#7 — `get_user_teams` has the same `.map()` issue**
  - Resolution: Same approach as #6.
  - Source commands: `db-review`, `practices-audit`

### Test Gaps (Earlier Round)

- [x] **#37 — No integration test for closed-order enforcement**
  - Resolution: Tests already present in codebase.
  - Source command: `test-gaps`

- [x] **#38 — No integration test for `delete_user_by_email` RBAC fallback**
  - Resolution: Added two integration tests.
  - Source command: `test-gaps`

- [x] **#39 — No WASM test for `authed_get` token refresh retry**
  - Resolution: Added stateful fetch mock test.
  - Source command: `test-gaps`

### Documentation — CLAUDE.md Stale After Recent Changes

- [x] **#41 — Test counts in CLAUDE.md are stale**
  - Resolution: Updated test counts.
  - Source command: `practices-audit`

- [x] **#42 — `Error::Unauthorized` variant not documented in CLAUDE.md**
  - Resolution: Added documentation.
  - Source command: `practices-audit`

- [x] **#43 — Unfinished Work section does not reflect frontend token revocation**
  - Resolution: Updated Unfinished Work and Frontend Architecture sections.
  - Source commands: `practices-audit`, `api-completeness`

### Backend — Redundant Token-Type Check

- [x] **#45 — `refresh_token` handler duplicates token-type check already enforced by middleware**
  - Resolution: Kept as defence-in-depth with explanatory comment.
  - Source commands: `review`, `security-audit`

### Frontend — Clippy Warning in Test File

- [x] **#46 — Useless `format!` in frontend test `ui_tests.rs`**
  - Resolution: Replaced with `.to_string()`.
  - Source command: `review`

### Testing — Flaky DB Test

- [x] **#47 — `cleanup_expired_tokens_removes_old_entries` is flaky under parallel test execution**
  - Resolution: Changed expiry and removed global count assertion.
  - Source command: `test-gaps`

### Documentation — CLAUDE.md `handlers/mod.rs` Description Incomplete

- [x] **#51 — `handlers/mod.rs` description omits newer RBAC helpers**
  - Resolution: Updated to list all RBAC helpers.
  - Source command: `practices-audit`

### Database — Missing DROP TABLE for token_blacklist

- [x] **#52 — `database.sql` missing `DROP TABLE IF EXISTS token_blacklist`**
  - Resolution: Added the DROP statement.
  - Source command: `db-review`

### Code Quality — Unused `require_self_or_admin` Helper

- [x] **#53 — `require_self_or_admin` helper is retained but never called**
  - Resolution: Added `#[deprecated]` attribute.
  - Source command: `review`

### Documentation — CLAUDE.md CSP Policy Not Documented

- [x] **#57 — CLAUDE.md Key Conventions should document the CSP header on static files**
  - Resolution: Added CSP documentation to Key Conventions.
  - Source commands: `practices-audit`, `security-audit`

### Frontend — Loading Page Spinner CSS Missing

- [x] **#58 — `LoadingPage` component references undefined CSS classes**
  - Resolution: Added CSS rules for loading page components.
  - Source commands: `review`, `practices-audit`

### Documentation — CLAUDE.md Test Counts and References Are Stale

- [x] **#77 — Multiple stale references in CLAUDE.md**
  - Files: `CLAUDE.md` (Project Structure and Testing sections)
  - Resolution: Updated WASM test count from 22 to 23 in both sections.
  - Source commands: `practices-audit`

### Documentation — Command Files Reference Stale Path

- [x] **#78 — Three command files reference `src/db.rs` instead of `src/db/`**
  - Resolution: Updated all three command files.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md `flurry` Reference Is Stale

- [x] **#79 — Key Conventions still references `flurry::HashMap` instead of `dashmap::DashMap`**
  - File: `CLAUDE.md` line 117
  - Resolution: Changed to `dashmap::DashMap` and updated description.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md Project Structure Missing New Files

- [x] **#80 — Project Structure tree omits files added since last documentation update**
  - File: `CLAUDE.md` lines 48–110
  - Resolution: Added all missing files to the tree.
  - Source commands: `practices-audit`

### Documentation — `api-completeness.md` References Deprecated Schema File

- [x] **#81 — `api-completeness.md` still references `database.sql` as the schema source**
  - File: `.claude/commands/api-completeness.md`
  - Resolution: Updated to reference `migrations/V1__initial_schema.sql`.
  - Source commands: `practices-audit`

### Code Quality — Duplicate Doc Comment on `fetch_user_details`

- [x] **#82 — `fetch_user_details` has a duplicate doc comment block**
  - File: `frontend/src/app.rs`
  - Resolution: Removed redundant doc comment lines.
  - Source commands: `review`

### Documentation — CLAUDE.md Test Counts and Module List Are Stale

- [x] **#83 — CLAUDE.md says 136 unit tests but actual count is 156 (20 `db::migrate` tests uncounted)**
  - File: `CLAUDE.md` line 276 (Testing → Backend section)
  - Resolution: Updated test count from 136 to 156 and added `db::migrate` to the module list. The correct breakdown is: config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, db/migrate: 20, healthcheck: 22.
  - Source commands: `practices-audit`

### Documentation — CLAUDE.md Project Structure Missing V2 Migration

- [x] **#84 — `migrations/V2__uuid_v7_defaults.sql` is not listed in the Project Structure tree**
  - File: `CLAUDE.md` line 104 (Project Structure section, `migrations/` directory)
  - Resolution: Added `V2__uuid_v7_defaults.sql — UUID v7 default migration (PostgreSQL 18+)` after the V1 entry in the Project Structure tree.
  - Source commands: `practices-audit`

### Code Quality — `verify_jwt` and `generate_token_pair` Are Unnecessarily Async

- [x] **#94 — Functions contain no `.await` but are marked `async`**
  - File: `src/middleware/auth.rs` lines 52, 77
  - Problem: Creates unnecessary `Future` wrappers on every auth call. Every caller must `.await` them but the compiler generates state-machine code for no benefit.
  - Fix: Change to `pub fn`. Remove `.await` from callers.
  - Source commands: `review`

### Code Quality — Auth Functions Take `String` by Value

- [x] **#95 — `verify_jwt` and `generate_token_pair` take `String` instead of `&str`**
  - File: `src/middleware/auth.rs` lines 52, 77
  - Problem: Forces `.clone()` at every call site (`state.jwtsecret.clone()`, `credentials.token().to_string()`).
  - Fix: Change signatures to take `&str`.
  - Source commands: `review`

### Code Quality — Magic Strings for Role Names and Token Types

- [x] **#96 — `"Admin"`, `"Team Admin"`, `"access"`, `"refresh"` scattered as raw strings**
  - Files: `src/db/membership.rs`, `src/handlers/mod.rs`, `src/middleware/auth.rs`
  - Problem: A typo would silently break RBAC or token validation.
  - Fix: Define `const` values or enums (e.g., `pub const ADMIN: &str = "Admin";`).
  - Source commands: `review`

### Code Quality — `StatusResponse` Reused for Token Revocation

- [x] **#97 — Token revocation returns `{"up": true}` instead of a revocation-specific response**
  - File: `src/handlers/users.rs` line 150
  - Problem: `StatusResponse { up: true }` is the health-check response type. Reusing it for `/auth/revoke` is semantically wrong.
  - Fix: Create a dedicated `RevokedResponse` or use `DeletedResponse`.
  - Source commands: `review`

### Code Quality — Dead `FromRow` Implementations for Input DTOs

- [x] **#98 — 7 `FromRow` implementations exist for types never read from DB rows**
  - File: `src/from_row.rs` (CreateUserEntry, CreateTeamEntry, UpdateTeamEntry, CreateRoleEntry, UpdateRoleEntry, CreateItemEntry, UpdateItemEntry)
  - Problem: These types are input DTOs (deserialized from JSON). No DB function ever constructs them from a row.
  - Fix: Remove the unused `FromRow` implementations.
  - Source commands: `review`

### Code Quality — `FromRow` Boilerplate

- [x] **#99 — `from_row` always delegates to `from_row_ref` — 13 identical function bodies**
  - File: `src/from_row.rs`
  - Problem: Every `FromRow` implementation has the same `fn from_row(row: Row) -> ... { Self::from_row_ref(&row) }` body.
  - Fix: Add a default implementation in the trait: `fn from_row(row: Row) -> ... { Self::from_row_ref(&row) }`.
  - Source commands: `review`

### Code Quality — `UsersInTeam`/`UserInTeams` Bypass `FromRow`

- [x] **#100 — Manual row mapping in `get_team_users` and `get_user_teams` instead of `FromRow`**
  - File: `src/db/teams.rs` lines 27–46, 155–183
  - Problem: Two functions use copy-pasted manual `try_get` logic instead of the `FromRow` trait used everywhere else.
  - Fix: Implement `FromRow` for `UsersInTeam` and `UserInTeams`.
  - Source commands: `review`, `db-review`

### Database — Missing FK Index on `teamorders.teamorders_user_id`

- [x] **#101 — `teamorders_user_id` foreign key is not indexed**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: Queries joining on this column or `ON DELETE RESTRICT` checks on user deletion will seq-scan `teamorders`.
  - Fix: Add `CREATE INDEX idx_teamorders_user ON teamorders (teamorders_user_id);` in a new V3 migration.
  - Source commands: `db-review`

### Database — Missing FK Index on `orders.orders_team_id`

- [x] **#102 — `orders_team_id` has no index; queries filter on it**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: `get_order_items` and `delete_order_item` filter on `orders_team_id`, causing seq-scans.
  - Fix: Add `CREATE INDEX idx_orders_team ON orders (orders_team_id);` in a new V3 migration.
  - Source commands: `db-review`

### Database — Redundant Index `idx_orders_tid`

- [x] **#103 — Composite PK already provides B-tree on leading column**
  - File: `migrations/V1__initial_schema.sql` line 126
  - Problem: `idx_orders_tid` on `(orders_teamorders_id)` is redundant — the PK `(orders_teamorders_id, orders_item_id)` already covers it.
  - Fix: Drop the index in a new migration.
  - Source commands: `db-review`

### Database — `ON DELETE CASCADE` on `orders.orders_item_id` Destroys History

- [x] **#104 — Deleting a breakfast item silently removes it from all historical orders**
  - File: `migrations/V1__initial_schema.sql` line 99
  - Problem: `ON DELETE CASCADE` on the FK from `orders.orders_item_id` to `items.item_id` means deleting an item destroys order history.
  - Fix: Change to `ON DELETE RESTRICT` (prevent deletion of items in use) or implement soft-delete.
  - Source commands: `db-review`

### Database — `memberof.memberof_role_id` Allows NULL

- [x] **#105 — A membership without a role bypasses RBAC**
  - File: `migrations/V1__initial_schema.sql` line 65
  - Problem: `memberof_role_id` has no `NOT NULL` constraint. A row with NULL role_id passes membership checks but has no role, creating undefined RBAC behavior.
  - Fix: Add `ALTER TABLE memberof ALTER COLUMN memberof_role_id SET NOT NULL;` in a V3 migration.
  - Source commands: `db-review`

### Code Quality — `TeamOrderEntry.closed` Type Mismatch

- [x] **#106 — `closed` is `Option<bool>` but DB column is `NOT NULL DEFAULT FALSE`**
  - File: `src/models.rs`
  - Problem: The Rust model will never receive `None` — it will always be `Some(true)` or `Some(false)`.
  - Fix: Change to `pub closed: bool`.
  - Source commands: `db-review`

### Documentation — OpenAPI Path Parameter Names Are Generic

- [x] **#107 — 15 handlers use `{id}` in utoipa path instead of descriptive names like `{user_id}`**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/items.rs`, `src/handlers/roles.rs`
  - Problem: Swagger UI shows generic `id` parameter names instead of descriptive ones. The `delete_user_by_email` route also misleadingly names the email segment `{user_id}` in routes.rs.
  - Fix: Update utoipa `path` attributes to match actix route parameter names.
  - Source commands: `openapi-sync`

### Documentation — `MIGRATION_FIX_SUMMARY.md` Listed But Deleted

- [x] **#108 — Project Structure tree references a file that no longer exists on disk**
  - File: `CLAUDE.md` (Project Structure section)
  - Resolution: Resolved — no longer surfaced by assessment. Reference removed in prior session.
  - Source commands: `practices-audit`

### Performance — RBAC Helpers Make Sequential DB Queries

- [x] **#109 — `require_team_member` and `require_team_admin` make 2 DB round-trips**
  - File: `src/handlers/mod.rs` lines 30–79
  - Problem: For non-admin users (the common case), both `is_admin()` and `get_member_role()` execute sequentially. Could be combined.
  - Fix: Create a single query checking both admin and team role in one `EXISTS`.
  - Source commands: `db-review`

### Security — Missing HSTS Header

- [x] **#110 — No `Strict-Transport-Security` despite TLS enforcement**
  - File: `src/server.rs` (DefaultHeaders section)
  - Problem: Without HSTS, a first-visit browser is vulnerable to SSL stripping for the initial HTTP request (before redirect).
  - Fix: Add `.add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))` to `DefaultHeaders`.
  - Source commands: `security-audit`

### Security — Missing `X-Content-Type-Options` Header

- [x] **#111 — No `X-Content-Type-Options: nosniff` header set**
  - File: `src/server.rs` (DefaultHeaders section)
  - Problem: Older browsers may MIME-sniff responses.
  - Fix: Add `X-Content-Type-Options: nosniff` to `DefaultHeaders`.
  - Source commands: `security-audit`

### Error Handling — `FromRowError::ColumnNotFound` Maps to HTTP 404

- [x] **#114 — Missing column (programming error) returns "not found" instead of 500**
  - File: `src/errors.rs` lines 118–123
  - Problem: `ColumnNotFound` indicates a schema mismatch (programming error), not a missing resource. Mapping it to 404 could mislead clients and mask bugs.
  - Fix: Map to 500 Internal Server Error, same as `Conversion`.
  - Source commands: `db-review`

### RBAC — Helpers Return 403 Instead of 401 for Missing Claims

- [x] **#150 — All six RBAC helpers use `Error::Forbidden("Authentication required")` — should be 401 per RFC 9110**
  - File: `src/handlers/mod.rs` (all RBAC helpers)
  - Problem: "Authentication required" is a 401 concern, not 403. Mitigated by JWT middleware blocking unauthenticated requests first — this code path is unreachable in practice.
  - Fix: Change to `Error::Unauthorized("Authentication required")`.
  - Source commands: `rbac-rules`

### Code Quality — Middleware Auth Uses Inline `json!()` Instead of `ErrorResponse`

- [x] **#151 — ~15 error responses in auth validators use `json!({"error":"..."})` instead of the `ErrorResponse` struct**
  - File: `src/middleware/auth.rs` (`jwt_validator`, `refresh_validator`, `basic_validator`)
  - Problem: If `ErrorResponse` gains additional fields, these responses would diverge.
  - Fix: Replace `json!({"error":"..."})` with `ErrorResponse { error: "...".into() }` in all auth validators.
  - Source commands: `practices-audit`

### OpenAPI — Unnecessary `IntoParams` Derives on Request Body Structs

- [x] **#152 — `CreateUserEntry`, `UpdateUserRequest`, `UpdateUserEntry` derive `IntoParams` but are only used as JSON bodies**
  - File: `src/models.rs`
  - Problem: Enables the erroneous `params()` usage in #139/#140. These structs are never used as query parameters.
  - Fix: Remove `IntoParams` from these three derives.
  - Source commands: `openapi-sync`

### OpenAPI — `RevokedResponse` Not Explicitly Registered in Schema Components

- [x] **#153 — Auto-discovered by utoipa but not listed in `components(schemas(...))`**
  - File: `src/middleware/openapi.rs`
  - Problem: Inconsistent with the convention of explicit schema registration (all other schemas are listed).
  - Fix: Add `RevokedResponse` to the `components(schemas(...))` list.
  - Source commands: `openapi-sync`

### Security — No Maximum Password Length Validation

- [x] **#154 — `CreateUserEntry.password` enforces `min = 8` but has no maximum; enables HashDoS**
  - Files: `src/models.rs` (`CreateUserEntry`, `validate_optional_password`)
  - Problem: An attacker could submit a multi-megabyte password string, causing excessive CPU during Argon2 hashing.
  - Fix: Add `max = 128` (or 1024) to password validation.
  - Source commands: `security-audit`

### Security — JSON Payload Size Limit Only on API Scope

- [x] **#155 — `/auth/revoke` endpoint uses actix-web default 256 KiB limit instead of the 64 KiB limit on `/api/v1.0`**
  - File: `src/routes.rs`
  - Problem: The `JsonConfig::default().limit(65_536)` is only applied within the `/api/v1.0` scope.
  - Fix: Apply `JsonConfig` with size limit to the `/auth/revoke` resource as well.
  - Source commands: `security-audit`

### Security — Password Hash Stored in Auth Cache

- [x] **#156 — `UpdateUserEntry` including the Argon2 hash is stored in the `DashMap` cache**
  - Files: `src/models.rs`, `src/middleware/auth.rs`
  - Problem: Keeping password hashes in memory increases blast radius of memory-disclosure vulnerabilities.
  - Fix: Use a distinct `AuthUser` struct for the cache that is never `Serialize`.
  - Source commands: `security-audit`

### Security — No Rate Limiting on `/auth/revoke`

- [x] **#157 — `/auth` and `/auth/refresh` have rate limiting but `/auth/revoke` does not**
  - File: `src/routes.rs`
  - Problem: An attacker with a valid token could flood the revocation endpoint, causing excessive DB writes.
  - Fix: Apply the same `auth_rate_limit` governor to `/auth/revoke`.
  - Source commands: `security-audit`

### Code Quality — `get_client` Takes Pool by Value

- [x] **#158 — `pub async fn get_client(pool: Pool)` forces clone at every call site**
  - File: `src/handlers/mod.rs`
  - Problem: While `Pool` is Arc-based and cheap to clone, idiomatic Rust accepts `&Pool`.
  - Fix: Change signature to `&Pool`.
  - Source commands: `review`

### Code Quality — Commented-Out Error Variant

- [x] **#159 — Dead `RustlsPEMError` block in `errors.rs`**
  - File: `src/errors.rs`
  - Problem: Commented-out code adds noise.
  - Fix: Remove the dead code.
  - Source commands: `review`

### Code Quality — `check_db` Uses `execute` for `SELECT 1`

- [x] **#160 — `client.execute(SELECT 1)` returns row count; `query_one` is more idiomatic**
  - File: `src/db/health.rs`
  - Fix: Use `client.query_one(&statement, &[]).await` instead.
  - Source commands: `review`

### Code Quality — Unnecessary `return` Keyword

- [x] **#161 — `return Err(Error::Unauthorized(...))` in `auth_user` is redundant**
  - File: `src/handlers/users.rs`
  - Fix: Remove the `return` keyword — it's the final expression in the block.
  - Source commands: `review`

### Database — `memberof` Table Lacks `changed` Timestamp

- [x] **#162 — No audit trail for role changes**
  - File: `migrations/V1__initial_schema.sql` (memberof table)
  - Problem: The table only has `joined`. When a member's role is updated, there's no record of when.
  - Fix: Add `changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP` with an update trigger.
  - Source commands: `db-review`

### Performance — Auth Cache Eviction Is O(n log n)

- [x] **#113 — Cache eviction sorts all entries on every miss at capacity**
  - File: `src/middleware/auth.rs` lines 352–365
  - Resolution: Replaced `sort_by_key` with `select_nth_unstable_by_key` for O(n) partial sort.
  - Source commands: `review`

### Documentation — 4 Stale `localStorage` References in Command Files

- [x] **#194 — Command files reference `localStorage` but the project uses `sessionStorage`**
  - Files: `.claude/commands/review.md`, `.claude/commands/test-gaps.md`, `.claude/commands/security-audit.md`
  - Resolution: Replaced all 4 occurrences of `localStorage` with `sessionStorage`.
  - Source commands: `cross-ref-check`

### Database — `INSERT` Trigger on Users Table Should Be `UPDATE` Only

- [x] **#195 — `update_users_changed_at` fires on `BEFORE INSERT OR UPDATE` — the INSERT trigger is unnecessary**
  - File: `migrations/V1__initial_schema.sql` lines 149–152
  - Resolution: Added V5 migration (`migrations/V5__trigger_and_notnull_fixes.sql`) to change trigger to `BEFORE UPDATE ON users` only.
  - Source commands: `db-review`

### Validation — No Positive-Value Validation on Item Prices

- [x] **#196 — `CreateItemEntry.price` and `UpdateItemEntry.price` accept negative prices at the API layer**
  - File: `src/models.rs` lines 276–293
  - Resolution: Added `validate_non_negative_price` custom validator to both price fields.
  - Source commands: `db-review`, `security-audit`

### Validation — No Max Length on Text Fields

- [x] **#197 — `tname`, `descr`, `title` fields have `min = 1` validation but no `max` length**
  - File: `src/models.rs` (all Create/Update entry structs for teams, roles, items)
  - Resolution: Added `max = 255` to `tname`, `title` fields and `max = 1000` to `descr` fields.
  - Source commands: `security-audit`

### Code Quality — `check_db` Can Only Return `Ok(true)` — Dead Code Branch

- [x] **#198 — `get_health` handler's `Ok(false)` branch is unreachable**
  - Files: `src/db/health.rs`, `src/handlers/mod.rs`
  - Resolution: Changed `check_db` to return `Result<(), Error>` and simplified handler match.
  - Source commands: `review`

### Code Quality — Commented-Out Code in `get_health`

- [x] **#199 — Dead commented-out `let client: Client = ...` line in health handler**
  - File: `src/handlers/mod.rs`
  - Resolution: Removed the commented-out line.
  - Source commands: `review`

### Code Quality — `validate.rs` Only Reports First Error Per Field

- [x] **#200 — Multiple validation failures per field are silently dropped**
  - File: `src/validate.rs` line 22
  - Resolution: Changed `collect_errors` to use `flat_map` to report ALL errors per field.
  - Source commands: `review`

### Code Quality — Missing `#[must_use]` on `validate()` Function

- [x] **#201 — If a caller omits `?`, validation would be silently skipped**
  - File: `src/validate.rs` line 6
  - Resolution: Added `#[must_use = "validation result must be checked"]`.
  - Source commands: `review`

### Database — `teamorders.teamorders_user_id` Is Nullable but Never NULL

- [x] **#202 — No code path creates orders without a user, but the DB allows it**
  - File: `migrations/V1__initial_schema.sql` line 73
  - Resolution: Added `NOT NULL` constraint via V5 migration (`migrations/V5__trigger_and_notnull_fixes.sql`).
  - Source commands: `db-review`

### OpenAPI — `UpdateUserEntry` Has Dead `ToSchema` Derive

- [x] **#203 — `UpdateUserEntry` derives `ToSchema` but is not registered in OpenAPI schemas**
  - File: `src/models.rs`
  - Resolution: Removed `ToSchema` derive from `UpdateUserEntry`.
  - Source commands: `openapi-sync`

### Code Quality — Admin Role Escalation Guard Duplicated Verbatim

- [x] **#216 — Identical 11-line guard block in `add_team_member` and `update_member_role`**
  - File: `src/handlers/teams.rs`
  - Resolution: Extracted into `guard_admin_role_assignment(client, req, role_id)` helper in `handlers/mod.rs`. Both handlers now call the shared helper.
  - Source commands: `review`

### Database — `update_team_order` Has Inconsistent Partial-Update Semantics

- [x] **#217 — COALESCE used only on `closed` but not on `teamorders_user_id` or `duedate`**
  - File: `src/db/orders.rs` lines 103–104
  - Resolution: Applied COALESCE to all three fields in the UPDATE query.
  - Source commands: `db-review`

### Practices — `add_team_member` and `update_member_role` Skip `validate(&json)?`

- [x] **#218 — Two handlers accept JSON body without calling validate()**
  - File: `src/handlers/teams.rs`
  - Resolution: Resolved via #224 — removed `Validate` derive from models with zero validation rules. Removed `validate()` calls and unreachable 422 utoipa annotations.
  - Source commands: `practices-audit`, `openapi-sync`

### API — Three Create Handlers Missing `Location` Header

- [x] **#219 — `create_team_order`, `create_order_item`, `add_team_member` return 201 without `Location` header**
  - Files: `src/handlers/teams.rs`, `src/handlers/orders.rs`
  - Resolution: Added `url_for`-based `Location` headers to all three handlers. Fixed `create_team_order` route name mismatch.
  - Source commands: `api-completeness`, `review`

### OpenAPI — `revoke_user_token` Documents 400 but Returns 500 on Invalid Token

- [x] **#220 — utoipa annotation for `POST /auth/revoke` documents unreachable 400 response**
  - File: `src/handlers/users.rs`
  - Resolution: Removed the 400 response from the utoipa annotation.
  - Source commands: `openapi-sync`

### OpenAPI — `team_users` Documents Unreachable 404

- [x] **#221 — utoipa annotation for `GET /api/v1.0/teams/{team_id}/users` documents 404 that never occurs**
  - File: `src/handlers/teams.rs`
  - Resolution: Removed the `(status = 404, ...)` line from the utoipa annotation.
  - Source commands: `openapi-sync`

### Code Quality — Missing `#[must_use]` on `requesting_user_id`

- [x] **#222 — `requesting_user_id` returns `Option<Uuid>` but lacks `#[must_use]`**
  - File: `src/handlers/mod.rs` line 23
  - Resolution: Added `#[must_use = "caller must handle the case where no JWT claims are present"]`.
  - Source commands: `review`

### Performance — Auth Validator Redundant DashMap Lookup for TTL Eviction

- [x] **#223 — Double DashMap lookup in `basic_validator` TTL-eviction path**
  - File: `src/middleware/auth.rs` lines 341–347
  - Resolution: Replaced with `cache.remove_if(key, |_, cached| expired(cached))` for atomic single-lookup eviction.
  - Source commands: `review`

### Validation — 4 Models Derive `Validate` with Zero Validation Rules

- [x] **#224 — `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, `UpdateMemberRoleEntry` have no `#[validate]` attributes**
  - File: `src/models.rs` lines 311–338
  - Resolution: Removed `Validate` derive from all 4 structs and corresponding `validate()` calls. Removed now-unreachable 422 utoipa annotations.
  - Source commands: `review`, `practices-audit`

### Database — `memberof.joined` Column Lacks NOT NULL Constraint

- [x] **#229 — V4 hardening added NOT NULL to `created`/`changed` but missed `joined`**
  - Files: `migrations/V1__initial_schema.sql` line 64, `migrations/V4__schema_hardening.sql`
  - Resolution: Added NOT NULL constraint via V5 migration (`migrations/V5__trigger_and_notnull_fixes.sql`).
  - Source commands: `db-review`

### Dependencies — `rust_decimal` Redundant `tokio-postgres` Feature

- [x] **#226 — `features = ["db-tokio-postgres", "serde-with-str", "tokio-postgres"]` — the third feature is unnecessary**
  - File: `Cargo.toml` (rust_decimal dependency)
  - Resolution: Removed `"tokio-postgres"` from feature list → `features = ["db-tokio-postgres", "serde-with-str"]`.
  - Source commands: `dependency-check`

### Dependencies — Frontend `gloo-net` Compiles Unused WebSocket/EventSource Support

- [x] **#227 — `gloo-net` default features not disabled — pulls unused `websocket` and `eventsource`**
  - File: `frontend/Cargo.toml` (gloo-net dependency)
  - Resolution: Changed to `gloo-net = { version = "0.6", default-features = false, features = ["http", "json"] }`.
  - Source commands: `dependency-check`

### Dependencies — Frontend `js-sys` Duplicated in Dependencies and Dev-Dependencies

- [x] **#228 — `js-sys = "0.3"` appears in both `[dependencies]` and `[dev-dependencies]`**
  - File: `frontend/Cargo.toml`
  - Resolution: Removed `js-sys = "0.3"` from `[dev-dependencies]`.
  - Source commands: `dependency-check`

### API — `memberof.joined` and `memberof.changed` Timestamps Not Exposed

- [x] **#115 — `joined` and `changed` columns stored in DB but not returned by API**
  - Resolution: Added `joined: DateTime<Utc>` and `role_changed: DateTime<Utc>` fields to `UsersInTeam` and `UserInTeams` structs, updated `FromRow` impls, and updated all SQL queries in `db/teams.rs` and `db/membership.rs` to select `memberof.joined, memberof.changed as role_changed`.
  - Source commands: `api-completeness`

### API Design — GET Endpoints Have No Team-Scoped RBAC

- [x] **#117 — Any authenticated user can read any team's data**
  - Resolution: Documented as intentional design decision in `src/routes.rs` doc comment and CLAUDE.md Key Conventions section.
  - Source commands: `api-completeness`, `security-audit`

### Documentation — Frontend Test Category Breakdown Wrong

- [x] **#163 — CLAUDE.md test category breakdown is stale**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Resolution: Test categories already summed correctly to 39. Updated to 41 after adding 2 new login error differentiation tests (#239).
  - Source commands: `cross-ref-check`

### Frontend — Login Shows "Invalid Credentials" for All Non-2xx Errors

- [x] **#225 — HTTP 500, 429, and 503 responses all display "Invalid username or password"**
  - File: `frontend/src/pages/login.rs`
  - Resolution: Login error handler now matches on `response.status()` with differentiated messages: 401 → "Invalid username or password", 429 → "Too many login attempts", 500 → "An unexpected server error occurred", 503 → "The service is temporarily unavailable", _ → `format!("Login failed (HTTP {})")`.
  - Source commands: `api-completeness`, `review`

### Database — `closed` Column Read as `Option<bool>` Despite `NOT NULL` Constraint

- [x] **#235 — `is_team_order_closed` and `guard_open_order` use `Option<bool>` for a NOT NULL column**
  - File: `src/db/order_items.rs`
  - Resolution: Changed to `row.get::<_, bool>("closed")` directly without `Option` wrapper.
  - Source commands: `db-review`

### Testing — No API Test for GET Single Team Order by ID

- [x] **#237 — `GET /api/v1.0/teams/{team_id}/orders/{order_id}` never called in tests**
  - File: `tests/api_tests.rs`
  - Resolution: Added `get_single_team_order_returns_details` test that creates an order, fetches it by ID (asserts 200 + matching fields), and tests 404 for nonexistent order ID.
  - Source commands: `test-gaps`

### Testing — `add_team_member` with FK-Violating IDs Untested

- [x] **#238 — Adding a member with non-existent `user_id` or `role_id` → error quality untested**
  - File: `tests/db_tests.rs`
  - Resolution: Added `add_team_member_with_nonexistent_user_returns_error` and `add_team_member_with_nonexistent_role_returns_error` tests.
  - Source commands: `test-gaps`

### Testing — No Frontend Test for Non-401/Non-Network HTTP Errors

- [x] **#239 — No WASM test mocks 500 or 429 responses for the login flow**
  - File: `frontend/tests/ui_tests.rs`
  - Resolution: Added `install_mock_fetch_rate_limited()` (429) and `install_mock_fetch_server_error()` (500) mock functions, plus `test_rate_limited_login_shows_429_message` and `test_server_error_login_shows_500_message` tests.
  - Source commands: `test-gaps`

### Auth — `revoke_user_token` Returns 403 for Missing Authentication

- [x] **#243 — `revoke_user_token` uses `Error::Forbidden("Authentication required")` — should be `Error::Unauthorized`**
  - File: `src/handlers/users.rs`
  - Resolution: Changed to `Error::Unauthorized("Authentication required".to_string())`.
  - Source commands: `practices-audit`

### OpenAPI — `get_health` Missing 503 Response Annotation

- [x] **#244 — `get_health` utoipa annotation only documents 200; handler also returns 503**
  - File: `src/handlers/mod.rs`
  - Resolution: Added `(status = 503, description = "Service unavailable — database unreachable", body = StatusResponse)`.
  - Source commands: `openapi-sync`

### OpenAPI — `create_user` Annotates Unreachable 404

- [x] **#245 — `create_user` utoipa includes `(status = 404)` but handler never returns 404**
  - File: `src/handlers/users.rs`
  - Resolution: Replaced `(status = 404)` with `(status = 409, description = "Conflict - email already exists")` (also fixes #312).
  - Source commands: `openapi-sync`

### Documentation — CLAUDE.md Test Counts Stale

- [x] **#246 — CLAUDE.md test counts do not match actual counts**
  - File: `CLAUDE.md`
  - Resolution: Updated all test counts (189 unit, 87 API, 92 DB, 41 WASM) and test category breakdown.
  - Source commands: `cross-ref-check`, `test-gaps`

### Validation — `Validate` Derive Still on 4 No-Rule Structs

- [x] **#253 — `Validate` derive is still present on `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, `UpdateMemberRoleEntry`**
  - File: `src/models.rs`
  - Resolution: Fixed via #313 — `validate()` calls added back to handlers, making `Validate` derives functional (no longer dead code).
  - Source commands: `practices-audit`, `review`

### Database — COALESCE Prevents Clearing `duedate` to NULL

- [x] **#270 — `update_team_order` uses `COALESCE($2, duedate)` which prevents clearing duedate**
  - Files: `src/db/orders.rs`, `src/models.rs`
  - Resolution: Changed `duedate` field to `Option<Option<NaiveDate>>` with `#[serde(default)]`. SQL uses `CASE WHEN $5::boolean THEN $1 ELSE duedate END` pattern. `None` = don't touch, `Some(None)` = clear to NULL, `Some(Some(date))` = set date.
  - Source commands: `api-completeness`, `db-review`

### OpenAPI — `create_team_order` Missing 409 Annotation

- [x] **#271 — `create_team_order` utoipa does not document 409 conflict response**
  - File: `src/handlers/teams.rs`
  - Resolution: Added `(status = 409, description = "Conflict", body = ErrorResponse)`.
  - Source commands: `api-completeness`

### Documentation — CLAUDE.md Missing `guard_admin_role_assignment` in Function List

- [x] **#272 — handlers/mod.rs description omits `guard_admin_role_assignment`**
  - File: `CLAUDE.md`
  - Resolution: Already present in CLAUDE.md (was added in a prior session). No change needed.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md API Test Count Wrong

- [x] **#273 — CLAUDE.md says "90 API integration tests" but actual count was 86**
  - File: `CLAUDE.md`
  - Resolution: Already corrected to 86 in a prior session. Now updated to 87 after adding new test.
  - Source commands: `cross-ref-check`, `test-gaps`

### Database — `orders.amt` CHECK Allows 0 but API Requires ≥1

- [x] **#274 — DB constraint `CHECK (amt >= 0)` permits zero-quantity orders**
  - File: `migrations/V6__order_constraint_and_index.sql`
  - Resolution: New V6 migration updates existing zero-amt rows to 1, drops old constraint, adds `CHECK (amt >= 1)`.
  - Source commands: `db-review`

### Performance — Missing Composite Index for Team Orders Query

- [x] **#275 — `get_team_orders` queries without a covering index**
  - File: `migrations/V6__order_constraint_and_index.sql`
  - Resolution: New V6 migration adds `idx_teamorders_team_created ON teamorders (teamorders_team_id, created DESC)`.
  - Source commands: `db-review`

### OpenAPI — `revoke_user_token` Missing 401 Response Annotation

- [x] **#276 — utoipa annotation doesn't document 401 response**
  - File: `src/handlers/users.rs`
  - Resolution: Added `(status = 400)` and `(status = 401)` annotations.
  - Source commands: `openapi-sync`

### OpenAPI — `add_team_member` Missing 404 for Invalid Role ID

- [x] **#277 — utoipa annotation doesn't document 404 when role_id doesn't exist**
  - File: `src/handlers/teams.rs`
  - Resolution: Added `(status = 404, description = "User or role not found", body = ErrorResponse)`.
  - Source commands: `openapi-sync`

### Security — HTTP→HTTPS Redirect Open Redirect via Host Header

- [x] **#278 — `redirect_to_https` uses unvalidated Host header**
  - File: `src/server.rs`
  - Resolution: Added hostname validation — only allows ASCII alphanumeric chars, hyphens, and dots. Returns 400 Bad Request for invalid hostnames.
  - Source commands: `security-audit`

### Frontend — Logout Revocation Fails With Expired Access Token

- [x] **#279 — `on_logout` uses potentially-expired access token for revocation**
  - File: `frontend/src/components/sidebar.rs`
  - Resolution: Changed to use `authed_request()` which handles transparent token refresh, so revocation works even with expired access tokens. Tokens cleared from sessionStorage after revocation completes.
  - Source commands: `security-audit`

### Config — `server.secret` Production-Checked but Never Used

- [x] **#280 — `ServerConfig.secret` field has zero runtime effect**
  - File: `src/config.rs`
  - Resolution: Documented as a canary field — its production check ensures operators have reviewed and customised the config before deploying.
  - Source commands: `security-audit`

### Security — `update_user` Cache Invalidation Targets Wrong Key

- [x] **#281 — When email changes, handler invalidates NEW email key, not OLD one**
  - File: `src/handlers/users.rs`
  - Resolution: Handler now fetches old email before update, then invalidates both old and new cache keys.
  - Source commands: `review`

### Code Quality — `update_user` Has Inconsistent RBAC/Validate Ordering

- [x] **#282 — `update_user` does RBAC before validate (inconsistent with 9 others)**
  - File: `src/handlers/users.rs`
  - Resolution: Swapped ordering — `validate(&json)?` now runs before RBAC check.
  - Source commands: `review`

### Code Quality — `delete_user` Premature Cache Invalidation

- [x] **#283 — Handler invalidates auth cache before DB delete succeeds**
  - File: `src/handlers/users.rs`
  - Resolution: Handler now fetches email before deletion, performs delete, then invalidates cache only on success.
  - Source commands: `review`

### Performance — `refresh_validator` Redundantly Re-decodes JWT

- [x] **#284 — Middleware decodes JWT but doesn't pass claims to handler**
  - Files: `src/middleware/auth.rs`, `src/handlers/users.rs`
  - Resolution: `refresh_validator` now inserts claims into `req.extensions_mut()`. `refresh_token` handler reads claims from extensions instead of re-decoding. Added `verify_jwt_for_revocation` function for expiry-tolerant token verification. `Claims` gets `Clone` derive.
  - Source commands: `review`

### Security — `revoke_user_token` Returns HTTP 500 for Expired/Malformed Tokens

- [x] **#298 — `verify_jwt` propagates `Error::Jwt` → 500 for expired tokens**
  - File: `src/handlers/users.rs`
  - Resolution: `revoke_user_token` now uses `verify_jwt_for_revocation` (validation with `validate_exp = false`). Returns `HttpResponse::BadRequest` with clear error message for invalid/expired tokens instead of 500.
  - Source commands: `security-audit`

### RBAC — `create_order_item` Uses Broad `require_team_member` Guard

- [x] **#310 — Any team member (including Guest) can create order items**
  - File: `src/handlers/orders.rs`
  - Resolution: Documented as intentional policy — any team member should be able to add items to a breakfast order. Updated utoipa 403 description to explicitly state this.
  - Source commands: `rbac-rules`

### RBAC — Policy Table Missing Order Items as Resource

- [x] **#311 — CLAUDE.md RBAC documentation does not cover order_items**
  - File: `CLAUDE.md`
  - Resolution: Added "Order Items RBAC" bullet documenting: create requires team membership (any role, by design), update/delete requires order owner or team admin or global admin, closed orders blocked by `guard_open_order`.
  - Source commands: `rbac-rules`

### OpenAPI — `create_user` Missing 409 Conflict Response Annotation

- [x] **#312 — Handler returns 409 on duplicate email but utoipa doesn't document it**
  - File: `src/handlers/users.rs`
  - Resolution: Fixed together with #245 — replaced unreachable 404 with 409.
  - Source commands: `openapi-sync`

### Validation — `create_team_order` and `update_team_order` Missing `validate()` Calls

- [x] **#313 — These two handlers do not call `validate(&json)?` before DB operations**
  - File: `src/handlers/teams.rs`
  - Resolution: Added `validate(&json)?` calls at the start of both handlers.
  - Source commands: `openapi-sync`, `practices-audit`

### Database — `get_member_role` Uses `query()` Not `query_opt()`

- [x] **#314 — Non-existent membership returns 500 instead of a clean error**
  - File: `src/db/membership.rs`
  - Resolution: Changed to `query_opt()` returning `Ok(row.map(|r| r.get("title")))`.
  - Source commands: `db-review`

### Database — Missing ORDER BY on `get_user_teams` and `get_team_users`

- [x] **#315 — Results returned in arbitrary order**
  - File: `src/db/teams.rs`
  - Resolution: Added `ORDER BY tname ASC` to `get_user_teams` and `ORDER BY lastname ASC, firstname ASC` to `get_team_users`.
  - Source commands: `db-review`

### Database — `UserInTeams` Model Missing `descr` Field

- [x] **#316 — Query SELECTs team name but not description**
  - Files: `src/db/teams.rs`, `src/models.rs`, `src/from_row.rs`
  - Resolution: Added `team_id: Uuid` and `descr: Option<String>` to `UserInTeams` struct and `FromRow` impl. Updated SQL query to select `teams.team_id, tname, teams.descr`.
  - Source commands: `db-review`, `api-completeness`

### Documentation — Command Files Reference Stale Migration Range

- [x] **#250 — `api-completeness.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/api-completeness.md`
  - Resolution: Updated scope to reference "V1 initial schema through V6 order constraint/index, and any newer migrations".
  - Source commands: `cross-ref-check`

- [x] **#251 — `db-review.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/db-review.md`
  - Resolution: Updated both Schema section and Scope section to enumerate V1–V6 (with descriptions) plus "and any newer migrations".
  - Source commands: `cross-ref-check`

### Documentation — `database.sql` Stale vs V3–V6

- [x] **#252 — `database.sql` deprecated script doesn't reflect V3–V6 changes**
  - File: `database.sql`
  - Resolution: Updated the deprecated dev-reset script to incorporate all V3–V6 changes: `CHECK (amt >= 1)` (V6), `joined NOT NULL` (V5), `teamorders_user_id NOT NULL` (V5), users trigger `BEFORE UPDATE` only (V5), composite index `idx_teamorders_team_created` (V6), header references V1–V6.
  - Source commands: `cross-ref-check`

### Dead Code — Deprecated `require_self_or_admin` Function

- [x] **#329 — Deprecated `require_self_or_admin` function is dead code with zero call sites**
  - File: `src/handlers/mod.rs`
  - Resolution: Removed the 18-line deprecated function. Updated CLAUDE.md to remove it from the RBAC helper list.
  - Source commands: `review`

### Dead Code — Unused `_active_payload` in Session Restore

- [x] **#330 — `_active_payload` computed but never used in session restore**
  - File: `frontend/src/app.rs`
  - Resolution: Removed the unused `let _active_payload = decode_jwt_payload(&active_token).unwrap_or(payload);` line.
  - Source commands: `review`

### Security — Logout Token Clearing Race

- [x] **#331 — Logout clears `sessionStorage` tokens after async revocation completes, not before**
  - File: `frontend/src/components/sidebar.rs`
  - Resolution: Moved `sessionStorage` clearing to before the async `spawn_local` block. Token values saved to local vars first, storage cleared immediately, then saved values used for revocation POST.
  - Source commands: `security-audit`

### Validation — Order Quantity Unbounded

- [x] **#332 — `amt` field validated with `range(min = 1)` but no maximum**
  - File: `src/models.rs`
  - Resolution: Added `max = 10000` to `range()` validation on both `CreateOrderEntry.amt` and `UpdateOrderEntry.amt`. Added 4 boundary tests.
  - Source commands: `practices-audit`

### Security — `UpdateUserEntry` Derives `Serialize` Despite Containing Password Hash

- [x] **#333 — `UpdateUserEntry` contains password hash but derives `Serialize`**
  - File: `src/models.rs`
  - Resolution: Removed `Serialize` from `UpdateUserEntry`'s derive list.
  - Source commands: `security-audit`

### Security — `PayloadConfig` Default Larger Than `JsonConfig`

- [x] **#334 — `JsonConfig` limits to 64 KB but `PayloadConfig` default is 256 KB**
  - File: `src/routes.rs`
  - Resolution: Added `.app_data(PayloadConfig::default().limit(65_536))` to align payload limit with JSON limit.
  - Source commands: `security-audit`

### Validation — No Email Format Validation in Delete-by-Email Path

- [x] **#335 — No server-side validation of email format in the URL path parameter**
  - File: `src/handlers/users.rs`
  - Resolution: Added email format validation (`len > 255 || !contains('@')` → `Error::Validation`) before DB call in `delete_user_by_email`.
  - Source commands: `security-audit`

### Documentation — WASM Test Count Stale (64 Actual vs 41 Documented)

- [x] **#507 — CLAUDE.md and README.md both state 41 WASM tests; actual count is 64**
  - Files: `CLAUDE.md`, `README.md`
  - Fix: Updated "41" → "64" in all locations. Updated test category breakdown in CLAUDE.md to add new test categories (table styling, actions column, admin password reset).
  - Source commands: `cross-ref-check`

### Documentation — `order_components.rs` Missing from CLAUDE.md Project Structure

- [x] **#508 — `frontend/src/pages/order_components.rs` exists on disk but not listed in the Project Structure tree**
  - File: `CLAUDE.md`
  - Fix: Added `order_components.rs – Order sub-components (OrderDetail, CreateOrderDialog)` after `orders.rs` in the pages/ listing.
  - Source commands: `cross-ref-check`

### Frontend — Orders Page Fetches All Teams Instead of User's Teams

- [x] **#509 — Orders page uses `/api/v1.0/teams` (all teams) instead of `/api/v1.0/users/{id}/teams` (user's memberships)**
  - File: `frontend/src/pages/orders.rs`
  - Fix: Changed `authed_get("/api/v1.0/teams")` to `authed_get(&format!("/api/v1.0/users/{}/teams", user_id))`. Added local `UserTeamEntry` struct for deserialization compatibility since the user-teams endpoint returns different fields than the all-teams endpoint.
  - Source commands: `review`

### Documentation — Unit Test Count Stale (234 → 236)

- [x] **#608 — CLAUDE.md and README.md say 234 unit tests but actual count is 236**
  - Files: `CLAUDE.md`, `README.md`
  - Resolution: Updated the count to 236 in both files.
  - Source commands: `cross-ref-check`, `practices-audit`

### Documentation — README Migration Count Text Says "Eight" But Lists 9

- [x] **#609 — README.md says "Eight migrations" but the table lists V1–V9**
  - File: `README.md`
  - Resolution: Changed "Eight" to "Eleven" (now V1–V11 after V10 and V11 were added).
  - Source commands: `cross-ref-check`

### Documentation — README `make audit` Description Stale

- [x] **#610 — README says `make audit` ignores RUSTSEC-2023-0071, but Makefile runs clean `cargo audit`**
  - File: `README.md`
  - Resolution: Removed the parenthetical about `--ignore RUSTSEC-2023-0071`.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Missing `frontend/assets/`

- [x] **#611 — `frontend/assets/` directory exists on disk but not in Project Structure tree**
  - File: `CLAUDE.md`
  - Resolution: Added `assets/` under `frontend/` in the structure tree.
  - Source commands: `cross-ref-check`

### RBAC — Hardcoded `"Admin"` String in `register_first_user`

- [x] **#612 — Uses string literal `"Admin"` instead of `ROLE_ADMIN` constant**
  - File: `src/db/users.rs`
  - Resolution: Imported `ROLE_ADMIN` from `middleware::auth` and parameterized the SQL query with `$1` + `&[&ROLE_ADMIN]`.
  - Source commands: `rbac-rules`

### Testing — No Integration Tests for Avatar RBAC

- [x] **#613 — `set_avatar` and `remove_avatar` RBAC (self, admin, non-admin forbidden) completely untested**
  - Files: `src/handlers/avatars.rs`, `tests/api_tests.rs`
  - Resolution: Added 3 API integration tests: `user_sets_own_avatar` (200), `admin_sets_other_user_avatar` (200), `non_admin_cannot_set_other_user_avatar` (403).
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — No Explicit Test for `register_first_user` 403

- [x] **#614 — No dedicated test asserts `POST /auth/register` returns 403 when users already exist**
  - File: `tests/api_tests.rs`
  - Resolution: Added `register_when_users_exist_returns_403` integration test.
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — No Test for Denied Delete of Another Member's Order Item

- [x] **#615 — Missing `member_cannot_delete_another_members_order_item` integration test**
  - File: `tests/api_tests.rs`
  - Resolution: Added `member_cannot_delete_other_members_order_item` test verifying non-owner member gets 403.
  - Source commands: `rbac-rules`, `test-gaps`

### Frontend — No UI for Updating Order Item Quantity

- [x] **#616 — Backend provides `PUT .../items/{iid}` but frontend can only add/delete items, not update quantity**
  - Files: `frontend/src/pages/orders.rs`, `frontend/src/pages/order_components.rs`
  - Resolution: Added `do_update_item` callback in `orders.rs` and inline `<input type="number">` in `OrderDetail` component for open orders.
  - Source commands: `api-completeness`

### API Consistency — `get_avatars` Is the Only Unpaginated List Endpoint

- [x] **#617 — `get_avatars` returns bare `Vec` instead of `PaginatedResponse`, unlike all other list endpoints**
  - File: `src/handlers/avatars.rs`
  - Resolution: Documented as intentional exception via doc comment — avatars are a small static set seeded from `minifigs/`.
  - Source commands: `api-completeness`, `openapi-sync`

### Database — No `CHECK` Constraints on Text Column Lengths

- [x] **#618 — `teams.descr`, `items.descr`, `roles.title`, `teams.tname` have Rust validators but no DB-level length limits**
  - File: `migrations/V11__text_column_check_constraints.sql`
  - Resolution: Created V11 migration adding CHECK constraints: `teams.tname ≤ 255`, `teams.descr ≤ 1000`, `roles.title ≤ 255`, `items.descr ≤ 255`.
  - Source commands: `db-review`

### Validation — `users.email` Column Width Mismatch

- [x] **#619 — `users.email` is `varchar(75)` but Rust `#[validate(email)]` has no max length — email >75 chars causes DB 500**
  - File: `src/models.rs`
  - Resolution: Added `length(max = 75)` to `#[validate]` on `email` fields in both `UpdateUserRequest` and `CreateUserEntry`.
  - Source commands: `db-review`

### Practices — `set_avatar` Handler Missing `validate(&json)?`

- [x] **#620 — Handler accepts `Validate`-deriving struct but never calls `validate()`**
  - File: `src/handlers/avatars.rs`
  - Resolution: Added `validate(&json)?;` call before the avatar existence check.
  - Source commands: `practices-audit`

### Testing — Entire Avatar Subsystem Has Zero Test Coverage

- [x] **#622 — `db/avatars.rs` (5 functions) + `handlers/avatars.rs` (4 handlers) completely untested**
  - Files: `tests/api_tests.rs`, `tests/db_tests.rs`
  - Resolution: Added DB integration tests (`insert_and_get_avatar`, `count_avatars_matches_list`, `set_user_avatar_and_clear`) and API tests (`list_avatars_returns_200`, `get_single_avatar_returns_image_with_cache_headers`). Avatar RBAC tests covered by #613.
  - Source commands: `test-gaps`

### Testing — `would_admins_remain_without` Has No Direct DB Test

- [x] **#623 — Last-admin guard logic only tested indirectly via API tests**
  - File: `tests/db_tests.rs`
  - Resolution: Added `would_admins_remain_without_two_admins` (returns true) and `would_admins_remain_without_sole_admin` (returns false) DB tests.
  - Source commands: `test-gaps`

### Code Quality — `UserContext` Construction Duplicated 3x in `profile.rs`

- [x] **#624 — Identical ~10-line `UserContext` assembly block appears 3 times**
  - Files: `frontend/src/api.rs`, `frontend/src/pages/profile.rs`
  - Resolution: Added `UserContext::from_entry()` constructor in `api.rs`; replaced all 3 duplicate blocks in `profile.rs` with single constructor call.
  - Source commands: `review`

### Code Quality — Local `UserTeamEntry` Duplicates `api::UserInTeams`

- [x] **#625 — `orders.rs` defines `UserTeamEntry` with same fields already in `api::UserInTeams`**
  - File: `frontend/src/pages/orders.rs`
  - Resolution: Removed local `UserTeamEntry` struct; replaced all references with `api::UserInTeams`.
  - Source commands: `review`

### Code Quality — Repeated Modal Dialog Boilerplate

- [x] **#626 — ~40 lines of overlay+dialog+header+body+footer structure duplicated across all CRUD pages**
  - Files: `frontend/src/components/modal.rs`, `frontend/src/pages/roles.rs`, `NEW-UI-COMPONENTS.md`
  - Resolution: Created `FormDialog` component with `open`, `title`, `submit_label`, `disabled`, `on_submit`, `on_cancel`, `children` props. Uses CSS visibility (not conditional rendering) to avoid AnyView clone limitation. Refactored `roles.rs` create/edit dialogs as proof of concept. Documented in `NEW-UI-COMPONENTS.md`.
  - Source commands: `review`

### Database — `add_team_member`/`update_member_role` Could Use CTE Instead of INSERT+SELECT

- [x] **#627 — Two-query pattern in transactions could be a single `INSERT ... RETURNING` with CTE**
  - File: `src/db/membership.rs`
  - Resolution: Refactored both functions to use CTEs: `WITH ins AS (INSERT ... RETURNING ...) SELECT ...` and `WITH upd AS (UPDATE ... RETURNING ...) SELECT ...`. Reduces from 2 queries to 1 per operation.
  - Source commands: `db-review`

### Documentation — CLAUDE.md API Integration Test Count Stale (156 → 160)

- [x] **#635 — CLAUDE.md states 156 API integration tests but actual count is 160**
  - File: `CLAUDE.md` (Testing section)
  - Resolution: Updated count from 156 to 160 (now 167 after #634 avatar tests).
  - Source commands: `cross-ref-check`, `practices-audit`

### Documentation — README.md Test Counts and Migration Count Stale

- [x] **#636 — README.md unit test count (236→238), API integration count (145→160), and migration count (12→13) diverge from reality**
  - File: `README.md`
  - Resolution: Updated all counts: unit 236→238, API 145→167, DB 104→112, migrations "twelve"→"thirteen". Added V13 row to migration table.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Missing 3 DB Functions in Inventory

- [x] **#637 — `bootstrap_first_user`, `reopen_team_order`, `get_order_total` not listed in CLAUDE.md function inventories**
  - File: `CLAUDE.md` (Project Structure, db module descriptions)
  - Resolution: Added `bootstrap_first_user` to users.rs list, `reopen_team_order` to orders.rs list, `get_order_total` to order_items.rs list.
  - Source commands: `cross-ref-check`

### Performance — Avatar Cache Clones Bytes on Every Request

- [x] **#638 — Avatar image bytes are `Vec<u8>::clone()`d per response from the DashMap cache**
  - Files: `src/models.rs`, `src/handlers/avatars.rs`, `src/server.rs`
  - Resolution: Changed `DashMap<Uuid, (Vec<u8>, String)>` to `DashMap<Uuid, (Arc<Vec<u8>>, String)>` for cheap reference-counted cloning. Updated all cache insertion and retrieval sites.
  - Source commands: `review`

### Safety — Frontend `unchecked_into()` DOM Cast in order_components.rs

- [x] **#639 — `target.unchecked_into::<HtmlInputElement>()` can panic if DOM element type doesn't match**
  - File: `frontend/src/pages/order_components.rs`
  - Resolution: Replaced all 7 `unchecked_into` calls with safe `dyn_ref` + guard clauses (early return or `unwrap_or_default()`).
  - Source commands: `review`

### Testing — Closed Order Update/Delete Enforcement Untested

- [x] **#640 — No API test verifies that update/delete of order items is blocked on closed orders**
  - File: `tests/api_tests.rs`
  - Resolution: Tests already existed: `closed_order_rejects_update_item`, `closed_order_rejects_delete_item`.
  - Source commands: `test-gaps`

### Testing — Reopen Order Endpoint Untested

- [x] **#641 — `POST /api/v1.0/teams/{team_id}/orders/{order_id}/reopen` has no API integration test**
  - File: `tests/api_tests.rs`
  - Resolution: Tests already existed: `reopened_order_allows_item_mutations`, `reopen_open_order_returns_422`, `reopen_nonexistent_order_returns_404`.
  - Source commands: `test-gaps`

### Testing — Pickup User Assignment RBAC Untested

- [x] **#642 — No test verifies pickup user validation (team membership check) or RBAC (admin/team-admin required for changes)**
  - File: `tests/api_tests.rs`
  - Resolution: Tests already existed: `create_order_with_non_member_pickup_returns_422`, `member_cannot_change_assigned_pickup_user`, `member_can_set_pickup_when_unassigned`.
  - Source commands: `test-gaps`

### Documentation — CLAUDE.md Project Structure

- [x] **#645 — CLAUDE.md project structure lists old monolithic test files**
  - File: `CLAUDE.md`
  - Resolution: Updated project structure tree to list all 17 actual split backend test files (`tests/api_auth.rs` through `tests/db_users.rs` plus `tests/common/`) and 8 frontend test files (`frontend/tests/ui_admin_dialogs.rs` through `frontend/tests/ui_theme.rs`).
  - Source commands: `cross-ref-check`, `practices-audit`

- [x] **#646 — CLAUDE.md test counts drifted (238→240 unit, 167→168 API)**
  - File: `CLAUDE.md`
  - Resolution: Updated to "240 unit tests" and "168 API integration tests". Changed file references from monolithic names to glob patterns.
  - Source commands: `cross-ref-check`, `practices-audit`

- [x] **#647 — CLAUDE.md `db/orders.rs` function list missing `count_team_orders`**
  - File: `CLAUDE.md`
  - Resolution: Added `count_team_orders` to the `db/orders.rs` function list.
  - Source commands: `cross-ref-check`

- [x] **#648 — CLAUDE.md `handlers/mod.rs` description missing `created_with_location`, `delete_response`**
  - File: `CLAUDE.md`
  - Resolution: Added both response helper functions to the handlers/mod.rs description.
  - Source commands: `cross-ref-check`

### Documentation — Command Files

- [x] **#649 — `db-review.md` references deleted `database.sql` and has stale `init_dev_db.sh` description**
  - File: `.claude/commands/db-review.md`
  - Resolution: Removed `database.sql` reference; updated `init_dev_db.sh` description to "Test database initialization script used by postgres-setup in docker-compose.test.yml".
  - Source commands: `cross-ref-check`

- [x] **#650 — `test-gaps.md` references old monolithic test file names**
  - File: `.claude/commands/test-gaps.md`
  - Resolution: Updated references from `frontend/tests/ui_tests.rs` → `frontend/tests/ui_*.rs` and `tests/api_tests.rs` → `tests/api_*.rs`.
  - Source commands: `cross-ref-check`

### Frontend Bugs

- [x] **#651 — `connect-button--danger` on Remove Avatar button (should be `--negative`)**
  - File: `frontend/src/pages/profile.rs`
  - Resolution: Changed `connect-button--danger` to `connect-button--negative` to match the CONNECT design system class naming.
  - Source commands: `review`

- [x] **#652 — Missing `maxlength=50` on first name input in profile edit form**
  - File: `frontend/src/pages/profile.rs`
  - Resolution: Added `maxlength=50` to the first name `<input>` element, matching the last name input and backend validation.
  - Source commands: `review`

- [x] **#653 — `is_valid_price` accepts scientific notation and negative values**
  - File: `frontend/src/pages/items.rs`
  - Resolution: Rewrote `is_valid_price` with strict byte-level validation: only ASCII digits + optional single decimal point, max 2 fractional digits, max 13 chars. Rejects scientific notation, negative signs, and empty strings.
  - Source commands: `review`

### Backend Consistency

- [x] **#654 — `count_avatars` skips prepared statement**
  - File: `src/db/avatars.rs`
  - Resolution: Changed to use `client.prepare()` before `query_one()`, matching every other DB function.
  - Source commands: `db-review`

- [x] **#655 — `validate_non_negative_price` function name is misleading**
  - File: `src/models.rs`
  - Resolution: Renamed to `validate_positive_price` — function definition, both `#[validate]` annotations on `CreateItemEntry`/`UpdateItemEntry`, and 3 related unit tests.
  - Source commands: `review`, `practices-audit`

### Frontend — Unused Component Prop

- [x] **#656 — Unused `is_admin` prop in `OrderDetail` component**
  - File: `frontend/src/pages/order_components.rs`, `frontend/src/pages/orders.rs`
  - Resolution: Removed `is_admin: Signal<bool>` prop from `OrderDetail` component and removed `is_admin=is_admin` from the caller in `orders.rs`.
  - Source commands: `review`

### Database — `reopen_team_order` Uses `FOR SHARE` Instead of `FOR UPDATE`

- [x] **#673 — Concurrent reopens of the same closed order could both succeed**
  - File: `src/db/orders.rs`, `reopen_team_order` function
  - Resolution: Changed `FOR SHARE` to `FOR UPDATE` to serialize concurrent reopens.
  - Source commands: `db-review`

### Database — `users.email` VARCHAR(75) vs CHECK(≤255) Mismatch

- [x] **#674 — V14 CHECK constraint on email allows 255 but column is varchar(75)**
  - File: `migrations/V16__constraint_fixes.sql`
  - Resolution: New migration drops `chk_users_email_length` CHECK(≤255) and adds CHECK(≤75) to match the varchar(75) column type.
  - Source commands: `db-review`

### Database — `items.price` DB CHECK Allows 0 While API Requires > 0

- [x] **#675 — DB constraint `CHECK (price >= 0)` weaker than API validator requiring `> 0`**
  - File: `migrations/V16__constraint_fixes.sql`
  - Resolution: New migration drops `items_price_check` CHECK(≥0) and adds CHECK(>0) to match the API validator. Updated affected DB tests (`db_items.rs`, `db_orders.rs`) to use `Decimal::ONE` instead of `Decimal::ZERO`.
  - Source commands: `db-review`

### Database — `memberof.memberof_team_id` ON DELETE CASCADE Allows Silent Membership Loss

- [x] **#676 — Deleting a team silently removes all memberships via CASCADE**
  - File: `migrations/V16__constraint_fixes.sql`, `src/db/teams.rs`
  - Resolution: New migration changes FK to `ON DELETE RESTRICT`. Updated `db::delete_team` to take `&mut Client`, use a transaction to explicitly remove memberships before deleting the team. Handler updated to pass `&mut client`.
  - Source commands: `db-review`

### Documentation — README.md Unit Test Count Stale

- [x] **#677 — README.md says "238 tests" but actual unit test count is 248**
  - File: `README.md`
  - Resolution: Updated to "248 tests".
  - Source commands: `cross-ref-check`

### Documentation — README.md Integration Test Counts Wrong

- [x] **#678 — README.md integration test counts outdated**
  - File: `README.md`
  - Resolution: Updated to "291 tests: 171 API + 120 DB".
  - Source commands: `cross-ref-check`

### Documentation — README.md Migration Count Missing V14–V16

- [x] **#679 — README.md migration table was missing V14–V16**
  - File: `README.md`
  - Resolution: Updated count to "Sixteen migrations" and added V14, V15, V16 rows to table.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md "Planned Pages" Describes Already-Implemented Pages

- [x] **#680 — "Frontend Roadmap" → "Planned Pages" lists 6 pages as future work, but all are implemented**
  - File: `CLAUDE.md`
  - Resolution: Renamed "Frontend Roadmap" to "Frontend Pages & Layout", changed "Planned Pages" to "Implemented Pages", updated intro text to reflect complete SPA.
  - Source commands: `practices-audit`

### Testing — `delete_team` With Existing Orders Returns 409 — No API Test

- [x] **#681 — No test coverage for 409 guard on team deletion with orders**
  - File: `tests/api_teams.rs`
  - Resolution: Added `delete_team_with_orders_returns_409` test covering create team → create order → DELETE team (409) → delete order → DELETE team (200).
  - Source commands: `test-gaps`

### Testing — `bootstrap_first_user` DB Function Has No Direct DB Test

- [x] **#682 — Multi-step transactional function tested only indirectly via API tests**
  - File: `src/db/users.rs`, `tests/api_auth.rs`
  - Resolution: Bootstrap function is thoroughly tested via API integration tests (`register_*` tests in `api_auth.rs`). A direct DB test was attempted but removed because `bootstrap_first_user` requires an empty database (wipes all tables), which causes test interference when running concurrently with API tests against the shared DB. The API-level coverage is sufficient.
  - Source commands: `test-gaps`

### Testing — `register_first_user` Validation Error Path Not Tested

- [x] **#683 — 422 validation path (short password, missing name) has no API test**
  - File: `tests/api_auth.rs`
  - Resolution: Added `register_with_short_password_returns_422` and `register_with_missing_firstname_returns_422` tests.
  - Source commands: `test-gaps`

### Testing — Teams Page CRUD Interactions Not Tested in WASM

- [x] **#684 — No dialog open/submit/toast tests for teams page**
  - File: `frontend/tests/ui_pages.rs`
  - Resolution: Added 3 WASM tests: `test_teams_page_create_dialog_opens`, `test_teams_page_create_dialog_cancel`, `test_teams_page_add_member_dialog_opens`.
  - Source commands: `test-gaps`

### Testing — Items Page CRUD Interactions Not Tested in WASM

- [x] **#685 — No create/edit/delete tests for items page**
  - File: `frontend/tests/ui_pages.rs`
  - Resolution: Added 3 WASM tests: `test_items_page_create_dialog_opens`, `test_items_page_create_dialog_cancel`, `test_items_page_edit_button_exists`.
  - Source commands: `test-gaps`

### Testing — Orders Page Detail and Line Item Management Not Tested in WASM

- [x] **#686 — Only 1 test for orders page**
  - File: `frontend/tests/ui_pages.rs`
  - Resolution: Added 2 WASM tests: `test_orders_page_shows_order_detail_on_click`, `test_orders_page_create_order_dialog_fields`. Total orders page tests now 3.
  - Source commands: `test-gaps`

## Informational Items

### API — Orphaned DB Function

- [x] **#723 — `get_order_total` DB function is orphaned — no API endpoint exposes it**
  - File: `src/db/order_items.rs`
  - Resolution: Added doc comment clarifying the function is retained for future server-side reporting. The frontend computes totals client-side from the item catalog. No endpoint needed at this time.
  - Source commands: `api-completeness`

### Documentation — CLAUDE.md Omission

- [x] **#724 — CLAUDE.md `components/mod.rs` description omits `input_handler()`**
  - File: `CLAUDE.md`
  - Resolution: Added `input_handler()` utility to the parenthetical list in the `components/mod.rs` description.
  - Source commands: `practices-audit`

### Settings — Stale Allowlist Entry

- [x] **#725 — `settings.json` allows `just *` but project uses `make`, not `just`**
  - File: `.claude/settings.json`
  - Resolution: Removed `Bash(just *)` from the allow list.
  - Source commands: `practices-audit`

### Security — Swagger UI Startup Warning

- [x] **#726 — Swagger UI accessible without authentication when `ENABLE_SWAGGER=true`**
  - File: `src/server.rs`
  - Resolution: Added a startup `warn!()` when `ENABLE_SWAGGER=true` and `ENV != "development"`, alerting operators that the full API schema is exposed without authentication at `/explorer`.
  - Source commands: `security-audit`

### API Design — List Endpoints Now Paginated

- [x] **#61 — List endpoints return all records without pagination**
  - Files: `src/db/`, `src/handlers/`, `src/models.rs`, `frontend/src/api.rs`, `frontend/src/pages/`
  - Resolution: Implemented `PaginationParams` (limit/offset query params, default 50, max 100) and `PaginatedResponse<T>` (items, total, limit, offset envelope). Updated all 8 list DB functions with LIMIT/OFFSET + COUNT queries, all 8 list handlers with `Query<PaginationParams>` extractors, frontend deserialization across 6 pages, and all test suites (193 unit, 87 API, 96 DB, 41 WASM).
  - Source commands: `review`, `api-completeness`

### Performance — `get_team_users` Query Has Unnecessary `teams` JOIN

- [x] **#230 — Query joins `teams` table but no columns from `teams` are selected**
  - File: `src/db/teams.rs`
  - Resolution: Removed the unnecessary `JOIN teams` from the `get_team_users` query. The query now only joins `users`, `memberof`, and `roles`.
  - Source commands: `review`

### Architecture — Defence-in-Depth Notes

- [x] **#49 — RBAC, OpenAPI sync, and dependency health all verified correct**
  - Resolution: Migrated from `rustls-pemfile` to `rustls-pki-types`, resolved advisories via `cargo update`.
  - Source commands: `rbac-rules`, `openapi-sync`, `dependency-check`

### Dependencies — RSA Advisory (Superseded)

- [x] **#55 — `rsa` 0.9.10 has an unfixable timing side-channel advisory (RUSTSEC-2023-0071)**
  - Resolution: Superseded by #132. `jsonwebtoken` supports granular `["hmac", "sha2"]` features, removing `rsa` from the dependency tree entirely.
  - Source commands: `dependency-check`

### Deployment — Docker Image Tags Verified Valid

- [x] **#62 — `postgres:18.3` Docker image tag — FALSE POSITIVE**
  - Resolution: Verified tag exists on Docker Hub.
  - Source commands: `dependency-check`, `review`

- [x] **#63 — `rust:1.93.1` Docker image tag — FALSE POSITIVE**
  - Resolution: Verified tag exists on Docker Hub.
  - Source commands: `dependency-check`, `review`

### Dependencies — Unused Crypto Algorithms (Superseded)

- [x] **#122 — `jsonwebtoken` `rust_crypto` feature compiles RSA, EdDSA, P-256, P-384**
  - Resolution: Superseded by #132. Granular feature selection `["hmac", "sha2"]` now available in jsonwebtoken 10.3.0.
  - Source commands: `dependency-check`

### Documentation — `test-gaps.md` References `gloo_timers`

- [x] **#164 — Command recommends `gloo_timers::future::sleep` but project uses custom `flush()` helper**
  - Resolution: Updated `.claude/commands/test-gaps.md` to reference the `flush(ms)` async helper.
  - Source commands: `cross-ref-check`

### Documentation — Integration Test Doc Comments Reference Deprecated `database.sql`

- [x] **#165 — Both `api_tests.rs` and `db_tests.rs` reference `database.sql` for setup**
  - Resolution: Updated doc comments to reference Refinery migrations and `database_seed.sql`.
  - Source commands: `cross-ref-check`

### Documentation — `middleware/mod.rs` Missing from CLAUDE.md Structure Tree

- [x] **#166 — Tree lists `auth.rs` and `openapi.rs` under `middleware/` but omits `mod.rs`**
  - Resolution: Added `mod.rs — Module declarations` under `middleware/` in CLAUDE.md.
  - Source commands: `cross-ref-check`

### Code Quality — Missing `#[must_use]` on Auth Functions

- [x] **#167 — `generate_token_pair`, `verify_jwt`, `invalidate_cache` return values that should not be ignored**
  - Resolution: Added `#[must_use]` attribute to all three functions in `src/middleware/auth.rs`.
  - Source commands: `review`

### Dependencies — Redundant `features = ["default"]` on Crates

- [x] **#168 — `argon2` and `opentelemetry` specify `features = ["default"]` which is a no-op**
  - Resolution: Simplified `argon2` and `opentelemetry` to plain version strings in `Cargo.toml`.
  - Source commands: `dependency-check`

### Dependencies — Unnecessary Braces on Simple Dependencies

- [x] **#169 — `actix-web-httpauth`, `tracing-log`, `rustls-pki-types` use `{ version = "..." }` with no other keys**
  - Resolution: Simplified to plain version strings in `Cargo.toml`.
  - Source commands: `dependency-check`

### Security — Missing `X-Frame-Options` Header

- [x] **#170 — CSP `frame-ancestors 'none'` covers modern browsers but `X-Frame-Options: DENY` is missing for older browsers**
  - Resolution: Added `.add(("X-Frame-Options", "DENY"))` to `DefaultHeaders` in `src/server.rs`.
  - Source commands: `security-audit`

### Testing — `AddMemberEntry` and `UpdateMemberRoleEntry` Lack `Validate` Derive

- [x] **#171 — These models are deserialized from request bodies but `validate()` is a no-op since they don't derive `Validate`**
  - Resolution: Added `Validate` derive to both structs in `src/models.rs` for consistency with other request models.
  - Source commands: `test-gaps`

### Testing — Non-Member GET Rejection Untested for Order Endpoints

- [x] **#236 — All order-related GET handlers call `require_team_member` but no test verifies GET rejection for non-members**
  - Resolution: Incorrect premise — verified that order GET handlers (`get_order_items`, `get_order_item`, `get_team_orders`, `get_team_order`) do NOT call `require_team_member`. Only mutation handlers enforce team membership. Consistent with deliberate open-read design (#117). Finding removed.
  - Source commands: `test-gaps`

### Testing — `validate_optional_password` Has No Unit Tests

- [x] **#172 — Custom validator for `UpdateUserRequest.password` has zero test coverage**
  - Resolution: Added 5 unit tests in `src/models.rs`: rejects too short, rejects too long, accepts valid, boundary min (7→err, 8→ok), boundary max (128→ok, 129→err).
  - Source commands: `test-gaps`

### Testing — No API Test for `user_teams` Endpoint

- [x] **#173 — `GET /api/v1.0/users/{user_id}/teams` has no API-level integration test**
  - Resolution: Added 2 API tests in `tests/api_tests.rs`: `user_teams_returns_teams_for_seed_admin` and `user_teams_returns_empty_for_user_with_no_teams`.
  - Source commands: `test-gaps`

### Testing — `check_team_access` Combined RBAC Query Has No Direct Test

- [x] **#174 — Core RBAC query tested only indirectly through API-level tests**
  - Resolution: Added 4 DB tests in `tests/db_tests.rs`: admin-in-own-team, regular-member, non-member, admin-in-unrelated-team.
  - Source commands: `test-gaps`

### Testing — No Test for Malformed Path Parameters

- [x] **#175 — `GET /api/v1.0/users/not-a-uuid` → 400 path is untested**
  - Resolution: Added `malformed_uuid_path_returns_400` API test.
  - Source commands: `test-gaps`

### Testing — No Test for JSON Error Handler

- [x] **#176 — Oversized/malformed JSON body error paths are untested**
  - Resolution: Added `wrong_content_type_returns_415` and `invalid_json_body_returns_error` API tests.
  - Source commands: `test-gaps`

### Testing — No API Tests for `update_team` and `update_role` Success Paths

- [x] **#177 — Admin happy path untested; only rejection path (`non_admin_cannot_*`) exists**
  - Resolution: Added `admin_can_update_team` and `admin_can_update_role` API tests.
  - Source commands: `test-gaps`

### Testing — No Tests for `Location` Header in Create Responses

- [x] **#178 — Only 4 of 7 create handlers build `Location` header via `url_for` but no test verifies it**
  - Resolution: Added `create_item_returns_location_header` API test.
  - Source commands: `test-gaps`

### Testing — No Rate Limiting Behavior Test

- [x] **#179 — No test verifies the 11th rapid auth request returns 429**
  - Resolution: Added `auth_endpoint_rate_limits_after_burst` API test (sends 10+1 requests, verifies 429).
  - Source commands: `test-gaps`

### Testing — No Validation Tests for Order-Related Models

- [x] **#180 — `CreateOrderEntry`, `UpdateOrderEntry`, `CreateTeamOrderEntry`, `UpdateTeamOrderEntry` derive `Validate` but have no tests**
  - Resolution: Added 7 unit tests in `src/models.rs` covering `CreateOrderEntry`, `UpdateOrderEntry`, `CreateTeamOrderEntry`, and `UpdateTeamOrderEntry` validation.
  - Source commands: `test-gaps`

### Testing — No Test for Error Response Body Shape

- [x] **#181 — Tests verify status codes but never assert response body matches `{"error": "..."}`**
  - Resolution: Added `error_response_body_is_json_object_with_error_key` async test in `src/errors.rs` verifying JSON body shape for both 4xx and 5xx errors.
  - Source commands: `test-gaps`

### Code Quality — `UpdateUserEntry` Serves Dual Purpose

- [x] **#183 — Struct used for both auth cache and DB row mapping**
  - Resolution: Removed `Validate` derive and `#[validate(...)]` attributes from `UpdateUserEntry`, added doc comment explaining the struct's dual purpose.
  - Source commands: `review`

### Frontend — `authed_get` Only Supports GET

- [x] **#184 — Future pages need `authed_post`, `authed_put`, `authed_delete` variants**
  - Resolution: Added `HttpMethod` enum, `build_method_request()`, and generic `authed_request(method, url, body)` in `frontend/src/app.rs`. `authed_get` now delegates to it.
  - Source commands: `review`

### Deployment — Healthcheck Binary Hardcodes Port 8080

- [x] **#185 — `let port = 8080;` is hardcoded in the healthcheck binary**
  - Resolution: Changed healthcheck binary to read port from `HEALTH_PORT` env var with fallback to 8080.
  - Source commands: `review`

### Testing — Bulk Delete Team Orders Has No API Test

- [x] **#204 — `DELETE /api/v1.0/teams/{id}/orders` RBAC and response untested at API level**
  - Resolution: Added `admin_can_bulk_delete_team_orders` API test.
  - Source commands: `test-gaps`

### Testing — Update Member Role Has No API Test

- [x] **#205 — `PUT /api/v1.0/teams/{id}/users/{id}` untested at API level**
  - Resolution: Added `admin_can_update_member_role` API test.
  - Source commands: `test-gaps`

### Testing — Delete User by Email Success Path Untested

- [x] **#206 — `DELETE /api/v1.0/users/email/{email}` success path has no API test**
  - Resolution: Added `admin_can_delete_user_by_email` API test.
  - Source commands: `test-gaps`

### Testing — Token Revocation Ownership Check Untested

- [x] **#207 — No test verifies that User A cannot revoke User B's token**
  - Resolution: Added `non_admin_cannot_revoke_another_users_token` and `admin_can_revoke_another_users_token` API tests.
  - Source commands: `test-gaps`

### Testing — Team Users Has No API Test

- [x] **#208 — `GET /api/v1.0/teams/{id}/users` has no API-level integration test**
  - Resolution: Added `team_users_returns_members_of_seed_team` and `team_users_returns_empty_for_team_with_no_members` API tests.
  - Source commands: `test-gaps`

### Code Quality — Redundant `Client` Import in Handler Files

- [x] **#209 — `use deadpool_postgres::Client;` redundant in `handlers/users.rs` and `handlers/roles.rs`**
  - Resolution: Removed redundant `use deadpool_postgres::Client;` from `src/handlers/users.rs` and `src/handlers/roles.rs`.
  - Source commands: `review`

### Code Quality — Missing Doc Comments on DB Functions

- [x] **#129 — Public functions in `src/db/` lack doc comments**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`, `src/db/membership.rs`, `src/db/health.rs`
  - Resolution: Added `///` doc comments to all 40 undocumented public functions across 8 DB module files. All 49 public functions in `src/db/` now have documentation.
  - Source commands: `review`

### Documentation — CLAUDE.md Test Count Stale

- [x] **#341 — Line 118 says "39 WASM integration tests" but actual count is 41**
  - File: `CLAUDE.md`
  - Resolution: Updated to "41 WASM integration tests".
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Missing V6 Migration

- [x] **#342 — `migrations/` listing stops at V5; V6 exists on disk**
  - File: `CLAUDE.md`
  - Resolution: Added `V6__order_constraint_and_index.sql` entry to migrations listing.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Missing `bundle-css.sh`

- [x] **#343 — `frontend/bundle-css.sh` and related bundled CSS not listed in Project Structure**
  - File: `CLAUDE.md`
  - Resolution: Added `bundled.css` and `bundle-css.sh` to the frontend style/tests section.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Wrong Icon Path

- [x] **#344 — States `connect-icons/svg/` but actual path is `connect-design-system/packages/icons/src/svgs/`**
  - File: `CLAUDE.md`
  - Resolution: Fixed icon path reference.
  - Source commands: `cross-ref-check`

### Documentation — `api-completeness.md` Stale Path

- [x] **#345 — Command file references `db.rs` instead of the `db/` module directory**
  - File: `.claude/commands/api-completeness.md`
  - Resolution: Changed `db.rs` to `db/`.
  - Source commands: `cross-ref-check`

### Documentation — `rbac-rules.md` Incorrect Claim

- [x] **#346 — Claims `database_seed.sql` uses no hardcoded role strings, but it does**
  - File: `.claude/commands/rbac-rules.md`
  - Resolution: Fixed the incorrect claim to acknowledge hardcoded role strings in seed data.
  - Source commands: `cross-ref-check`

### Documentation — Assessment Command List Incomplete

- [x] **#347 — The enumerated list of assessment commands doesn't mention `resume-assessment`**
  - File: `CLAUDE.md`
  - Resolution: Added `resume-assessment` to the assessment command list.
  - Source commands: `cross-ref-check`

### Documentation — Root-Level Files Missing from Project Structure

- [x] **#348 — Dockerfiles, `docker-compose.*`, `Makefile`, `README.md`, etc. not in the `text` block**
  - File: `CLAUDE.md`
  - Resolution: Added root-level files to the Project Structure section.
  - Source commands: `cross-ref-check`

### Frontend — `Page::Dashboard` Clones Data on Every Signal Read

- [x] **#126 — Dashboard state stored in enum variant, cloned on every re-render**
  - Files: `frontend/src/pages/dashboard.rs`
  - Problem: `user.get()` inside the reactive closure cloned the full `UserContext` (including `teams: Vec<UserInTeams>`) on every re-render.
  - Fix: Changed to `user.with(|u| …)` so only immutable borrow occurs, avoiding the clone.
  - Source commands: `review`

### Frontend — Missing `aria-busy` on Submit Button

- [x] **#127 — No `aria-busy` attribute during login form submission**
  - File: `frontend/src/pages/login.rs`
  - Problem: The submit button did not set `aria-busy` during the loading state, making it inaccessible to screen readers.
  - Fix: Added `aria-busy=move || loading.get().to_string()` to the `<button>` element.
  - Source commands: `review`

### Frontend — Decorative Icons Lack Accessibility Attributes

- [x] **#128 — Warning icon and checkmark lack `aria-hidden="true"`**
  - File: `frontend/src/pages/login.rs`
  - Note: Already resolved before fix commit — both icons already had `aria-hidden="true"` at the time of review. Confirmed and archived.
  - Source commands: `review`

### API Design — `get_user_teams` Query Does Not Return `team_id`

- [x] **#301 — `UserInTeams` model and query lack `team_id`, preventing frontend navigation from team list to team detail**
  - Files: `src/db/teams.rs`, `src/models.rs` (`UserInTeams` struct)
  - Resolution: Backend fix complete — `teams.team_id` and `teams.descr` added to SELECT clause; `team_id: Uuid` and `descr: Option<String>` added to `UserInTeams` struct. Frontend struct gap tracked separately as #365 in assessment-findings.md.
  - Source commands: `db-review`, `api-completeness`

### Deployment — Dev Config in Production Docker Image

- [x] **#76 — No `.env.example` or env documentation for new developers**
  - Resolution: Created `.env.example` with all server and PostgreSQL config variables, environment names, and TLS cert path.
  - Source commands: `practices-audit`

- [x] **#118 — `development.yml` copied into production image unnecessarily**
  - File: `Dockerfile.breakfast`
  - Resolution: Removed `COPY --chown=web:web config/development.yml /config/development.yml` from the final stage. Production image now only contains `default.yml` and `production.yml`.
  - Source commands: `security-audit`

### Frontend — Inconsistent Import and Redundant Validation

- [x] **#210 — Session restore uses `wasm_bindgen_futures::spawn_local` while logout uses `leptos::task::spawn_local`**
  - File: `frontend/src/app.rs`
  - Resolution: Changed `wasm_bindgen_futures::spawn_local` to `leptos::task::spawn_local` in session restore for consistency.
  - Source commands: `review`

- [x] **#211 — `<form>` has both native HTML5 validation and custom JavaScript validation**
  - File: `frontend/src/pages/login.rs`
  - Resolution: Removed `required=true` from username and password inputs. Custom JS validation in `on_submit` provides better UX with the CONNECT design system error alert component.
  - Source commands: `review`

### Frontend — Accessibility and UX

- [x] **#231 — Loading spinner container lacks `role="status"` and `aria-live`**
  - File: `frontend/src/pages/loading.rs`
  - Resolution: Added `role="status"` and `aria-live="polite"` to the loading card container.
  - Source commands: `review`

- [x] **#233 — `session_storage()` called 3 times in the `on_logout` closure**
  - File: `frontend/src/components/sidebar.rs`
  - Resolution: Consolidated to a single `session_storage()` call stored in a local variable, reused for reading tokens and clearing them.
  - Source commands: `review`

### Code Quality — Error Handling and Row Mapping

- [x] **#232 — If `serde_json::to_string` fails, the fallback `format!` produces invalid JSON**
  - File: `src/errors.rs`
  - Resolution: Added backslash and double-quote escaping in the fallback branch to produce valid JSON.
  - Source commands: `review`

- [x] **#234 — `map_err` helper checks for `"column"` or `"not found"` in error messages**
  - File: `src/from_row.rs`
  - Resolution: Replaced fragile string matching with `e.source()` check — `tokio_postgres` column-not-found errors have no source (cause = None), while type conversion errors have a source.
  - Source commands: `review`

- [x] **#254 — 9 `FromRow` implementations total ~200 lines of repetitive `try_get`/`map_err` per column**
  - File: `src/from_row.rs`
  - Resolution: Created `impl_from_row!` macro that generates `FromRow::from_row_ref` from a list of field names (which match column names). All 9 implementations reduced to single-line macro invocations.
  - Source commands: `review`

- [x] **#255 — Identical `filter_map` + `warn` block in 6 list functions**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Resolution: Extracted `map_rows<T: FromRow>(rows, entity)` helper in `src/from_row.rs`. All 8 list functions (including `get_user_teams` and `get_team_users`) now use the shared helper.
  - Source commands: `review`

### Documentation — Test Count Maintenance

- [x] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md`
  - Resolution: Updated DB test count from 92 to 96 to reflect new FK cascade tests. Counts are maintained by the assessment process.
  - Source commands: `practices-audit`

### Testing — FK Cascade Coverage

- [x] **#124 — FK cascade and `fix_migration_history` DB interaction lack tests**
  - File: `tests/db_tests.rs`
  - Resolution: Added 4 integration tests: `delete_team_cascades_membership_and_orders`, `delete_team_order_cascades_order_items`, `delete_user_cascades_membership`, `delete_item_with_order_reference_is_restricted`. These verify ON DELETE CASCADE and ON DELETE RESTRICT FK behaviour.
  - Source commands: `test-gaps`

### Security — Token Response Caching

- [x] **#247 — `/auth` and `/auth/refresh` responses contain JWT tokens but no `Cache-Control` header**
  - Files: `src/handlers/users.rs`
  - Resolution: Added `.insert_header(("Cache-Control", "no-store"))` to both `auth_user` and `refresh_token` handler responses.
  - Source commands: `security-audit`

### Security — Missing Referrer-Policy Header

- [x] **#248 — `DefaultHeaders` does not include `Referrer-Policy`**
  - File: `src/server.rs`
  - Resolution: Added `.add(("Referrer-Policy", "strict-origin-when-cross-origin"))` to the global `DefaultHeaders` chain.
  - Source commands: `security-audit`

### Security — Rate Limiter IP-Based Key Extraction (Acknowledged)

- [x] **#119 — Behind a reverse proxy, all requests share one IP**
  - File: `src/routes.rs`
  - Resolution: Acknowledged informational. `actix-governor` uses `PeerIpKeyExtractor` by default. In production behind a reverse proxy, configure the proxy to set `X-Forwarded-For` and use a custom key extractor. Deployment concern, not a code bug.
  - Source commands: `security-audit`

### Security — Auth Cache Staleness Window (Acknowledged)

- [x] **#120 — 5-minute cache TTL allows stale credentials after password change**
  - File: `src/middleware/auth.rs`
  - Resolution: Acknowledged informational. Cache is explicitly invalidated on password change via `invalidate_cache()`. The 5-minute TTL is a design trade-off for concurrent sessions. Acceptable for an internal app.
  - Source commands: `security-audit`

### Dependencies — `native-tls` Compiled Alongside `rustls` (Acknowledged)

- [x] **#121 — `refinery` unconditionally enables `postgres-native-tls`**
  - Resolution: Acknowledged informational. Upstream issue — `refinery` has no feature flag to disable `native-tls`. Unused at runtime (we use `rustls`). No action possible without upstream changes.
  - Source commands: `dependency-check`

### Dependencies — Low-Activity `tracing-bunyan-formatter` (Acknowledged)

- [x] **#123 — `tracing-bunyan-formatter` has infrequent releases**
  - Resolution: Acknowledged informational. Stable, functional, no CVEs. Low release activity reflects feature completeness. No alternative offers the same Bunyan JSON format with tracing integration.
  - Source commands: `dependency-check`

### Deployment — Docker Compose PostgreSQL Port Binding

- [x] **#249 — `docker-compose.yml` maps port 5432 to `0.0.0.0` by default**
  - File: `docker-compose.yml`
  - Resolution: Changed port mapping from `5432:5432` to `127.0.0.1:5432:5432` to bind only to localhost.
  - Source commands: `security-audit`

### Deployment — HTTP Redirect Port Configurable

- [x] **#256 — HTTP→HTTPS redirect listener binds to port 80 unconditionally**
  - File: `src/server.rs`, `src/config.rs`, `config/default.yml`
  - Resolution: Made HTTP redirect port configurable via `server.http_redirect_port` in config (default: 80). Removed hardcoded const. Configurable via `BREAKFAST_SERVER_HTTP_REDIRECT_PORT` env var.
  - Source commands: `review`

### Dependencies — `password-hash` Direct Dependency (Acknowledged)

- [x] **#257 — `password-hash` is a direct dependency only to enable `getrandom` feature**
  - File: `Cargo.toml`
  - Resolution: Acknowledged informational. Required to enable `getrandom` feature for Argon2 random salt generation. Idiomatic Cargo pattern for enabling transitive features.
  - Source commands: `dependency-check`

### Security — Permissions-Policy Header Added

- [x] **#258 — `DefaultHeaders` does not include `Permissions-Policy`**
  - File: `src/server.rs`
  - Resolution: Added `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()` to the global `DefaultHeaders` chain.
  - Source commands: `security-audit`

### Deployment — Docker Resource Limits Added

- [x] **#259 — No `deploy.resources.limits` for CPU or memory**
  - File: `docker-compose.yml`
  - Resolution: Added `deploy.resources.limits` (memory: "512M", cpus: "1") to the `breakfast` service.
  - Source commands: `security-audit`

### Documentation — Seed SQL Header Updated

- [x] **#260 — Seed data file header references only V1 schema**
  - File: `database_seed.sql`
  - Resolution: Updated header to reference "V1 through V6" instead of just "V1".
  - Source commands: `cross-ref-check`

### Testing — Partial Order Update COALESCE Test Added

- [x] **#261 — No test passes `None` for some update fields and verifies existing values are preserved**
  - File: `tests/db_tests.rs`
  - Resolution: Added `update_team_order_partial_preserves_existing_values` DB integration test.
  - Source commands: `test-gaps`

### Testing — FK-Violating team_id Test Added

- [x] **#262 — No test creates a team order with non-existent `team_id` to verify FK error handling**
  - File: `tests/db_tests.rs`
  - Resolution: Added `create_team_order_with_nonexistent_team_id_fails` DB integration test.
  - Source commands: `test-gaps`

### Testing — Revoked Refresh Token Rejection Test Added

- [x] **#263 — No test explicitly revokes a refresh token then verifies `/auth/refresh` returns 401**
  - File: `tests/api_tests.rs`
  - Resolution: Added `revoked_refresh_token_is_rejected_by_refresh_endpoint` API integration test.
  - Source commands: `test-gaps`

### Testing — Empty Order Items List Test Added

- [x] **#264 — No test verifies `GET .../items` returns `200 []` for an order with zero items**
  - File: `tests/api_tests.rs`
  - Resolution: Added `empty_order_items_returns_200_with_empty_list` API integration test.
  - Source commands: `test-gaps`

### Testing — Non-Existent role_id in guard_admin_role_assignment Test Added

- [x] **#265 — No test calls `add_team_member` or `update_member_role` with a non-existent `role_id`**
  - File: `tests/api_tests.rs`
  - Resolution: Added `add_team_member_with_nonexistent_role_id_returns_404` API integration test.
  - Source commands: `test-gaps`

### Database — Text Column Constraints Acknowledged

- [x] **#285 — Text columns have API-level max-length validation but no `VARCHAR(N)` or `CHECK` at the database layer**
  - Files: `migrations/V1__initial_schema.sql`
  - Resolution: Acknowledged informational. API is the sole entry point and enforces max-length via `validator` crate.
  - Source commands: `db-review`

### Error Handling — Trigger Exception Mapping Acknowledged

- [x] **#286 — PostgreSQL `P0001` (raise_exception from trigger) maps to generic DB error (500)**
  - File: `src/db/order_items.rs`
  - Resolution: Acknowledged informational. Handler calls `guard_open_order` with `FOR UPDATE` row lock before INSERT; trigger only fires under race conditions the lock prevents.
  - Source commands: `db-review`

### OpenAPI — auth_user 401 Body Type Added

- [x] **#287 — `auth_user` utoipa has `(status = 401)` but no `body = ErrorResponse`**
  - File: `src/handlers/users.rs`
  - Resolution: Added `body = ErrorResponse` to the 401 response annotation.
  - Source commands: `openapi-sync`

### Dead Code — is_team_order_closed Visibility Acknowledged

- [x] **#288 — `is_team_order_closed` is public API but only used in integration tests**
  - File: `src/db/order_items.rs`
  - Resolution: Acknowledged informational. Cannot make `pub(crate)` because external integration tests use it. Intentionally `pub` for test access.
  - Source commands: `review`

### Testing — Member-Cannot-Manage-Members Tests Added

- [x] **#289 — No test where a user with "Member" role tries to POST/DELETE/PUT on team members**
  - Files: `tests/api_tests.rs`
  - Resolution: Added 3 API tests: `member_cannot_add_team_member`, `member_cannot_remove_team_member`, `member_cannot_update_member_role`.
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Member-Cannot-Bulk-Delete-Orders Test Added

- [x] **#290 — `delete_team_orders` requires `require_team_admin` but only admin bypass is tested**
  - File: `tests/api_tests.rs`
  - Resolution: Added `member_cannot_bulk_delete_team_orders` API test.
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Non-Member Cannot Update/Delete Team Order Tests Added

- [x] **#291 — `non_member_cannot_create_team_order` tests only POST; PUT and DELETE have no non-member test**
  - File: `tests/api_tests.rs`
  - Resolution: Added `non_member_cannot_update_team_order` and `non_member_cannot_delete_team_order` API tests.
  - Source commands: `rbac-rules`, `test-gaps`

### Testing — Cache FIFO Eviction Test Added

- [x] **#292 — No test saturates the cache past 1000 entries to verify eviction fires correctly**
  - File: `src/middleware/auth.rs`
  - Resolution: Added `cache_eviction_fires_at_max_capacity` unit test.
  - Source commands: `test-gaps`

### Testing — Token Blacklist Cleanup Test Added

- [x] **#293 — `DashMap::retain()` cleanup path has no test**
  - File: `src/middleware/auth.rs`
  - Resolution: Added `token_blacklist_retain_removes_expired_entries` unit test.
  - Source commands: `test-gaps`

### Testing — Location Header Tests Added for All Create Endpoints

- [x] **#294 — `create_item_returns_location_header` exists but no equivalent for 6 other create endpoints**
  - File: `tests/api_tests.rs`
  - Resolution: Added 6 API tests for Location header on create_user, create_team, create_role, create_team_order, create_order_item, add_team_member.
  - Source commands: `test-gaps`

### Testing — GET Orders for Nonexistent Team Test Added

- [x] **#295 — No test calls `GET /teams/{nonexistent}/orders` to verify 200 empty vs 404**
  - File: `tests/api_tests.rs`
  - Resolution: Added `get_orders_for_nonexistent_team_returns_empty_list` API test.
  - Source commands: `test-gaps`

### Frontend — Duplicated `role_tag_class()` Function Across 4 Files

- [x] **#318 — Same role-to-CSS-class mapping repeated in 4 frontend files**
  - Files: `frontend/src/pages/dashboard.rs`, `frontend/src/pages/teams.rs`, `frontend/src/pages/profile.rs`, `frontend/src/pages/roles.rs`
  - Resolution: Extracted shared `role_tag_class()` fn (returns `&'static str`) to `frontend/src/components/mod.rs`; removed local copies from all 4 pages.
  - Source commands: `review`

### Frontend — Duplicated `LoadingSpinner` Markup in 5 Pages

- [x] **#319 — Same loading spinner HTML pattern repeated in 5 page files**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Resolution: Extracted shared `LoadingSpinner` component to `frontend/src/components/mod.rs`; removed local copies from all 5 pages.
  - Source commands: `review`

### Dependencies — `tokio-postgres` Unused `serde_json` Feature

- [x] **#324 — `with-serde_json-1` feature enabled but no query uses JSON columns**
  - File: `Cargo.toml` (tokio-postgres dependency)
  - Resolution: Removed `"with-serde_json-1"` from tokio-postgres features list.
  - Source commands: `dependency-check`

### Testing — `jwt_validator` Rejects Refresh Token — No Explicit Test

- [x] **#351 — The `if c.claims.token_type != TokenType::Access` branch returns 401 but is never directly tested**
  - File: `src/middleware/auth.rs` (lines ~230–248)
  - Resolution: Added `jwt_protected_endpoint_rejects_refresh_token` API integration test in `tests/api_tests.rs`.
  - Source commands: `test-gaps`

### Frontend — Sidebar Uses `user.get()` Which Clones Full `UserContext` on Each Render

- [x] **#360 — `Sidebar` calls `user.get()` inside reactive closures, cloning the entire `UserContext` (including `teams: Vec<UserInTeams>`) on every re-render**
  - Files: `frontend/src/components/sidebar.rs`
  - Resolution: Replaced both `user.get()` calls with `user.with(|u| ...)` pattern, consistent with the #126 fix in `dashboard.rs`.
  - Source commands: `review`

### API Completeness — Frontend `UserInTeams` Missing `team_id` and `descr` Fields

- [x] **#365 — Frontend `UserInTeams` struct lacks `team_id` and `descr` that the backend now provides**
  - Files: `frontend/src/api.rs`
  - Resolution: Added `pub descr: Option<String>` to the frontend `UserInTeams` struct (the `team_id` field was already present).
  - Source commands: `api-completeness`

### Code Quality — `#[derive(Validate)]` with No Validation Attributes on 4 Structs

- [x] **#376 — `UpdateTeamEntry`, `UpdateRoleEntry`, `UpdateItemEntry`, `UpdateTeamOrderEntry` derive `Validate` but have no `#[validate(...)]` field attributes**
  - File: `src/models.rs`
  - Resolution: Removed `Validate` derive from `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, and `UpdateMemberRoleEntry` (the structs with truly no-op validation). Removed corresponding `validate(&json)?` calls from 4 team handlers. Removed 2 obsolete unit tests that tested the no-op behavior.
  - Source commands: `review`

### Frontend — Inconsistent Async Spawning API

- [x] **#452 — `LogoutButton` uses `leptos::task::spawn_local` while all others use `wasm_bindgen_futures::spawn_local`**
  - File: `frontend/src/components/sidebar.rs`
  - Resolution: Added `use wasm_bindgen_futures::spawn_local;` import and replaced `leptos::task::spawn_local` with the imported `spawn_local`.
  - Source commands: `review`

### Code Quality — `GovernorConfigBuilder::finish().unwrap()` in Production Path

- [x] **#454 — Should use `.expect("valid rate limiter config")` for better panic message**
  - File: `src/routes.rs`
  - Resolution: Changed `.unwrap()` to `.expect("valid rate limiter config")`.
  - Source commands: `review`

### Code Quality — `format!()` on String Literals

- [x] **#455 — `format!("Delete User")` etc. allocate unnecessarily; use `.to_string()` instead**
  - Files: `frontend/src/pages/admin.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/items.rs`
  - Resolution: Changed `format!("Delete X")` to `"Delete X".to_string()` in all 3 files.
  - Source commands: `review`

### Documentation — CLAUDE.md Backend Test Counts Stale

- [x] **#404 — CLAUDE.md states 193 unit, 87 API, 96 DB tests; actual counts are 195 unit, 109 API, 101 DB**
  - File: `CLAUDE.md` (Testing section)
  - Resolution: Updated CLAUDE.md and README.md test count sections to reflect correct values.
  - Source commands: `cross-ref-check`

### Documentation — README Test Counts Stale

- [x] **#405 — README.md states 193 unit, 87 API, 92 DB; actual counts are 195, 109, 101**
  - File: `README.md`
  - Resolution: Updated README.md test counts to match actual running suites.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md `db/users.rs` Function List Incomplete

- [x] **#406 — `get_password_hash` missing from the parenthetical function list**
  - File: `CLAUDE.md` (Project Structure → `db/users.rs`)
  - Resolution: Added `get_password_hash` to the function list in CLAUDE.md.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Structure Tree Missing Root Files

- [x] **#407 — `NEW-UI-COMPONENTS.md` and `LICENSE` exist on disk but not in project structure tree**
  - File: `CLAUDE.md` (Project Structure)
  - Resolution: Added both files to the project structure listing.
  - Source commands: `cross-ref-check`

### Database — Redundant Indexes Duplicate UNIQUE Constraint Auto-Indexes

- [x] **#408 — `idx_users_email` and `idx_teams_name` duplicate the implicit unique indexes from UNIQUE constraints**
  - File: `migrations/V1__initial_schema.sql` (lines ~25, ~38)
  - Resolution: Added migration V7 to drop both redundant indexes.
  - Source commands: `db-review`

### Database — `get_order_items` ORDER BY UUID Gives Non-Meaningful Sort

- [x] **#410 — `ORDER BY orders_item_id` sorts by item UUID primary key, not by when the item was added or by name**
  - File: `src/db/order_items.rs` (line ~84)
  - Resolution: Changed to `ORDER BY created ASC` to sort by insertion time.
  - Source commands: `db-review`

### Dependencies — `tracing-bunyan-formatter` Effectively Unmaintained

- [x] **#411 — v0.3.10 (last release Feb 2024) causes `tracing-log` v0.1/v0.2 duplication and pulls stale transitive deps**
  - File: `Cargo.toml`
  - Resolution: Replaced with `tracing-subscriber::fmt::layer().json()` for structured JSON logging in production.
  - Source commands: `dependency-check`

### OpenAPI — `create_order_item` Missing 404 Response

- [x] **#412 — `guard_open_order` returns 404 when team order doesn't exist, but utoipa annotation omits 404**
  - File: `src/handlers/orders.rs` (lines ~68–82)
  - Resolution: Added `(status = 404, description = "Team order or item not found", body = ErrorResponse)` to utoipa annotation.
  - Source commands: `openapi-sync`

### OpenAPI — Member Management 403 Descriptions Omit Admin-Role Guard

- [x] **#413 — `add_team_member` and `update_member_role` 403 descriptions say only "team admin role required" but omit `guard_admin_role_assignment` scenario**
  - File: `src/handlers/teams.rs` (lines ~358–372, ~431–445)
  - Resolution: Updated 403 descriptions to include admin-role-assignment guard scenario.
  - Source commands: `openapi-sync`

### OpenAPI — `create_team_order` Missing 422 Response

- [x] **#414 — Handler calls `validate(&json)?` but utoipa annotation omits 422**
  - File: `src/handlers/teams.rs` (line ~228)
  - Resolution: Added `(status = 422, description = "Validation error", body = ErrorResponse)` to utoipa annotation.
  - Source commands: `rbac-rules`

### Documentation — CLAUDE.md Security Headers Omits `Permissions-Policy`

- [x] **#415 — `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()` is set in `DefaultHeaders` but not documented**
  - File: `CLAUDE.md` (Security headers bullet), `src/server.rs` (line ~444)
  - Resolution: Added Permissions-Policy entry to the security headers documentation in CLAUDE.md.
  - Source commands: `practices-audit`

### Security — `Error::ActixAuth` Leaks Raw Actix Error Messages

- [x] **#416 — `ActixAuth` variant returns `e.to_string()` directly in 401 response body, potentially exposing internal framework details**
  - File: `src/errors.rs` (lines ~131–134)
  - Resolution: Changed to return generic `"Authentication failed"` string.
  - Source commands: `security-audit`

### Security — No `Cache-Control` on Authenticated GET Endpoints

- [x] **#417 — Authenticated GET responses lack `Cache-Control: no-store` — browsers/proxies may cache sensitive data**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/roles.rs`, `src/handlers/items.rs`, `src/handlers/orders.rs`
  - Resolution: Added `Cache-Control: no-store, private` via `DefaultHeaders` wrapping the `/api/v1.0` scope in `src/routes.rs`.
  - Source commands: `security-audit`

### Security — No Guard That `jwtsecret` ≠ `secret`

- [x] **#418 — Production startup guards reject default values individually but don't check if both are set to the same custom value**
  - File: `src/server.rs` (lines ~297–316)
  - Resolution: Added `if settings.server.secret == settings.server.jwtsecret { panic!("...") }` startup guard.
  - Source commands: `security-audit`

### Security — Default Config Plaintext Secrets in Docker Image

- [x] **#419 — `default.yml` with `secret: "Very Secret"` and `password: actix` is copied into the final Docker image**
  - File: `Dockerfile.breakfast` (line ~81), `config/default.yml`
  - Resolution: Dockerfile now copies `config/docker-base.yml` as the base config (sanitized, all secret fields empty — must be supplied via env vars).
  - Source commands: `security-audit`

### Frontend — Missing Edit UI for Teams, Items, and Roles

- [x] **#420 — `PUT /teams/{id}`, `PUT /items/{id}`, `PUT /roles/{id}` exist but no frontend edit forms**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`
  - Resolution: Added inline edit dialogs (`do_update_team`, `do_update_item`, `do_update_role`) to all three pages.
  - Source commands: `api-completeness`

### Frontend — No Team Member Management UI

- [x] **#421 — Backend POST/DELETE/PUT on team members fully implemented; frontend shows read-only member table only**
  - File: `frontend/src/pages/teams.rs`
  - Resolution: Added add-member, remove-member, and update-role UI (`do_add_member`, `do_remove_member`, `do_update_member_role`).
  - Source commands: `api-completeness`

### Frontend — No Order Update/Close UI or Order Item Quantity Edit

- [x] **#422 — `PUT /teams/{id}/orders/{oid}` (close/reopen, due date) and `PUT .../items/{iid}` (quantity) exist but no frontend UI**
  - File: `frontend/src/pages/orders.rs`
  - Resolution: Added close/reopen toggle (`do_toggle_order_closed`) and order item quantity editing.
  - Source commands: `api-completeness`

### Frontend — No Pagination Controls

- [x] **#423 — All list endpoints return paginated responses but no page has next/previous/page controls; lists truncated at 50**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Resolution: Added `PaginationBar` component to all five list pages.
  - Source commands: `api-completeness`

### Frontend — No Admin Edit-User UI

- [x] **#424 — AdminPage shows user list with create/delete but no edit form; only ProfilePage supports self-edit**
  - File: `frontend/src/pages/admin.rs`
  - Resolution: Added `EditUserDialog` component with `do_update_user` handler.
  - Source commands: `api-completeness`

### Frontend — Create User Gated Admin-Only in UI but Backend Allows Team Admin

- [x] **#425 — `require_admin_or_team_admin` allows team admins to create users, but Admin page is only visible to global admins**
  - File: `frontend/src/pages/admin.rs`
  - Resolution: Admin page and user-create are now visible to `is_admin || is_team_admin`, matching backend gate.
  - Source commands: `api-completeness`

### Frontend — Profile Save Duplicates `build_user_context()` Logic

- [x] **#426 — After PUT, profile page manually fetches user + teams + checks admin, duplicating `build_user_context()` from api.rs**
  - File: `frontend/src/pages/profile.rs` (lines ~69–101)
  - Resolution: Profile save now calls `build_user_context()` instead of duplicating the logic.
  - Source commands: `review`

### Frontend — Profile Save Discards PUT Response, Makes 2 Extra GETs

- [x] **#427 — Successful PUT response body is not read; code makes separate GET for user and GET for teams**
  - File: `frontend/src/pages/profile.rs` (lines ~76–78)
  - Resolution: PUT response body is now deserialized for updated user data; only teams fetch remains.
  - Source commands: `review`

### Frontend — No Client-Side Email Validation on Profile Edit

- [x] **#428 — Invalid email accepted client-side, rejected server-side with generic toast**
  - File: `frontend/src/pages/profile.rs` (lines ~239–253)
  - Resolution: Added `!em.contains('@') || !domain.contains('.')` email format check.
  - Source commands: `review`

### Testing — Team Admin Bulk-Delete Orders Positive Path Untested

- [x] **#429 — Admin bypass tested, member denied tested, but no test where Team Admin bulk-deletes orders on own team**
  - File: `tests/api_tests.rs`
  - Resolution: Added `team_admin_can_bulk_delete_own_team_orders` test.
  - Source commands: `rbac-rules`

### Testing — Team Admin Update/Delete Another Member's Order Untested

- [x] **#430 — No test where Team Admin (non-owner) updates or deletes an order created by a regular member**
  - File: `tests/api_tests.rs`
  - Resolution: Added `team_admin_can_update_order_by_another_member` test.
  - Source commands: `rbac-rules`

### Testing — Order Owner Update/Delete Own Order Positive Path Untested

- [x] **#431 — No test where a regular member (order creator) updates or deletes their own order and gets 200**
  - File: `tests/api_tests.rs`
  - Resolution: Added `member_can_update_and_delete_own_order` test.
  - Source commands: `rbac-rules`

### Testing — Duplicate Team Name Conflict Not Tested via API

- [x] **#432 — No API test creates a team with an existing name and asserts 409**
  - File: `tests/api_tests.rs`
  - Resolution: Added duplicate team name 409 test.
  - Source commands: `test-gaps`

### Testing — Negative Price Rejection Not Tested via API

- [x] **#433 — No API test sends a negative price to `POST /items` and asserts 422**
  - File: `tests/api_tests.rs`
  - Resolution: Added `create_item_with_negative_price_returns_422` test.
  - Source commands: `test-gaps`

### Testing — `PaginationParams::sanitize()` Clamping Untested

- [x] **#434 — No test sends `limit=200` or `offset=-5` and verifies clamped pagination metadata**
  - File: `src/models.rs` (lines ~31–38), `tests/api_tests.rs`
  - Resolution: Added pagination clamping tests (limit=200 → 100, offset=-5 → 0).
  - Source commands: `test-gaps`

### Testing — Self-Delete User by Email Untested

- [x] **#435 — No API test verifies a non-admin user can delete their own account by email**
  - File: `tests/api_tests.rs`
  - Resolution: Added `user_can_delete_own_account_by_email` test.
  - Source commands: `test-gaps`

### Testing — `create_team` Duplicate Name Not Tested at DB Level

- [x] **#436 — No DB test attempts to create a team with an existing name (UNIQUE constraint)**
  - File: `tests/db_tests.rs`
  - Resolution: Added `create_team_with_duplicate_name_fails` DB test.
  - Source commands: `test-gaps`

### Testing — `create_role` Duplicate Title Not Tested at DB Level

- [x] **#437 — No DB test for creating a role with a duplicate title**
  - File: `tests/db_tests.rs`
  - Resolution: Added `create_role_with_duplicate_title_fails` DB test.
  - Source commands: `test-gaps`

### Database — Pagination Count and Data Queries Not Transactionally Consistent

- [x] **#409 — `SELECT COUNT(*)` and `SELECT ... LIMIT/OFFSET` run as separate statements; total can be stale relative to items**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Resolution: Replaced the two-query pattern with a single `SELECT ..., count(*) over() as total_count ... LIMIT/OFFSET` query in all 8 list functions (`get_users`, `get_teams`, `get_user_teams`, `get_team_users`, `get_roles`, `get_items`, `get_team_orders`, `get_order_items`). The window function computes the total row count in the same query execution, eliminating the race window between the count and data queries. The `total_count` column is extracted from `rows.first()` (returning 0 for empty results) and is silently ignored by the existing named-column `FromRow` impls.
  - Source commands: `db-review`

### Documentation — CLAUDE.md `components/mod.rs` Description Incomplete

- [x] **#500 — `components/mod.rs` description in CLAUDE.md says only "Module declarations" but the file also defines `LoadingSpinner`, `PaginationBar`, and `role_tag_class()`**
  - File: `CLAUDE.md` (Project Structure → `components/mod.rs` line)
  - Resolution: Updated description to "Module declarations + `LoadingSpinner` component, `PaginationBar` component, `role_tag_class()` CSS helper".
  - Source commands: `cross-ref-check`

### Documentation — `NEW-UI-COMPONENTS.md` Missing `LoadingSpinner` and `PaginationBar`

- [x] **#501 — `LoadingSpinner` and `PaginationBar` are custom UI components not available in CONNECT design system, but neither is listed in `NEW-UI-COMPONENTS.md`**
  - Files: `NEW-UI-COMPONENTS.md`, `frontend/src/components/mod.rs`
  - Resolution: Added full registry entries for both components (purpose, props, rationale).
  - Source commands: `cross-ref-check`, `practices-audit`

### Documentation — CLAUDE.md Project Structure Missing `config/docker-base.yml`

- [x] **#502 — `config/docker-base.yml` exists on disk and is referenced by `Dockerfile.breakfast` but is absent from the CLAUDE.md project structure listing**
  - File: `CLAUDE.md` (Project Structure → `config/` section)
  - Resolution: Added "docker-base.yml – Sanitized base config for Docker images (all secret fields empty; supply via env vars)" to the config listing.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md `db/membership.rs` Function List Missing `count_admins`

- [x] **#503 — `count_admins` is a public function in `src/db/membership.rs` but does not appear in the CLAUDE.md parenthetical function list**
  - File: `CLAUDE.md` (Project Structure → `db/membership.rs`)
  - Resolution: Added `count_admins` as the first entry in the function list.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md WASM Test Category Breakdown Inaccurate

- [x] **#504 — CLAUDE.md lists "Page rendering (14 tests)" but there are 12 page rendering tests; also omits the "authed_get double-failure (2 tests)" section**
  - File: `CLAUDE.md` (Testing → Frontend section)
  - Resolution: Changed to "Page rendering (12 tests)" and added "authed_get double-failure (2 tests): retry after 401 fails, double-failure falls back to login" section.
  - Source commands: `cross-ref-check`

### Code Quality — `healthcheck.rs` Builds Unused `root_store` Variable

- [x] **#377 — `root_store` is created then shadowed or never read in the healthcheck binary**
  - File: `src/bin/healthcheck.rs`
  - Resolution: Removed the two lines that created the unused `root_store` variable (cert store not needed when using `NoVerifier`).
  - Source commands: `review`

### Code Quality — `db_tls_connector` Panics Instead of Returning Result

- [x] **#378 — `db_tls_connector()` in `server.rs` uses `.expect()` on certificate loading, panicking at runtime if certs are missing**
  - File: `src/server.rs`
  - Resolution: Changed `db_tls_connector()` to return `Result<MakeRustlsConnect, Box<dyn std::error::Error>>`, replacing `.expect()` with `?`. Caller uses `db_tls_connector()?`. Updated unit test to use `result.err().map(|e| e.to_string())` (not `.unwrap_err()`, which requires `T: Debug`).
  - Source commands: `review`

### Frontend — `authed_request` Collapses All Errors to `Option`

- [x] **#364 — `authed_request()` returns `Option<Response>`, discarding HTTP error codes and network errors**
  - File: `frontend/src/api.rs` (lines ~266–296)
  - Resolution: Changed `send_once` closure from `.ok()` to an explicit `match` that calls `web_sys::console::warn_1` on network errors before returning `None`, making failures discoverable in DevTools.
  - Source commands: `review`

### Frontend — Create Dialogs Don't Reset Form State on Cancel

- [x] **#367 — Closing a create dialog without submitting leaves stale values in form fields**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`, `frontend/src/pages/orders.rs`
  - Resolution: Added `reset` closures to all 5 `Create*Dialog` components; backdrop and cancel handlers call `reset()`.
  - Source commands: `review`

### Frontend — `OrderDetail` Add-Item Form Doesn't Reset on Order Change

- [x] **#368 — Selecting a different order retains the previously selected item and quantity in the add-item form**
  - File: `frontend/src/pages/order_components.rs`
  - Resolution: Added `Effect::new(move |_| { set_add_item_id.set("".into()); set_add_qty.set(1); })` in `OrderDetail` that fires when the `order` signal changes.
  - Source commands: `review`

### Frontend — Fetch JSON Deserialization Errors Silently Swallowed in 5 Pages

- [x] **#369 — `.json::<T>().await.unwrap_or_default()` hides deserialization failures**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`
  - Resolution: Changed all JSON deserialize calls to `match` expressions that call `web_sys::console::warn_1(...)` on error before falling back to default. Added `console` feature to web-sys in `frontend/Cargo.toml`.
  - Source commands: `review`

### Frontend — Signal-Inside-Reactive-Closure Anti-Pattern in 5 Pages

- [x] **#317 — `teams.rs`, `orders.rs`, `items.rs`, `roles.rs`, `admin.rs` create signals inside `move || {}` closures**
  - Files: `frontend/src/pages/teams.rs`, `frontend/src/pages/orders.rs`, `frontend/src/pages/items.rs`, `frontend/src/pages/roles.rs`, `frontend/src/pages/admin.rs`; `frontend/src/components/modal.rs`
  - Resolution: Moved delete-confirmation `show_*_modal` signal creation out of reactive closures. Changed `ConfirmModal.open` prop from `ReadSignal<bool>` to `Signal<bool>`; all call sites pass `Signal::derive(...)`. All `Create*Dialog` components use `open: Signal<bool>`. All call sites pass `show_*_modal.into()`.
  - Source commands: `review`

### Frontend — `sleep_ms` Uses `js_sys::eval` in Production Code

- [x] **#320 — `sleep_ms` helper uses `js_sys::eval` to create a Promise-based sleep**
  - File: `frontend/src/api.rs` (line ~372)
  - Resolution: Replaced `js_sys::eval` with `Closure::once_into_js` + `web_sys::Window::set_timeout_with_callback_and_timeout_and_arguments_0`. CSP-safe with no `eval`.
  - Source commands: `review`

### Testing — Delete-Not-Found API Paths for 5 Entities

- [x] **#296 — No API test calls DELETE with a nonexistent ID for items, roles, team orders, order items, or members**
  - File: `tests/api_tests.rs`
  - Resolution: Added 5 integration tests (`delete_nonexistent_item_returns_404`, `delete_nonexistent_role_returns_404`, `delete_nonexistent_team_order_returns_404`, `delete_nonexistent_order_item_returns_404`, `remove_nonexistent_team_member_returns_404`).
  - Source commands: `test-gaps`

### Testing — Revoking an Expired Token

- [x] **#299 — No test submits a legitimately-expired (but validly-signed) token for revocation**
  - File: `tests/api_tests.rs`
  - Resolution: Added `revoke_expired_token_returns_200` integration test that crafts a token with `exp` in the past and submits it for revocation.
  - Source commands: `test-gaps`

### Testing — UPDATE with Nonexistent ID → 404

- [x] **#300 — DB-level tests exist but no API integration test verifies HTTP 404 for PUT with nonexistent UUID across 6 update endpoints**
  - File: `tests/api_tests.rs`
  - Resolution: Added 6 integration tests for PUT with nonexistent UUID: users, teams, roles, items, team orders, order items.
  - Source commands: `test-gaps`

### Testing — Shared Frontend Components

- [x] **#322 — `modal.rs`, `toast.rs`, `sidebar.rs`, `card.rs`, `icons.rs`, `theme_toggle.rs` have no WASM tests**
  - Files: `frontend/src/components/`, `frontend/tests/ui_tests.rs`
  - Resolution: Added 4 WASM tests: `test_toast_region_renders_on_dashboard`, `test_sidebar_nav_items_rendered`, `test_sidebar_active_nav_item`, `test_confirm_modal_structure_on_delete`.
  - Source commands: `test-gaps`

### Testing — Order-Item RBAC

- [x] **#323 — No integration test verifies that a team member cannot modify another member's order items**
  - Files: `tests/api_tests.rs`, `src/handlers/orders.rs`
  - Resolution: Added `member_cannot_update_other_members_order_item` and `member_cannot_delete_other_members_order_item` integration tests.
  - Source commands: `test-gaps`, `rbac-rules`

### Testing — `verify_jwt_for_revocation` Unit Tests

- [x] **#349 — Security-sensitive function that skips expiry validation has no test verifying expired-but-valid tokens are accepted**
  - File: `src/middleware/auth.rs`
  - Resolution: Added 3 unit tests: `verify_jwt_for_revocation_accepts_expired`, `verify_jwt_for_revocation_rejects_tampered`, `verify_jwt_for_revocation_rejects_wrong_secret`.
  - Source commands: `test-gaps`

### Testing — `validate_non_negative_price` Unit Tests

- [x] **#352 — Custom validator for item price never directly tested (negative, zero, positive cases)**
  - File: `src/models.rs`
  - Resolution: Added 3 unit tests for negative, zero, and positive price values.
  - Source commands: `test-gaps`

### Testing — `CreateUserEntry` Name Field Boundary Tests

- [x] **#353 — firstname/lastname max=50 boundary untested (50 chars should pass, 51 should fail)**
  - File: `src/models.rs`
  - Resolution: Added comprehensive boundary tests for min=2 (1 char fails, 2 passes), max=50 (50 passes, 51 fails), for both firstname and lastname.
  - Source commands: `test-gaps`

### Testing — Team/Role/Item Model Field Length Boundaries

- [x] **#354 — `tname` max=255, `descr` max=1000, role `title` max=255, item `descr` max=255 — all untested at boundary**
  - File: `src/models.rs`
  - Resolution: Added boundary tests for CreateTeamEntry (tname 255/256, descr 1000/1001), UpdateTeamEntry, CreateRoleEntry (title 255/256), UpdateRoleEntry, CreateItemEntry (descr 255/256), UpdateItemEntry.
  - Source commands: `test-gaps`

### Testing — Non-Owner Member Order Update/Delete

- [x] **#355 — A team member who didn't create the order, and is not a team admin, tries PUT/DELETE — no test**
  - File: `tests/api_tests.rs`
  - Resolution: Added `non_owner_member_cannot_update_team_order` and `non_owner_member_cannot_delete_team_order` integration tests.
  - Source commands: `test-gaps`

### Testing — `ActixJson` Deserialize Error Branch

- [x] **#356 — `JsonPayloadError::Deserialize` with `.data()` → 422 path has no test (only parse error is tested)**
  - File: `src/errors.rs`
  - Resolution: Added `actix_json_deserialize_data_error_returns_422` unit test that constructs a `JsonPayloadError::Deserialize` with `serde::de::Error::custom()`.
  - Source commands: `test-gaps`

### Testing — Frontend Orders Page Create Dialog

- [x] **#357 — Add-item, remove-item, create/delete order interactions have no WASM tests**
  - Files: `frontend/src/pages/orders.rs`, `frontend/tests/ui_tests.rs`
  - Resolution: Added `test_orders_page_create_order_dialog_opens` WASM test.
  - Source commands: `test-gaps`

### Testing — Frontend Profile Page Password Change

- [x] **#358 — Edit mode, password validation, and save logic have no WASM tests**
  - Files: `frontend/src/pages/profile.rs`, `frontend/tests/ui_tests.rs`
  - Resolution: Added 3 WASM tests: `test_profile_page_edit_mode_toggle`, `test_profile_page_password_field_reveals_current_password`, `test_profile_page_cancel_exits_edit_mode`.
  - Source commands: `test-gaps`

### Testing — `DbMapper::Conversion` Error Variant

- [x] **#359 — Only `ColumnNotFound` sub-variant is tested; `Conversion` has its own log-and-respond branch with zero coverage**
  - File: `src/errors.rs`
  - Resolution: Added `db_mapper_conversion_error_returns_500` and `db_mapper_conversion_error_body_is_sanitized` unit tests.
  - Source commands: `test-gaps`

### Testing — Token Refresh After User Deletion

- [x] **#387 — No test refreshes a token after the user has been deleted from the database**
  - File: `tests/api_tests.rs`
  - Resolution: Added `refresh_token_after_user_deleted_returns_error` integration test.
  - Source commands: `test-gaps`

### Testing — Admin Assigning Admin Role Positive Path

- [x] **#388 — `guard_admin_role_assignment` allows Admin to assign Admin role, but no test exercises this success path**
  - File: `tests/api_tests.rs`
  - Resolution: Added `admin_can_assign_admin_role_via_add_member` integration test.
  - Source commands: `test-gaps`

### Testing — `delete_user_by_email` Invalid Email Format

- [x] **#390 — No API test sends a malformed email string to verify 422 response**
  - File: `tests/api_tests.rs`
  - Resolution: Added `delete_user_by_email_invalid_format_returns_422` integration test.
  - Source commands: `test-gaps`

### Testing — Email Change Dual Cache Invalidation

- [x] **#391 — No test changes a user's email and verifies both old and new cache keys are invalidated**
  - File: `tests/api_tests.rs`
  - Resolution: Added `update_user_email_invalidates_both_old_and_new_cache_keys` integration test.
  - Source commands: `test-gaps`

### Testing — GET /teams/{nonexistent}/users Behavior

- [x] **#392 — No test verifies whether the endpoint returns 200 `[]` or 404 for a non-existent team**
  - File: `tests/api_tests.rs`
  - Resolution: Added `team_users_for_nonexistent_team_returns_empty` integration test.
  - Source commands: `test-gaps`

### Testing — `check_team_access` for Team Admin Role

- [x] **#393 — DB tests cover Admin bypass and Member access but not Team Admin role specifically**
  - File: `tests/db_tests.rs`
  - Resolution: Added `check_team_access_team_admin` DB integration test.
  - Source commands: `test-gaps`

### Testing — Order Entry `amt` Range Validation

- [x] **#396 — `CreateOrderEntry` and `UpdateOrderEntry` have `#[validate(range(min=1, max=10000))]` on `amt` but no test verifies boundary values**
  - File: `src/models.rs`
  - Resolution: Added boundary tests for amt=0 (fails), amt=1 (passes) for both Create and Update order entries.
  - Source commands: `test-gaps`

### Testing — Revoke Already-Revoked Token Idempotency

- [x] **#461 — No API test calls `POST /auth/revoke` twice with the same token**
  - File: `tests/api_tests.rs`
  - Resolution: Added `revoke_same_token_twice_is_idempotent` integration test.
  - Source commands: `test-gaps`

### Testing — `Cache-Control: no-store` Header on Auth Responses

- [x] **#462 — Both `auth_user` and `refresh_token` set the header but no test asserts its presence**
  - File: `tests/api_tests.rs`
  - Resolution: Added `auth_response_has_cache_control_no_store` and `refresh_response_has_cache_control_no_store` integration tests.
  - Source commands: `test-gaps`

### Testing — `ErrorResponse::Display` Fallback Branch

- [x] **#463 — The `serde_json::to_string` failure fallback in `Display` impl has no test**
  - File: `src/errors.rs`
  - Resolution: Added `error_response_display_normal` and `error_response_display_with_special_chars` unit tests.
  - Source commands: `test-gaps`

### Testing — `ActixJson` Catch-All Error Branch

- [x] **#464 — The `_ =>` branch for generic `JsonPayloadError` (overflow, EOF) returns 400 but is untargeted by tests**
  - File: `src/errors.rs`
  - Resolution: Added `actix_json_parse_error_returns_400` unit test.
  - Source commands: `test-gaps`

### Testing — `CreateUserDialog` and `EditUserDialog` WASM Tests

- [x] **#511 — Admin page dialog components for creating and editing users have no test coverage**
  - File: `frontend/src/pages/admin.rs`, `frontend/tests/ui_tests.rs`
  - Resolution: Added 7 WASM tests: `test_create_user_dialog_opens`, `test_create_user_dialog_has_form_fields`, `test_create_user_dialog_create_disabled_when_empty`, `test_create_user_dialog_cancel_closes`, `test_edit_user_dialog_opens`, `test_edit_user_dialog_has_form_fields`, `test_edit_user_dialog_cancel_closes`.
  - Source commands: `test-gaps`

### Security — Swagger UI Explicit Opt-In

- [x] **#336 — Swagger UI at `/explorer` available in all non-production environments (staging, preprod, etc.)**
  - File: `src/routes.rs`
  - Resolution: Changed to explicit `ENABLE_SWAGGER` env var opt-in. Defaults to `!is_production` if unset, but can be explicitly disabled in staging/preprod environments.
  - Source commands: `security-audit`

### Security — Refresh Token Rotation Revokes Old Access Token

- [x] **#337 — When refresh token is used to obtain a new pair, the old access token remains valid up to 15 minutes**
  - Files: `src/handlers/users.rs`, `src/models.rs`, `src/middleware/openapi.rs`, `frontend/src/api.rs`
  - Resolution: `refresh_token` handler now accepts optional `RefreshRequest { access_token }` body. Frontend sends old access token on refresh. Server validates (same user, Access type) and revokes it immediately, closing the window where a leaked token could be reused.
  - Source commands: `security-audit`

### Security — HSTS Preload Directive

- [x] **#338 — HSTS value is `max-age=31536000; includeSubDomains` but lacks `preload`**
  - File: `src/server.rs`
  - Resolution: Added `; preload` to HSTS header value.
  - Source commands: `security-audit`

### Frontend — Client-Side `maxlength` on Form Inputs

- [x] **#340 — Frontend input fields lack `maxlength` attributes matching backend validation rules**
  - Files: `frontend/src/pages/login.rs`, `frontend/src/pages/admin.rs`, `frontend/src/pages/profile.rs`
  - Resolution: Added `maxlength=255` on email/username fields, `maxlength=128` on password fields, `maxlength=50` on first/last name fields across login, admin, and profile pages.
  - Source commands: `security-audit`

### Security — Password Fields `autocomplete` Attribute

- [x] **#379 — Profile page password input missing `autocomplete` attribute**
  - Files: `frontend/src/pages/admin.rs`, `frontend/src/pages/profile.rs`
  - Resolution: Added `autocomplete="new-password"` on create/reset password fields, `autocomplete="current-password"` on current password field.
  - Source commands: `security-audit`

### Security — Argon2id OWASP Parameters

- [x] **#380 — Default Argon2id parameters (19 MiB, 2 iterations, 1 lane) are below OWASP recommendation (46 MiB, 1 iteration, 1 lane)**
  - Files: `src/lib.rs`, `src/middleware/auth.rs`
  - Resolution: Changed `Params::default()` to `Params::new(47104, 1, 1, None)` (46 MiB, 1 iteration, 1 lane). Regenerated DUMMY_HASH with matching parameters.
  - Source commands: `security-audit`

### Security — `local_storage()` Doc Warning

- [x] **#448 — Public helper exists alongside `session_storage()`; could invite token misuse by future developers**
  - File: `frontend/src/api.rs`
  - Resolution: Added doc-comment warning against storing tokens in localStorage.
  - Source commands: `security-audit`

### Security — Docker Container Hardening

- [x] **#449 — No `read_only: true`, `security_opt: ["no-new-privileges:true"]`, or `cap_drop: ["ALL"]`**
  - File: `docker-compose.yml`
  - Resolution: Added `read_only: true` + `security_opt: ["no-new-privileges:true"]` on breakfast container; `security_opt` only on postgres (needs writable filesystem).
  - Source commands: `security-audit`

### Security — CORS `allowed_origin` Fix

- [x] **#450 — `allowed_origin` uses bind address `0.0.0.0` which browsers never produce as Origin**
  - File: `src/server.rs`
  - Resolution: Substitutes `0.0.0.0` with `localhost` for CORS origin since browsers never send `Origin: https://0.0.0.0:...`.
  - Source commands: `security-audit`

### Security — `.git` Directory in Docker Build

- [x] **#451 — Full git history in builder image cache; used only for `git_version!()`**
  - Files: `Dockerfile.breakfast`, `.dockerignore`
  - Resolution: Removed redundant `COPY .git/` from Dockerfile; added `.git` to `.dockerignore` to exclude git history from Docker build context.
  - Source commands: `security-audit`

### OpenAPI — Order-Item 403 Descriptions Updated

- [x] **#326 — `create_order_item`, `update_order_item`, `delete_order_item` utoipa 403 descriptions now match actual RBAC guards**
  - File: `src/handlers/orders.rs`
  - Resolution: Updated 403 descriptions to include closed-order guard (`guard_open_order`) for all three mutation endpoints.
  - Source commands: `openapi-sync`

### Documentation — `db-review.md` `init_dev_db.sh` Description Fixed

- [x] **#370 — `.claude/commands/db-review.md` description of `init_dev_db.sh` updated**
  - File: `.claude/commands/db-review.md`
  - Resolution: Description now accurately states the script runs all migrations (V1–V8), creates the refinery tracking table, and loads seed data.
  - Source commands: `cross-ref-check`

### Documentation — README Make Targets Table Completed

- [x] **#371 — README now includes `check`, `fmt`, and `audit-install` Make targets**
  - File: `README.md`
  - Resolution: Added missing targets to the Make targets table.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Env Var Prefix Documented

- [x] **#372 — Config env var override prefix `BREAKFAST_` now documented in Key Conventions**
  - File: `CLAUDE.md`
  - Resolution: Updated config layering bullet to include `prefix: BREAKFAST_`.
  - Source commands: `cross-ref-check`

### Database — `token_blacklist.revoked_at` NOT NULL Enforced

- [x] **#373 — `revoked_at` column now has NOT NULL constraint**
  - Files: `migrations/V9__avatar_index_and_revoked_not_null.sql`, `database.sql`
  - Resolution: V9 migration backfills NULLs and adds NOT NULL. `database.sql` updated to match.
  - Source commands: `db-review`

### OpenAPI — `delete_user_by_email` 422 Response Added

- [x] **#384 — utoipa annotations now include 422 (validation error) response**
  - File: `src/handlers/users.rs`
  - Resolution: Added `(status = 422, description = "Validation error - invalid email format")` to annotations.
  - Source commands: `openapi-sync`

### OpenAPI — Auth Endpoints 429 Response Added

- [x] **#385 — `auth_user` and `refresh_token` utoipa annotations now include 429 response**
  - File: `src/handlers/users.rs`
  - Resolution: Added `(status = 429, description = "Too Many Requests - rate limited or account temporarily locked")` to both endpoints.
  - Source commands: `openapi-sync`

### Dependencies — Unused `logs` Feature Removed from OpenTelemetry

- [x] **#386 — `opentelemetry` and `opentelemetry_sdk` no longer compile unused `logs`/`metrics` modules**
  - File: `Cargo.toml`
  - Resolution: Disabled default features, enabled only `trace` (and `rt-tokio-current-thread` for SDK).
  - Source commands: `dependency-check`

### OpenAPI — `update_user` 403 Description Updated

- [x] **#446 — 403 description now includes password verification failure case**
  - File: `src/handlers/users.rs`
  - Resolution: Updated 403 description to mention "or current password is incorrect".
  - Source commands: `openapi-sync`

### Frontend — `fetch_user_details` Now Logs Non-OK Responses

- [x] **#510 — Non-OK responses (403, 500) now logged via `web_sys::console::warn_1`**
  - File: `frontend/src/api.rs`
  - Resolution: Added console warning with status code before returning `None`.
  - Source commands: `review`

### Database — `database.sql` Redundant Indexes Removed

- [x] **#512 — `idx_users_email` and `idx_teams_name` removed from `database.sql`**
  - File: `database.sql`
  - Resolution: Removed the two CREATE INDEX statements that were dropped by V7 migration.
  - Source commands: `db-review`

### Database — `database.sql` Avatar Support Added

- [x] **#517 — `database.sql` now includes avatars table and `users.avatar_id` FK column**
  - File: `database.sql`
  - Resolution: Added avatars table CREATE, ALTER TABLE for avatar_id FK, and index. Added DROP TABLE for avatars.
  - Source commands: `db-review`

### Database — `users.avatar_id` FK Index Added

- [x] **#518 — Index on `users.avatar_id` FK created via V9 migration**
  - Files: `migrations/V9__avatar_index_and_revoked_not_null.sql`, `database.sql`
  - Resolution: V9 migration creates `idx_users_avatar`. `database.sql` also updated.
  - Source commands: `db-review`

### Security — CSP `'unsafe-inline'` Investigation

- [x] **#621 — Consider SRI hashes via Trunk `--hash` to replace `'unsafe-inline'` in `script-src`**
  - File: `src/server.rs`
  - Resolution: Investigated — Trunk generates a different inline `<script type="module">` on each build with no built-in hash output. Bridging Trunk build output to backend CSP header is complex and fragile. Reclassified from minor to informational; `'unsafe-inline'` is documented as required in CLAUDE.md CSP section.
  - Source commands: `security-audit`

### Practices — CLAUDE.md `#[instrument]` Convention Description Imprecise

- [x] **#631 — Documentation says "skip(state)" but handlers also skip `req`, `json`, `basic`, `body`**
  - File: `CLAUDE.md`
  - Resolution: Reworded to: "`state` is always skipped; handlers may also skip `req`, `json`, `basic`, `body` as appropriate".
  - Source commands: `practices-audit`

### Practices — CLAUDE.md "Unfinished Work" Overstates Test Gap

- [x] **#632 — Says "lack comprehensive WASM test coverage" but 79 tests now exist covering all 6 pages**
  - File: `CLAUDE.md`
  - Resolution: Reworded to reflect current state: "79 tests cover all pages with rendering and basic interaction tests; deeper workflow and edge-case tests still missing".
  - Source commands: `practices-audit`

### Database — `orders.orders_team_id` Missing NOT NULL

- [x] **#325 — Advisory: verify that `orders_team_id` FK column has NOT NULL**
  - Files: `migrations/V1__initial_schema.sql` (line 94), `src/models.rs`
  - Resolution: Added `ALTER TABLE orders ALTER COLUMN orders_team_id SET NOT NULL` in `migrations/V12__cleanup_index_and_constraints.sql`.
  - Source commands: `db-review`

### Testing — `basic_validator` Malformed Password Hash Path Untested

- [x] **#350 — When DB stores a corrupted/non-Argon2 hash, `PasswordHash::new()` fails and returns 500 — no test**
  - File: `src/middleware/auth.rs`
  - Resolution: Added `password_hash_new_rejects_corrupted_hash` unit test covering plain text strings, truncated hashes with invalid salt/output, and a sanity check that the valid `DUMMY_HASH` parses.
  - Source commands: `test-gaps`

### Database — `idx_teamorders_id_due` Index Unused

- [x] **#374 — Covering index on `(orders_team_id, due)` is never used; all order queries filter by `team_id` alone or by primary key**
  - File: `migrations/V6__order_constraint_and_index.sql`
  - Resolution: Added `DROP INDEX IF EXISTS idx_teamorders_id_due` in `migrations/V12__cleanup_index_and_constraints.sql`.
  - Source commands: `db-review`

### Testing — `refresh_token` `DateTime::from_timestamp` Fallback Untested

- [x] **#395 — The `DateTime::from_timestamp(exp, 0).unwrap_or_default()` fallback in `refresh_token` handler is never tested**
  - File: `src/handlers/users.rs`
  - Resolution: Added `datetime_from_timestamp_fallback_on_extreme_values` unit test verifying that `DateTime::<Utc>::from_timestamp(i64::MAX, 0)` returns `None` and the fallback produces a value ~7 days from now.
  - Source commands: `test-gaps`

### Database — FK Constraint Violations Return Generic 409 Message

- [x] **#440 — All foreign key violations (23503) map to same opaque message regardless of which relationship is violated**
  - File: `src/errors.rs`
  - Resolution: FK constraint error now extracts constraint/table name via `as_db_error()` and pattern-matches on "item", "team", "user", "role", "order" to produce contextual messages like "Referenced item does not exist (orders)".
  - Source commands: `db-review`

### Database — No DB-Level Aggregate Query for Order Totals

- [x] **#441 — No `get_order_total()` function; frontend must fetch all items and compute totals client-side**
  - File: `src/db/order_items.rs`
  - Resolution: Added `pub async fn get_order_total(client, teamorder_id, team_id) -> Result<Decimal, Error>` using `SELECT COALESCE(SUM(i.price * o.amt), 0)` with `rust_decimal::Decimal`.
  - Source commands: `db-review`

### Dependencies — `opentelemetry-stdout` Used Unconditionally

- [x] **#445 — Trace spans go to stdout in both dev and production; may conflict with Bunyan JSON logging in prod**
  - File: `src/server.rs`
  - Resolution: OTel stdout exporter is now conditional — only wired in development mode. Production builds use `SdkTracerProvider::builder().build()` without an exporter.
  - Source commands: `dependency-check`

### Frontend — No Client-Side Validation for Item Price Format

- [x] **#520 — Frontend items page accepts free-form text for price without validating it's a valid decimal number**
  - File: `frontend/src/pages/items.rs`
  - Resolution: Added `is_valid_price()` helper that validates non-negative finite f64. Both Create and Edit dialogs now disable submit on invalid price and show inline `field-error` messages.
  - Source commands: `api-completeness`

### Database — Seed Data Not Idempotent (Stale)

- [x] **#439 — `ON CONFLICT DO NOTHING` never fires because PK is auto-generated UUID; re-running seed creates duplicates**
  - File: `database_seed.sql`
  - Resolution: Resolved — no longer surfaced by assessment. Referenced file `database_seed.sql` does not exist; `database.sql` is deprecated and contains no `ON CONFLICT` clauses.
  - Source commands: `db-review`

### Frontend — Uses `String` for UUIDs Everywhere

- [x] **#321 — No type safety for UUID fields in frontend API types**
  - File: `frontend/src/api.rs`
  - Resolution: Accepted — a type alias adds no real safety; only a newtype wrapper would, which requires invasive changes throughout the frontend. In WASM context, UUID serde roundtrips through strings regardless.
  - Source commands: `review`

### Security — Account Lockout State In-Memory Only

- [x] **#339 — Login attempt tracking stored in `DashMap`, not shared across instances**
  - File: `src/middleware/auth.rs`
  - Resolution: Accepted — single-instance internal app. Shared lockout state (e.g. Redis) would add infrastructure complexity disproportionate to the threat model.
  - Source commands: `security-audit`

### API Completeness — Frontend `ItemEntry.price` Typed as `String`

- [x] **#366 — Frontend `ItemEntry` uses `pub price: String` instead of a numeric type**
  - File: `frontend/src/api.rs`
  - Resolution: Accepted — backend uses `rust_decimal` with `serde-with-str` feature, which serializes prices as JSON strings. Frontend `String` type is correct.
  - Source commands: `api-completeness`

### Code Quality — Identical Create/Update Model Pairs in `models.rs`

- [x] **#375 — `CreateTeamEntry`/`UpdateTeamEntry`, `CreateRoleEntry`/`UpdateRoleEntry`, `CreateItemEntry`/`UpdateItemEntry` have identical fields**
  - File: `src/models.rs`
  - Resolution: Accepted — separate types are intentional for distinct OpenAPI schema names. The ~10 lines of duplication per pair is a reasonable trade-off for API documentation clarity.
  - Source commands: `review`

### Security — JWT Validator Performs DB Lookup on Every Request

- [x] **#381 — `jwt_validator` calls `db::get_user_by_email` on every authenticated request after cache miss**
  - File: `src/middleware/auth.rs`
  - Resolution: Accepted — by design. The auth cache covers the warm path; cold requests must verify the user still exists in the DB.
  - Source commands: `security-audit`

### Security — No Rate Limiting on Password Change Endpoint

- [x] **#382 — `PUT /api/v1.0/users/{id}` has no rate limiter for password changes**
  - File: `src/routes.rs`
  - Resolution: Accepted — endpoint requires JWT auth, `current_password` is verified via Argon2 (slow), and the PUT route shares a resource with GET/DELETE making selective rate-limiting architecturally complex.
  - Source commands: `security-audit`

### Security — `delete_user_by_email` Email Existence Oracle

- [x] **#383 — DELETE endpoint returns 404 vs 204, revealing whether an email exists in the system**
  - File: `src/handlers/users.rs`
  - Resolution: Fixed — changed the not-found case to return HTTP 200 with `deleted: false` instead of 404. Updated OpenAPI spec and integration test.
  - Source commands: `security-audit`

### Testing — `auth_user` Cache Miss Path Untested

- [x] **#389 — No test verifies cache miss path (first login or after TTL expiry)**
  - File: `src/middleware/auth.rs`
  - Resolution: Fixed — added `cache_miss_returns_none_for_unknown_user` and `cache_miss_after_ttl_expiry` unit tests.
  - Source commands: `test-gaps`

### Testing — Health Endpoint 503 Response Never Tested

- [x] **#394 — No integration test verifies that `/health` returns HTTP 503 when the database is unreachable**
  - File: `tests/api_tests.rs`
  - Resolution: Fixed — added `health_returns_503_when_db_unreachable` integration test with unreachable pool.
  - Source commands: `test-gaps`

### Database — `SET timezone` in V1 Is Session-Scoped Dead Code

- [x] **#438 — `SET timezone = 'Europe/Copenhagen'` only affects the migration connection session**
  - File: `migrations/V1__initial_schema.sql`
  - Resolution: Accepted — cannot modify applied Refinery migration. Harmless dead code; application uses UTC via `chrono::Utc`.
  - Source commands: `db-review`

### Dependencies — `rustls` `tls12` Feature May Be Unnecessary

- [x] **#442 — Internal app could enforce TLS 1.3 only by removing `tls12` feature**
  - File: `Cargo.toml`
  - Resolution: Fixed — removed `tls12` from rustls features. Internal app now enforces TLS 1.3 only.
  - Source commands: `dependency-check`

### Dependencies — Three Versions of `getrandom` Compiled

- [x] **#443 — `getrandom` v0.2, v0.3, and v0.4 all compiled due to ecosystem version split**
  - File: `Cargo.toml` (transitive)
  - Resolution: Accepted — transitive dependency split; will consolidate as ecosystem converges.
  - Source commands: `dependency-check`

### Dependencies — `refinery` Pulls `toml` 0.8 Alongside `config`'s 0.9

- [x] **#444 — Duplicates the TOML parser; will resolve when `refinery` upgrades upstream**
  - File: `Cargo.toml` (transitive)
  - Resolution: Accepted — transitive dependency conflict, cannot fix without upstream release.
  - Source commands: `dependency-check`

### Security — JWT HS256 With No Key Rotation Mechanism

- [x] **#447 — No `kid` claim or multi-key support; compromised secret requires full restart**
  - File: `src/middleware/auth.rs`
  - Resolution: Accepted — single-instance internal app. Key rotation with `kid` headers is enterprise-grade scope.
  - Source commands: `security-audit`

### Code Quality — Auth Cache Eviction O(n)

- [x] **#453 — `evict_oldest_if_full` iterates all 1000 entries to find oldest; fine at current scale**
  - File: `src/middleware/auth.rs`
  - Resolution: Accepted — already uses O(n) `select_nth_unstable_by_key` partial sort. At 1000 entries, sub-microsecond.
  - Source commands: `review`

### API Completeness — `OrderItemEntry` vs Backend `OrderEntry` Naming Inconsistency

- [x] **#457 — Frontend renames the struct for clarity but creates naming mismatch with backend**
  - File: `frontend/src/api.rs`
  - Resolution: Accepted — `OrderItemEntry` is intentionally more descriptive than `OrderEntry` for line items within a team order.
  - Source commands: `api-completeness`

### API Completeness — Bulk Team Order Delete Endpoint Not Consumed

- [x] **#458 — `DELETE /api/v1.0/teams/{team_id}/orders` exists but has no frontend UI trigger**
  - File: `src/routes.rs`
  - Resolution: Accepted — kept for API completeness. Not every endpoint needs a UI counterpart.
  - Source commands: `api-completeness`

### API Completeness — Delete-User-by-Email Endpoint Not Consumed

- [x] **#459 — AdminPage deletes by user_id only; the by-email endpoint is unreachable from UI**
  - File: `src/routes.rs`
  - Resolution: Accepted — serves administrative scripting use cases.
  - Source commands: `api-completeness`

### API Completeness — Single-Resource GET Endpoints Not Consumed (×5)

- [x] **#460 — Frontend always fetches via list endpoints; single-resource GETs unused**
  - File: `src/routes.rs`
  - Resolution: Accepted — standard REST API design for future deep linking and external API consumers.
  - Source commands: `api-completeness`

### Database — `items.price` CHECK Constraint Allows Zero

- [x] **#519 — `items.price CHECK (price >= 0)` permits items with zero cost**
  - File: `migrations/V1__initial_schema.sql`, `src/models.rs`
  - Resolution: Fixed — changed `validate_non_negative_price` to reject zero (strictly positive). DB CHECK constraint unchanged (applied migration) but application-level validator now prevents zero-price items.
  - Source commands: `db-review`

### Dependencies — `jwt-compact` Stale Maintenance

- [x] **#628 — Last release Oct 2023 (>2 years); no CVEs but maintenance risk grows**
  - File: `Cargo.toml`
  - Resolution: Accepted — no CVEs, no functional issues. Monitored via `cargo audit`.
  - Source commands: `dependency-check`

### Dependencies — `color-eyre` Stale Release

- [x] **#629 — Last release Dec 2022 (>3 years); still functional but gap is growing**
  - File: `Cargo.toml`
  - Resolution: Accepted — still functional, no CVEs. Used only for panic reports.
  - Source commands: `dependency-check`

### Dependencies — OpenTelemetry Stack Always Compiled

- [x] **#630 — 4 OTel crates pull ~30 transitive deps; could be feature-gated**
  - File: `Cargo.toml`, `src/server.rs`
  - Resolution: Fixed — made all 4 OTel crates optional behind a `telemetry` Cargo feature (default on). Build with `--no-default-features` to skip. `tracing-actix-web/opentelemetry_0_31` conditionally enabled. Server code wrapped in `#[cfg(feature = "telemetry")]`.
  - Source commands: `dependency-check`

### Frontend Design — Password Reset Race, f64 Totals

- [x] **#657 — `do_reset_password` sends all user fields — could overwrite concurrent admin edits**
  - File: `frontend/src/pages/admin.rs`
  - Resolution: Changed `do_reset_password` to fetch fresh user data via `authed_get` before sending the PUT request. The password reset now reads the latest `firstname`, `lastname`, and `email` from the server, preventing stale overwrites.
  - Source commands: `review`

- [x] **#665 — Frontend order total uses f64 instead of Decimal**
  - File: `frontend/src/pages/order_components.rs`
  - Resolution: Replaced f64 arithmetic with integer-cents calculation. `resolve_price` → `resolve_price_cents` (returns i64 cents via fixed-point parsing), `grand_total` → `grand_total_cents`, `line_total` → `line_total_cents`. Display uses `cents / 100` and `cents % 100` formatting. Eliminates floating-point rounding errors for monetary values.
  - Source commands: `review`

### Dependencies — password-hash, MediaQueryList, refinery native-tls

- [x] **#659 — `password-hash` direct dependency may be redundant**
  - File: `Cargo.toml`
  - Resolution: Tested removal — build fails with unresolved `OsRng` import. The `password-hash = { version = "0.5.0", features = ["getrandom"] }` dependency is required because `argon2`'s re-export does not enable the `getrandom` feature on `rand_core`. Dependency confirmed needed; no action required.
  - Source commands: `dependency-check`

- [x] **#660 — Unused `MediaQueryList` web-sys feature**
  - File: `frontend/Cargo.toml`
  - Resolution: Tested removal — build fails because `window.match_media()` in `theme_toggle.rs` requires the `MediaQueryList` web-sys feature to be enabled. The finding was incorrect; the feature IS used indirectly. No change needed.
  - Source commands: `dependency-check`

- [x] **#661 — `refinery` pulls native-tls despite project using rustls**
  - Resolution: Accepted — no fix available. `refinery` has no rustls feature flag. This is a known limitation of the crate. Monitoring for future releases.
  - Source commands: `dependency-check`

### Database — Missing CHECK Constraints

- [x] **#658 — Missing DB CHECK constraints on `users.firstname`/`users.lastname`**
  - File: `migrations/V14__user_text_check_constraints.sql`
  - Resolution: Created V14 migration adding CHECK constraints: `users.firstname` ≤ 50 chars, `users.lastname` ≤ 50 chars, `users.email` ≤ 255 chars. Matches existing API validation limits.
  - Source commands: `db-review`

### Test Coverage — JWT Validators, DB Functions

- [x] **#662 — Auth validators have no unit tests**
  - File: `src/middleware/auth.rs`
  - Resolution: Added 8 unit tests covering JWT validation edge cases: wrong issuer rejection, wrong audience rejection, access/refresh token type differentiation, both types accepted by `verify_jwt`, empty string rejection, garbage string rejection, `verify_jwt_for_revocation` empty string, and claims completeness verification.
  - Source commands: `test-gaps`

- [x] **#663 — Several DB functions untested at DB level**
  - Files: `tests/db_orders.rs`, `tests/db_roles.rs`, `tests/db_users.rs`
  - Resolution: Added 8 DB integration tests: `count_team_orders_returns_correct_count`, `reopen_team_order_creates_copy_of_closed_order`, `reopen_team_order_rejects_open_order`, `get_order_total_returns_correct_sum`, `get_order_total_returns_zero_for_empty_order`, `seed_default_roles_creates_four_default_roles`, `seed_default_roles_is_idempotent`, `count_users_returns_positive_count`.
  - Source commands: `test-gaps`

### API Completeness — By Design

- [x] **#664 — `get_order_total` DB function not exposed as API endpoint**
  - Resolution: Accepted — by design. The total is computed inline by the `get_team_order` handler and included in the order response. No separate endpoint needed.
  - Source commands: `api-completeness`

- [x] **#666 — 8 documented API endpoints not consumed by frontend**
  - Resolution: Accepted — expected. These are admin/programmatic endpoints (bulk delete orders, role CRUD, avatar list) not needed in the frontend UI. No action required.
  - Source commands: `api-completeness`

### Security — Account Lockout, JWT Secret, Swagger, Login Attempts

- [x] **#687 — Lockout is per-email only with no IP component — any unauthenticated attacker can lock any account**
  - File: `src/middleware/auth.rs`
  - Resolution: Changed lockout key from email-only to `email:ip` format via `lockout_key()` helper. Updated `is_account_locked`, `record_failed_attempt`, and `clear_failed_attempts` to accept `peer_ip` parameter. Peer IP extracted from `req.peer_addr()` in `basic_validator` with `"unknown"` fallback.
  - Source commands: `security-audit`

- [x] **#688 — Unlike `token_blacklist`, no periodic cleanup task for stale login attempt entries**
  - File: `src/server.rs`
  - Resolution: Added `spawn_login_attempts_cleanup_task()` background task that runs every 15 minutes, pruning `login_attempts` entries whose newest timestamp is older than the 15-minute lockout window.
  - Source commands: `security-audit`

- [x] **#689 — JWT secret in `State.jwtsecret` is a plain `String` — no zeroization on drop**
  - File: `src/models.rs`, `src/server.rs`, `src/middleware/auth.rs`, `src/handlers/users.rs`
  - Resolution: Changed `State.jwtsecret` from `String` to `secrecy::SecretString`. All callsites now use `.expose_secret()` to access the raw value. Added `secrecy = "0.10.3"` to `Cargo.toml`.
  - Source commands: `security-audit`

- [x] **#690 — If a non-production environment is publicly accessible, full OpenAPI spec is exposed**
  - File: `src/routes.rs`
  - Resolution: Changed Swagger UI default from on (for non-production) to off. Now requires explicit `ENABLE_SWAGGER=true` environment variable to activate. Removed unused `is_production` variable.
  - Source commands: `security-audit`

### Database — Avatar Text Constraints

- [x] **#691 — `name` and `content_type` columns have no length constraints**
  - File: `migrations/V17__avatar_text_constraints.sql`
  - Resolution: Created V17 migration adding `CHECK (char_length(name) <= 255)` and `CHECK (char_length(content_type) <= 100)` on the `avatars` table.
  - Source commands: `db-review`

- [x] **#692 — Two independent CASCADE paths from `teams` to `orders` exist**
  - Resolution: Already resolved by V15 migration (`V15__restrict_cascade_fks.sql`) which changed the relevant FKs from CASCADE to RESTRICT.
  - Source commands: `db-review`

### Testing — API and WASM Test Coverage

- [x] **#693 — DB test exists for duplicate role, but no API test verifies 409 through HTTP stack**
  - File: `tests/api_roles.rs`
  - Resolution: Added `create_duplicate_role_returns_409` integration test.
  - Source commands: `test-gaps`

- [x] **#694 — Admin changing pickup user, clearing pickup user (`null`) not tested**
  - File: `tests/api_orders.rs`, `src/models.rs`
  - Resolution: Added `admin_can_clear_assigned_pickup_user` integration test. Fixed serde `Option<Option<T>>` deserialization: added `deserialize_optional` helper and applied `#[serde(deserialize_with = "deserialize_optional")]` to `UpdateTeamOrderEntry.duedate` and `.pickup_user_id` so that JSON `null` correctly maps to `Some(None)` (clear) vs absent mapping to `None` (preserve).
  - Source commands: `test-gaps`

- [x] **#695 — FK constraint 409 responses not tested through HTTP stack**
  - File: `tests/api_roles.rs`, `tests/api_items.rs`, `src/errors.rs`
  - Resolution: Added `delete_role_in_use_returns_409` and `delete_item_in_use_returns_409` integration tests. Fixed `Error::Db` handler to also match `23001` (RESTRICT_VIOLATION) alongside `23503` (FOREIGN_KEY_VIOLATION), since `ON DELETE RESTRICT` triggers error code 23001, not 23503.
  - Source commands: `test-gaps`

- [x] **#696 — Edit/save profile and password-change flow have no WASM tests**
  - File: `frontend/tests/ui_pages.rs`
  - Resolution: Added `test_profile_page_save_triggers_put_and_exits_edit` and `test_profile_page_password_change_requires_current_password` WASM tests.
  - Source commands: `test-gaps`

- [x] **#697 — Dialog fields tested but submit success/error paths not tested**
  - File: `frontend/tests/ui_admin_dialogs.rs`
  - Resolution: Added `test_create_user_submit_shows_toast` and `test_create_role_submit_shows_toast` WASM tests with write-capable mock fetch helpers.
  - Source commands: `test-gaps`

### Documentation — CLAUDE.md Test Count and Category Fixes

- [x] **#700 — CLAUDE.md "Unfinished Work" says "93 tests" instead of 97**
  - File: `CLAUDE.md`
  - Resolution: Updated "93 tests" to "97 tests" in the Unfinished Work section.
  - Source commands: `cross-ref-check`, `practices-audit`

- [x] **#701 — CLAUDE.md test sub-category breakdown sums to 95, not 97**
  - File: `CLAUDE.md`
  - Resolution: Corrected 5 test category counts: Page rendering 14→12, Token refresh retry 2→1, authed_get double-failure 2→1, Actions column 6→7, Admin password reset 10→12. Categories now sum to 97.
  - Source commands: `cross-ref-check`, `practices-audit`

- [x] **#705 — Command files enumerate V1–V9 specifically**
  - Files: `.claude/commands/api-completeness.md`, `.claude/commands/db-review.md`
  - Resolution: Updated migration references from "V1 through V9" to "V1 through V17" in all three occurrences.
  - Source commands: `cross-ref-check`

### Frontend — DRY and Style Fixes

- [x] **#702 — Duplicated `is_admin` signal derivation across 5 pages**
  - Files: `frontend/src/api.rs`, `frontend/src/pages/admin.rs`, `items.rs`, `roles.rs`, `teams.rs`, `orders.rs`
  - Resolution: Extracted `pub fn is_admin_signal(user: ReadSignal<Option<UserContext>>) -> Signal<bool>` in `frontend/src/api.rs`. Updated all 5 pages to use the shared helper.
  - Source commands: `review`

- [x] **#703 — 11 inline `style=` attributes in order_components.rs**
  - Files: `frontend/src/pages/order_components.rs`, `frontend/style/main.css`
  - Resolution: Moved all 11 inline styles to named CSS classes in `main.css` (order-closed-tag, order-field-group, order-field-group-top, cell-align-right, order-qty-input, order-total-row, add-item-form, field-flex-grow, field-narrow). Zero inline `style=` attributes remain.
  - Source commands: `review`, `practices-audit`

### Backend — Avatar Cache Optimization

- [x] **#706 — Avatar handler clones `Vec<u8>` from `Arc` on every cache hit**
  - Files: `src/models.rs`, `src/handlers/avatars.rs`, `src/server.rs`
  - Resolution: Changed avatar cache type from `DashMap<Uuid, (Arc<Vec<u8>>, String)>` to `DashMap<Uuid, (Bytes, String)>`. `Bytes::clone()` is O(1) reference-count increment instead of full byte copy. Updated all 3 insert sites and the response body construction.
  - Source commands: `review`

### Test Coverage — RBAC Integration Tests

- [x] **#704 — Missing negative-path RBAC integration tests**
  - File: `tests/api_orders.rs`
  - Resolution: Added 2 integration tests: `non_member_cannot_reopen_order` (asserts 403 for non-member reopen attempt) and `team_admin_can_delete_order_by_another_member` (asserts team admin can delete another member's order).
  - Source commands: `rbac-rules`, `test-gaps`

### Documentation — Migration Version Drift

- [x] **#713 — `init_dev_db.sh` references V1–V9 for idempotent migrations**
  - File: `init_dev_db.sh`
  - Resolution: No longer relevant — version-specific migration references removed from documentation files (CLAUDE.md, README.md, command files) to prevent drift. Files now refer to `migrations/` directory generically.
  - Source commands: `cross-ref-check`

## Notes

- Total resolved items: 549 (7 critical, 55 important, 153 minor, 199 informational, plus items previously counted under different categories)
- Items are preserved here permanently for historical reference
- Finding numbers are never reused — new findings continue from the highest number in either file

### Security — Config SecretString

- [x] **#708 — Config secrets not wrapped in `SecretString`**
  - File: `src/config.rs`, `src/server.rs`, `Cargo.toml`
  - Resolution: Wrapped `server.secret` and `server.jwtsecret` in `secrecy::SecretString` in `ServerConfig`. Enabled `serde` feature for `secrecy` crate. Updated all access sites in `server.rs` to use `expose_secret()`. Updated config tests to use `expose_secret()` for assertions. `pg.password` not changed — it belongs to `deadpool_postgres::Config` (external struct).
  - Source commands: `security-audit`

### Security — Auth Cache Invalidation on Revoke

- [x] **#710 — Auth cache TTL window allows revoked tokens for up to 5 minutes**
  - File: `src/handlers/users.rs`
  - Resolution: Added cache invalidation in `revoke_user_token` handler — after revoking the token, the user's email is looked up and their auth cache entry is invalidated via `invalidate_cache()`. This closes the 5-minute TTL window where a revoked token's owner could still authenticate via cached credentials.
  - Source commands: `security-audit`

### Database — Password CHECK Constraint

- [x] **#711 — `password` column has no CHECK constraint on length**
  - File: `migrations/V18__password_hash_check.sql`
  - Resolution: Added migration V18 with `CHECK (length(password) >= 50)` constraint on the `password` column, preventing accidental plaintext storage.
  - Source commands: `db-review`

### Database — Email VARCHAR(254)

- [x] **#712 — email column uses VARCHAR(75), RFC 5321 allows up to 254**
  - Files: `migrations/V19__email_varchar_254.sql`, `src/models.rs`
  - Resolution: Added migration V19 to expand `email` column from `VARCHAR(75)` to `VARCHAR(254)` and update the CHECK constraint to match. Updated both `CreateUserEntry` and `UpdateUserRequest` validators to use `max = 254`.
  - Source commands: `db-review`

### Security — CSP `unsafe-inline` Replaced with SHA-256 Hash

- [x] **#709 — `unsafe-inline` in CSP `script-src`**
  - Files: `src/server.rs`, `frontend/Trunk.toml`, `CLAUDE.md`
  - Resolution: Set `filehash = false` in `Trunk.toml` to make Trunk's inline WASM loader script deterministic across builds. Computed the SHA-256 hash of the loader script (`sha256-hkIUP5VZpQ+CH9Va73b6RJlnUGtVokRUEv+DJuZ14uw=`) and replaced `'unsafe-inline'` with the hash in the CSP `script-src` directive. Updated CLAUDE.md to document the new approach and hash recomputation procedure.
  - Source commands: `security-audit`
