# Resolved Assessment Findings

This file contains all assessment findings that have been resolved, organized by their original severity. Items are moved here from `.claude/assessment-findings.md` when marked `[x]` (completed) as part of the "assess project" process.

Last updated: 2026-03-03

## Critical Items

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

## Important Items

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

## Minor Items

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

## Informational Items

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

## Notes

- Total resolved items: 148 (6 critical, 43 important, 63 minor, 6 informational, plus items previously counted under different categories)
- Items are preserved here permanently for historical reference
- Finding numbers are never reused — new findings continue from the highest number in either file
