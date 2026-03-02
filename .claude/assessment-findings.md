# Assessment Findings

Last assessed: 2026-03-02 (full re-assessment — 45 new findings: #85–#129)

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will preserve completed items, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

### Transaction Safety — TOCTOU Race on Closed-Order Checks

- [x] **#85 — `create_order_item`, `update_order_item`, and `delete_order_item` have TOCTOU race conditions**
  - File: `src/handlers/orders.rs` (all three mutation handlers)
  - Problem: Each handler checks `is_team_order_closed()` then performs the mutation as two separate, non-transactional DB operations. Between the check and the mutation, a concurrent request could close the order, allowing an item to be added/updated/deleted on a closed order.
  - Fix: Wrap the closed-order check and the mutation in a single DB transaction with `SELECT ... FOR UPDATE` on the `teamorders` row. Alternatively, add a DB-level trigger on the `orders` table that prevents INSERT/UPDATE/DELETE when the parent `teamorders.closed = true`.
  - Source commands: `db-review`

## Important Items

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

## Minor Items

### Code Quality — Dead S3 Config Fields

- [ ] **#59 — `s3_key_id` and `s3_key_secret` are loaded and stored but never used**
  - Files: `src/models.rs` lines 46–47 (`State` struct), `src/config.rs` lines 13–14 (`ServerConfig` struct), `src/server.rs` lines 340–341 (state construction), `config/default.yml` lines 6–7, `config/development.yml` lines 5–6, `config/production.yml` lines 6–7
  - Problem: The `s3_key_id` and `s3_key_secret` fields are defined in `ServerConfig`, loaded from config files, stored in `State`, and propagated through all test helpers (`routes.rs`, `server.rs`, `middleware/auth.rs`), but no handler, middleware, or DB function ever reads them.
  - Fix: Either remove the fields entirely from `ServerConfig`, `State`, all config files, and all test helpers — or, if S3 integration is planned, document the intent in CLAUDE.md's Unfinished Work section.
  - Source commands: `review`, `practices-audit`

### Code Quality — Dead `database.url` Config Field

- [ ] **#68 — `database.url` field in `Settings` is configured but unused**
  - Files: `src/config.rs` lines 19–22 (`Database` struct with `#[allow(dead_code)]`), `config/default.yml` lines 10–11, `config/development.yml` lines 1–2
  - Problem: The `Database` struct contains a single `url` field marked `#[allow(dead_code)]`. The DB pool is created from the `pg.*` config fields, not from `database.url`.
  - Fix: Remove the `Database` struct and its `database` field from `Settings`. Remove `database:` sections from config files. Update the config test.
  - Source commands: `review`, `practices-audit`

### Security — Seed Data Uses Hardcoded Argon2 Salt

- [ ] **#70 — All seed users share the same Argon2 hash with a hardcoded salt**
  - File: `database_seed.sql` lines 41–57
  - Problem: All 5 seed users have identical Argon2id password hashes using the salt `dGVzdHNhbHQxMjM0NTY`. While dev-only, this creates risk if accidentally run against production.
  - Fix: Add a prominent `-- WARNING: DO NOT RUN IN PRODUCTION` comment at the top of `database_seed.sql`.
  - Source commands: `security-audit`, `db-review`

### Frontend — All Components in Single `app.rs` File

- [ ] **#71 — Frontend `app.rs` is a 600+ line monolith**
  - File: `frontend/src/app.rs`
  - Problem: The entire frontend lives in a single file. As planned pages are built, this will become unmanageable.
  - Fix: Split into module structure when building the next frontend page.
  - Source commands: `review`, `practices-audit`

### Security — No Account Lockout After Failed Auth Attempts

- [ ] **#73 — Failed authentication is rate-limited but no lockout policy exists**
  - Files: `src/routes.rs` lines 19–23, `src/handlers/users.rs`
  - Problem: The `/auth` endpoint has rate limiting but no account-level lockout after N consecutive failures.
  - Fix: Track failed login attempts per email. Lock after threshold (e.g., 5 failures in 15 minutes).
  - Source commands: `security-audit`

### Deployment — Production Config Has Placeholder Hostname

- [ ] **#75 — `config/production.yml` uses `pick.a.proper.hostname` as the PG host**
  - File: `config/production.yml` line 2
  - Problem: Placeholder string with no startup validation catch.
  - Fix: Add a startup check similar to the secret-validation panic.
  - Source commands: `practices-audit`, `review`

### Code Quality — `verify_jwt` and `generate_token_pair` Are Unnecessarily Async

- [ ] **#94 — Functions contain no `.await` but are marked `async`**
  - File: `src/middleware/auth.rs` lines 52, 77
  - Problem: Creates unnecessary `Future` wrappers on every auth call. Every caller must `.await` them but the compiler generates state-machine code for no benefit.
  - Fix: Change to `pub fn`. Remove `.await` from callers.
  - Source commands: `review`

### Code Quality — Auth Functions Take `String` by Value

- [ ] **#95 — `verify_jwt` and `generate_token_pair` take `String` instead of `&str`**
  - File: `src/middleware/auth.rs` lines 52, 77
  - Problem: Forces `.clone()` at every call site (`state.jwtsecret.clone()`, `credentials.token().to_string()`).
  - Fix: Change signatures to take `&str`.
  - Source commands: `review`

### Code Quality — Magic Strings for Role Names and Token Types

- [ ] **#96 — `"Admin"`, `"Team Admin"`, `"access"`, `"refresh"` scattered as raw strings**
  - Files: `src/db/membership.rs`, `src/handlers/mod.rs`, `src/middleware/auth.rs`
  - Problem: A typo would silently break RBAC or token validation.
  - Fix: Define `const` values or enums (e.g., `pub const ADMIN: &str = "Admin";`).
  - Source commands: `review`

### Code Quality — `StatusResponse` Reused for Token Revocation

- [ ] **#97 — Token revocation returns `{"up": true}` instead of a revocation-specific response**
  - File: `src/handlers/users.rs` line 150
  - Problem: `StatusResponse { up: true }` is the health-check response type. Reusing it for `/auth/revoke` is semantically wrong.
  - Fix: Create a dedicated `RevokedResponse` or use `DeletedResponse`.
  - Source commands: `review`

### Code Quality — Dead `FromRow` Implementations for Input DTOs

- [ ] **#98 — 7 `FromRow` implementations exist for types never read from DB rows**
  - File: `src/from_row.rs` (CreateUserEntry, CreateTeamEntry, UpdateTeamEntry, CreateRoleEntry, UpdateRoleEntry, CreateItemEntry, UpdateItemEntry)
  - Problem: These types are input DTOs (deserialized from JSON). No DB function ever constructs them from a row.
  - Fix: Remove the unused `FromRow` implementations.
  - Source commands: `review`

### Code Quality — `FromRow` Boilerplate

- [ ] **#99 — `from_row` always delegates to `from_row_ref` — 13 identical function bodies**
  - File: `src/from_row.rs`
  - Problem: Every `FromRow` implementation has the same `fn from_row(row: Row) -> ... { Self::from_row_ref(&row) }` body.
  - Fix: Add a default implementation in the trait: `fn from_row(row: Row) -> ... { Self::from_row_ref(&row) }`.
  - Source commands: `review`

### Code Quality — `UsersInTeam`/`UserInTeams` Bypass `FromRow`

- [ ] **#100 — Manual row mapping in `get_team_users` and `get_user_teams` instead of `FromRow`**
  - File: `src/db/teams.rs` lines 27–46, 155–183
  - Problem: Two functions use copy-pasted manual `try_get` logic instead of the `FromRow` trait used everywhere else.
  - Fix: Implement `FromRow` for `UsersInTeam` and `UserInTeams`.
  - Source commands: `review`, `db-review`

### Database — Missing FK Index on `teamorders.teamorders_user_id`

- [ ] **#101 — `teamorders_user_id` foreign key is not indexed**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: Queries joining on this column or `ON DELETE RESTRICT` checks on user deletion will seq-scan `teamorders`.
  - Fix: Add `CREATE INDEX idx_teamorders_user ON teamorders (teamorders_user_id);` in a new V3 migration.
  - Source commands: `db-review`

### Database — Missing FK Index on `orders.orders_team_id`

- [ ] **#102 — `orders_team_id` has no index; queries filter on it**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: `get_order_items` and `delete_order_item` filter on `orders_team_id`, causing seq-scans.
  - Fix: Add `CREATE INDEX idx_orders_team ON orders (orders_team_id);` in a new V3 migration.
  - Source commands: `db-review`

### Database — Redundant Index `idx_orders_tid`

- [ ] **#103 — Composite PK already provides B-tree on leading column**
  - File: `migrations/V1__initial_schema.sql` line 126
  - Problem: `idx_orders_tid` on `(orders_teamorders_id)` is redundant — the PK `(orders_teamorders_id, orders_item_id)` already covers it.
  - Fix: Drop the index in a new migration.
  - Source commands: `db-review`

### Database — `ON DELETE CASCADE` on `orders.orders_item_id` Destroys History

- [ ] **#104 — Deleting a breakfast item silently removes it from all historical orders**
  - File: `migrations/V1__initial_schema.sql` line 99
  - Problem: `ON DELETE CASCADE` on the FK from `orders.orders_item_id` to `items.item_id` means deleting an item destroys order history.
  - Fix: Change to `ON DELETE RESTRICT` (prevent deletion of items in use) or implement soft-delete.
  - Source commands: `db-review`

### Database — `memberof.memberof_role_id` Allows NULL

- [ ] **#105 — A membership without a role bypasses RBAC**
  - File: `migrations/V1__initial_schema.sql` line 65
  - Problem: `memberof_role_id` has no `NOT NULL` constraint. A row with NULL role_id passes membership checks but has no role, creating undefined RBAC behavior.
  - Fix: Add `ALTER TABLE memberof ALTER COLUMN memberof_role_id SET NOT NULL;` in a V3 migration.
  - Source commands: `db-review`

### Code Quality — `TeamOrderEntry.closed` Type Mismatch

- [ ] **#106 — `closed` is `Option<bool>` but DB column is `NOT NULL DEFAULT FALSE`**
  - File: `src/models.rs`
  - Problem: The Rust model will never receive `None` — it will always be `Some(true)` or `Some(false)`.
  - Fix: Change to `pub closed: bool`.
  - Source commands: `db-review`

### Documentation — OpenAPI Path Parameter Names Are Generic

- [ ] **#107 — 15 handlers use `{id}` in utoipa path instead of descriptive names like `{user_id}`**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/items.rs`, `src/handlers/roles.rs`
  - Problem: Swagger UI shows generic `id` parameter names instead of descriptive ones. The `delete_user_by_email` route also misleadingly names the email segment `{user_id}` in routes.rs.
  - Fix: Update utoipa `path` attributes to match actix route parameter names.
  - Source commands: `openapi-sync`

### Documentation — `MIGRATION_FIX_SUMMARY.md` Listed But Deleted

- [ ] **#108 — Project Structure tree references a file that no longer exists on disk**
  - File: `CLAUDE.md` (Project Structure section)
  - Problem: `MIGRATION_FIX_SUMMARY.md` was deleted but CLAUDE.md still lists it.
  - Fix: Remove the entry from the Project Structure tree.
  - Source commands: `practices-audit`

### Performance — RBAC Helpers Make Sequential DB Queries

- [ ] **#109 — `require_team_member` and `require_team_admin` make 2 DB round-trips**
  - File: `src/handlers/mod.rs` lines 30–79
  - Problem: For non-admin users (the common case), both `is_admin()` and `get_member_role()` execute sequentially. Could be combined.
  - Fix: Create a single query checking both admin and team role in one `EXISTS`.
  - Source commands: `db-review`

### Security — Missing HSTS Header

- [ ] **#110 — No `Strict-Transport-Security` despite TLS enforcement**
  - File: `src/server.rs` (DefaultHeaders section)
  - Problem: Without HSTS, a first-visit browser is vulnerable to SSL stripping for the initial HTTP request (before redirect).
  - Fix: Add `.add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))` to `DefaultHeaders`.
  - Source commands: `security-audit`

### Security — Missing `X-Content-Type-Options` Header

- [ ] **#111 — No `X-Content-Type-Options: nosniff` header set**
  - File: `src/server.rs` (DefaultHeaders section)
  - Problem: Older browsers may MIME-sniff responses.
  - Fix: Add `X-Content-Type-Options: nosniff` to `DefaultHeaders`.
  - Source commands: `security-audit`

### Security — Swagger UI Exposed in Production

- [ ] **#112 — `/explorer` registered unconditionally regardless of environment**
  - File: `src/routes.rs` line 29
  - Problem: In production, this exposes the complete API schema, aiding attacker reconnaissance.
  - Fix: Conditionally register the Swagger UI scope only when `ENV != production`, or gate behind admin auth.
  - Source commands: `security-audit`

### Performance — Auth Cache Eviction Is O(n log n)

- [ ] **#113 — Cache eviction sorts all entries on every miss at capacity**
  - File: `src/middleware/auth.rs` lines 323–335
  - Problem: When the cache is full (1000 entries), every miss collects all entries into a `Vec`, sorts by timestamp, and removes the oldest 10%. This is O(n log n) per miss.
  - Fix: Use a proper LRU data structure (e.g., `lru` crate) or a min-heap.
  - Source commands: `review`

### Error Handling — `FromRowError::ColumnNotFound` Maps to HTTP 404

- [ ] **#114 — Missing column (programming error) returns "not found" instead of 500**
  - File: `src/errors.rs` lines 118–123
  - Problem: `ColumnNotFound` indicates a schema mismatch (programming error), not a missing resource. Mapping it to 404 could mislead clients and mask bugs.
  - Fix: Map to 500 Internal Server Error, same as `Conversion`.
  - Source commands: `db-review`

## Informational Items

### Dependencies — Unfixable RSA Advisory

- [ ] **#55 — `rsa` 0.9.10 has an unfixable timing side-channel advisory (RUSTSEC-2023-0071)**
  - Problem: The `rsa` crate (transitive dependency via `jsonwebtoken` 10.3.0) is affected by the Marvin Attack. No patched version is available upstream. The project uses HMAC (HS256), not RSA — the vulnerability is not exploitable.
  - Source command: `dependency-check` (cargo audit)
  - Action: No code changes possible — waiting on upstream fix. Monitor `jsonwebtoken` releases.

### Documentation — Test Count Maintenance Burden

- [ ] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md`
  - Problem: Hard-coded test counts go stale every time tests are added or removed. Proven again by findings #83 (prior assessment) and #91 (this assessment).
  - Source command: `practices-audit`
  - Action: Inherent maintenance cost. The assessment process updates counts each time it runs.

### API Design — No Pagination on List Endpoints

- [ ] **#61 — List endpoints return all records without pagination**
  - Files: `src/db/` (all `get_*` list functions), `src/handlers/` (corresponding GET collection handlers)
  - Problem: All collection endpoints return all rows. Works at current scale but would degrade with growth.
  - Source commands: `review`, `api-completeness`
  - Action: Add `LIMIT`/`OFFSET` when data growth warrants it.

### Deployment — No `.env.example` File for Onboarding

- [ ] **#76 — No `.env.example` or env documentation for new developers**
  - Problem: New developers must read multiple files to discover available environment variables.
  - Source commands: `practices-audit`
  - Action: Create `.env.example` listing available env vars.

### API — `memberof.joined` Timestamp Not Exposed

- [ ] **#115 — `joined` column stored in DB but not returned by API**
  - Files: `src/models.rs` (`UsersInTeam`, `UserInTeams`), `src/db/teams.rs`
  - Problem: `memberof.joined` timestamp is stored but neither model struct includes it, and DB queries don't select it.
  - Source commands: `api-completeness`
  - Action: Add to models and queries if frontend needs it.

### Frontend — Consumes Only 4 of 41 Endpoints

- [ ] **#116 — Frontend only uses auth (3) + user-detail (1) endpoints**
  - File: `frontend/src/app.rs`
  - Problem: 37 backend endpoints are fully implemented but await frontend page development.
  - Source commands: `api-completeness`
  - Action: Documented in CLAUDE.md Frontend Roadmap. Will be consumed as pages are built.

### API Design — GET Endpoints Have No Team-Scoped RBAC

- [ ] **#117 — Any authenticated user can read any team's data**
  - Files: `src/handlers/teams.rs`, `src/handlers/orders.rs`, `src/handlers/users.rs`
  - Problem: All GET endpoints only require JWT authentication, not team membership. Deliberate design choice.
  - Source commands: `api-completeness`, `security-audit`
  - Action: Document as intentional. Reconsider if multi-tenant isolation is needed.

### Deployment — Dev Config in Production Docker Image

- [ ] **#118 — `development.yml` copied into production image unnecessarily**
  - File: `Dockerfile.breakfast` line 78
  - Problem: Dev config with localhost DB strings is included in production image.
  - Source commands: `security-audit`
  - Action: Only copy `default.yml` and `production.yml`.

### Security — Rate Limiter Uses IP-Based Key Extraction

- [ ] **#119 — Behind a reverse proxy, all requests share one IP**
  - File: `src/routes.rs` lines 20–24
  - Problem: `actix-governor` defaults to `PeerIpKeyExtractor`. Behind a proxy, rate limiting is ineffective.
  - Source commands: `security-audit`
  - Action: Use `SmartIpKeyExtractor` or configure `X-Forwarded-For` reading in production.

### Security — Auth Cache Staleness Window

- [ ] **#120 — 5-minute cache TTL allows stale credentials after password change**
  - File: `src/middleware/auth.rs` lines 328–336
  - Problem: After a password change, the old password continues to work for up to 5 minutes via cache.
  - Source commands: `security-audit`
  - Action: Reduce TTL to 60s or implement cross-instance cache invalidation.

### Dependencies — `native-tls` Compiled Alongside `rustls`

- [ ] **#121 — `refinery` unconditionally enables `postgres-native-tls`**
  - Problem: Adds `native-tls` and platform TLS libraries to a project that uses `rustls` exclusively. No mitigation without upstream feature gate.
  - Source commands: `dependency-check`
  - Action: Accept compile-time cost. File upstream issue on `refinery` if desired.

### Dependencies — Unused Crypto Algorithms Compiled

- [ ] **#122 — `jsonwebtoken` `rust_crypto` feature compiles RSA, EdDSA, P-256, P-384**
  - Problem: ~30 transitive crates for algorithms never used (only HS256). No way to select HMAC-only without upstream changes.
  - Source commands: `dependency-check`
  - Action: Monitor `jsonwebtoken` for granular feature gates.

### Dependencies — Low-Activity `tracing-bunyan-formatter`

- [ ] **#123 — `tracing-bunyan-formatter` has infrequent releases**
  - Problem: Last published May 2024. Still usable but not frequently updated.
  - Source commands: `dependency-check`
  - Action: No action needed. Have `tracing-subscriber`'s built-in JSON formatter as fallback.

### Testing — Additional Coverage Gaps

- [ ] **#124 — Several test areas lack coverage: rate limiting, malformed JSON, FK cascade, `fix_migration_history`**
  - Problem: No tests for rate limiter behavior, malformed JSON body handling, FK cascade/constraint behavior on delete, or `fix_migration_history` DB interaction.
  - Source commands: `test-gaps`
  - Action: Add tests incrementally as high-risk code is modified.

### Frontend — `.unwrap()` on Event Targets in WASM

- [ ] **#125 — `ev.target().unwrap()` in input handlers could crash the WASM module**
  - File: `frontend/src/app.rs`
  - Problem: While `target()` is practically never `None` in a browser, `.unwrap()` in WASM triggers the panic hook and aborts the entire app.
  - Source commands: `review`
  - Action: Use `let Some(target) = ev.target() else { return; }` for defensive coding.

### Frontend — `Page::Dashboard` Clones Data on Every Signal Read

- [ ] **#126 — Dashboard state stored in enum variant, cloned on every re-render**
  - File: `frontend/src/app.rs`
  - Problem: `Page::Dashboard { name: String, email: String }` — every `page.get()` clones both strings.
  - Source commands: `review`
  - Action: Store dashboard state in a separate signal when the dashboard grows.

### Frontend — Missing `aria-busy` on Submit Button

- [ ] **#127 — No `aria-busy` attribute during login form submission**
  - File: `frontend/src/app.rs`
  - Problem: Button is disabled and text changes to "Signing in..." but no `aria-busy="true"` informs assistive technology.
  - Source commands: `review`
  - Action: Add `attr:aria-busy=move || loading.get()`.

### Frontend — Decorative Icons Lack Accessibility Attributes

- [ ] **#128 — Warning icon and checkmark lack `aria-hidden="true"`**
  - File: `frontend/src/app.rs` (ErrorAlert and SuccessBadge components)
  - Problem: Screen readers will announce raw Unicode character names. Adjacent text already conveys meaning.
  - Source commands: `review`
  - Action: Add `aria-hidden="true"` to the icon `<span>` elements.

### Code Quality — Missing Doc Comments on DB Functions

- [ ] **#129 — Public functions in `src/db/` lack doc comments**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Problem: Functions like `is_team_order_closed`, `get_member_role`, `is_team_admin_of_user` have nuanced behavior that warrants documentation.
  - Source commands: `review`
  - Action: Add doc comments incrementally when modifying these files.

## Completed Items

Items moved here after being resolved:

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

### Database — UUID Version Mismatch Between Schema and Application

- [x] **#69 — Schema defaults to UUID v4 but Rust code generates UUID v7**
  - Files: `migrations/V2__uuid_v7_defaults.sql` (new), `database.sql`, `init_dev_db.sh`
  - Resolution: Created V2 migration that `ALTER TABLE ... SET DEFAULT uuidv7()` on all five UUID primary key columns. Updated `database.sql` and `init_dev_db.sh`.
  - Source commands: `db-review`, `review`

### Documentation — CLAUDE.md Test Counts and References Are Stale

- [x] **#77 — Multiple stale references in CLAUDE.md**
  - Files: `CLAUDE.md` (Project Structure and Testing sections)
  - Resolution: Updated WASM test count from 22 to 23 in both sections.
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

### Deployment — Database Migration Tool Adopted

- [x] **#66 — Schema managed via destructive `DROP TABLE` DDL script**
  - Resolution: Adopted `refinery` 0.8 with versioned migrations.
  - Source commands: `db-review`, `security-audit`

### Security — In-Memory Token Blacklist Eviction

- [x] **#67 — `token_blacklist` in-memory DashMap has no eviction or size limit**
  - Resolution: Changed DashMap value to `DateTime<Utc>`, added `retain()` in cleanup task.
  - Source commands: `security-audit`, `review`

### Documentation — CLAUDE.md CSP Policy Not Documented

- [x] **#57 — CLAUDE.md Key Conventions should document the CSP header on static files**
  - Resolution: Added CSP documentation to Key Conventions.
  - Source commands: `practices-audit`, `security-audit`

### Frontend — Loading Page Spinner CSS Missing

- [x] **#58 — `LoadingPage` component references undefined CSS classes**
  - Resolution: Added CSS rules for loading page components.
  - Source commands: `review`, `practices-audit`

### Security — actix-files CVE (Verified Patched)

- [x] **#56 — `actix-files` had 2 known CVEs**
  - Resolution: Verified Cargo.lock pins patched version 0.6.10.
  - Source commands: `dependency-check`, `security-audit`

### Security — Password Hashing at User Creation

- [x] **#40 — `create_user` stores plaintext password instead of Argon2 hash**
  - Resolution: Fixed in prior session.
  - Source commands: `db-review`, `security-audit`

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

### Test Gaps

- [x] **#44 — No integration test for create-user -> authenticate round-trip**
  - Resolution: Added integration test.
  - Source command: `test-gaps`

### Backend — Error Response Consistency

- [x] **#15 — `auth_user` returns bare string instead of `ErrorResponse`**
  - Resolution: Routed through centralized `ResponseError` impl.
  - Source command: `review`

- [x] **#16 — `refresh_token` handler bypasses centralized error handling**
  - Resolution: Added `Error::Unauthorized` variant and updated handler.
  - Source command: `review`

### Frontend — Token Revocation on Logout

- [x] **#1 — Frontend logout does not revoke tokens server-side**
  - Resolution: Added `revoke_token_server_side` helper with fire-and-forget revocation.
  - Source commands: `api-completeness`, `security-audit`

### Database — Inconsistent Row Mapping Pattern

- [x] **#6 — `get_team_users` uses `.map()` instead of `filter_map` + `warn!()`**
  - Resolution: Changed to `filter_map` with `try_get()` and `warn!()`.
  - Source commands: `db-review`, `practices-audit`

- [x] **#7 — `get_user_teams` has the same `.map()` issue**
  - Resolution: Same approach as #6.
  - Source commands: `db-review`, `practices-audit`

### Architecture — Defence-in-Depth Notes

- [x] **#49 — RBAC, OpenAPI sync, and dependency health all verified correct**
  - Resolution: Migrated from `rustls-pemfile` to `rustls-pki-types`, resolved advisories via `cargo update`.
  - Source commands: `rbac-rules`, `openapi-sync`, `dependency-check`

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

### Security — Missing CSP Headers for Static Files

- [x] **#48 — No Content-Security-Policy header on static file responses**
  - Resolution: Added CSP via `DefaultHeaders` middleware.
  - Source commands: `security-audit`

### Security — Credentials Logged via `#[instrument]`

- [x] **#50 — `#[instrument]` on auth handlers doesn't skip credential parameters**
  - Resolution: Updated all `#[instrument]` annotations to skip credentials.
  - Source commands: `security-audit`, `review`

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

### Dependencies — `tokio-pg-mapper` Is Archived

- [x] **#60 — `tokio-pg-mapper` crate is unmaintained/archived**
  - Resolution: Replaced with custom `FromRow` trait in `src/from_row.rs`.
  - Source command: `dependency-check`

### Deployment — Docker Image Tags Verified Valid

- [x] **#62 — `postgres:18.3` Docker image tag — FALSE POSITIVE**
  - Resolution: Verified tag exists on Docker Hub.
  - Source commands: `dependency-check`, `review`

- [x] **#63 — `rust:1.93.1` Docker image tag — FALSE POSITIVE**
  - Resolution: Verified tag exists on Docker Hub.
  - Source commands: `dependency-check`, `review`

### Code Quality — Monolithic `src/db.rs` Refactored

- [x] **#64 — `src/db.rs` is 1,144+ lines covering all domain areas**
  - Resolution: Split into `src/db/` module directory with 9 domain files.
  - Source commands: `review`, `practices-audit`

### Dependencies — `flurry` Replaced with `dashmap`

- [x] **#65 — `flurry` 0.5.2 is unmaintained**
  - Resolution: Replaced with `dashmap` 6.1.0.
  - Source commands: `dependency-check`, `review`

### Security — HTTPS Redirect Implemented

- [x] **#72 — HTTP requests are not redirected to HTTPS**
  - Resolution: Added HTTP->HTTPS redirect server.
  - Source commands: `security-audit`

### Testing — Missing Test Coverage Areas Addressed

- [x] **#74 — Several areas lack dedicated test coverage**
  - Resolution: Added tests for from_row, openapi, healthcheck, CORS, frontend double-failure.
  - Source commands: `test-gaps`

### Documentation — Command Files Reference Stale Path

- [x] **#78 — Three command files reference `src/db.rs` instead of `src/db/`**
  - Resolution: Updated all three command files.
  - Source commands: `practices-audit`

## Notes

- All 170 backend unit tests pass (148 lib + 22 healthcheck); 65 API integration tests pass; 86 DB integration tests pass; 23 WASM tests pass. Total: 344 tests, 0 failures.
- Backend unit test breakdown: config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, db/migrate: 34, healthcheck: 22 = **170 total**.
- `cargo audit` reports 1 unfixable vulnerability: `rsa` 0.9.10 via `jsonwebtoken` (RUSTSEC-2023-0071). Not exploitable (only HS256 used).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` is clean on both crates.
- RBAC enforcement is correct across all handlers per the policy table.
- OpenAPI spec is synchronized with routes (41 operations, 27 schemas) — only cosmetic path parameter naming differences.
- All 10 assessment commands run: `api-completeness`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 1 critical (#85), 8 important (#86-#93), 27 minor (6 carried + 21 new), 19 informational (4 carried + 15 new). Total: 55 open items.
- 45 completed items preserved from prior assessments.
