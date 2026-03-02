# Assessment Findings

Last assessed: 2026-03-02 (full re-assessment round 2 — 56 new findings: #130–#185; #55/#122 superseded by #132; #125 upgraded to Important; #108 resolved)

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will move completed items to `.claude/resolved-findings.md`, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

### Database — `update_team_order` Can Set `closed` to NULL

- [ ] **#130 — Sending `null` for `closed` bypasses `guard_open_order` (which treats NULL as open via `.unwrap_or(false)`)**
  - Files: `src/db/orders.rs` (UPDATE query), `src/models.rs` (`UpdateTeamOrderEntry`)
  - Problem: `UpdateTeamOrderEntry.closed` is `Option<bool>`. When `closed` is `None`, the SQL `SET closed = $3` writes NULL to the DB. `guard_open_order` uses `.unwrap_or(false)` — so NULL counts as "open." An attacker who is a team member could re-open a closed order.
  - Fix: Use `COALESCE($3, closed)` in the SQL so NULL preserves the existing value, or make `closed` a required `bool` in `UpdateTeamOrderEntry`.
  - Source commands: `db-review`, `review`

### Database — Missing Index on `orders.orders_item_id`

- [ ] **#131 — FK RESTRICT lookups require sequential scan after V3 changed CASCADE→RESTRICT**
  - Files: `migrations/V3__indexes_constraints.sql`, `migrations/V1__initial_schema.sql`
  - Problem: V3 changed the FK on `orders.orders_item_id` from CASCADE to RESTRICT. When deleting an item, PostgreSQL must verify no orders reference it. The composite PK `(orders_teamorders_id, orders_item_id)` cannot serve this lookup because `orders_item_id` is the second column.
  - Fix: Add `CREATE INDEX IF NOT EXISTS idx_orders_item ON orders (orders_item_id);` in a V4 migration.
  - Source commands: `db-review`

### Dependencies — `jsonwebtoken` Pulls Vulnerable and Unnecessary Crypto Crates

- [ ] **#132 — `rust_crypto` feature enables ~15 unused crates including vulnerable `rsa` (RUSTSEC-2023-0071); granular `["hmac", "sha2"]` features are available**
  - File: `Cargo.toml` (jsonwebtoken dependency)
  - Problem: `features = ["rust_crypto"]` pulls `rsa`, `ed25519-dalek`, `p256`, `p384`, `rand` — none of which are used (only HS256). The `rsa` crate has an unfixable timing side-channel advisory. `jsonwebtoken` 10.3.0 supports individual feature selection.
  - Fix: Change `features = ["rust_crypto"]` to `features = ["hmac", "sha2"]`. This eliminates the advisory and removes ~15 crates from the dependency tree.
  - Source commands: `dependency-check`

## Important Items

### Database — Nullable Timestamp Columns Across All Tables

- [ ] **#133 — `created` and `changed` columns lack NOT NULL; Rust models use non-Optional types**
  - File: `migrations/V1__initial_schema.sql` (users, teams, roles, items, teamorders)
  - Problem: All timestamp columns use `DEFAULT CURRENT_TIMESTAMP` but no `NOT NULL`. An explicit NULL insert would cause a `FromRow` conversion error at runtime since the Rust models use `DateTime<Utc>` (non-optional).
  - Fix: V4 migration: `ALTER TABLE ... ALTER COLUMN created SET NOT NULL` and same for `changed` on all 5 entity tables.
  - Source commands: `db-review`

### Database — `items.price` Allows NULL

- [ ] **#134 — Item without a price makes order totals impossible to calculate**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs` (`ItemEntry`, `CreateItemEntry`, `UpdateItemEntry`)
  - Problem: `price numeric(10,2) CHECK (price >= 0)` has no NOT NULL. Rust models use `Option<Decimal>`.
  - Fix: Add NOT NULL to schema and change Rust type from `Option<Decimal>` to `Decimal`.
  - Source commands: `db-review`

### Database — `orders.amt` Allows NULL

- [ ] **#135 — Order item without a quantity is meaningless**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs` (`OrderEntry`, `CreateOrderEntry`, `UpdateOrderEntry`)
  - Problem: `amt int CHECK (amt >= 0)` has no NOT NULL. Rust models use `Option<i32>`.
  - Fix: Add `NOT NULL DEFAULT 1` to schema and change Rust type from `Option<i32>` to `i32`.
  - Source commands: `db-review`

### Database — `orders` Table Has No Timestamps

- [ ] **#136 — Unlike every other entity table, `orders` lacks `created`/`changed` columns**
  - File: `migrations/V1__initial_schema.sql` (orders table definition)
  - Problem: No audit trail for when order items were added or modified.
  - Fix: V4 migration: add `created` and `changed` columns with NOT NULL defaults and BEFORE UPDATE trigger, consistent with other tables.
  - Source commands: `db-review`

### Error Handling — Fragile 404 Detection via String Matching

- [ ] **#137 — 404 detection relies on matching `"query returned an unexpected number of rows"` string from tokio-postgres**
  - File: `src/errors.rs` (Error::Db handler)
  - Problem: If tokio-postgres ever changes this error message wording, all 404 responses silently degrade to 500s.
  - Fix: Use `query_opt` + explicit `Error::NotFound` in single-row DB functions, or match on the error kind instead of the string.
  - Source commands: `db-review`

### Documentation — `database.sql` Diverged from Migrations

- [ ] **#138 — Deprecated `database.sql` is out of sync with V3 migration**
  - File: `database.sql`
  - Problem: Still uses CASCADE (V3 changed to RESTRICT), still creates `idx_orders_tid` (V3 drops it), missing NOT NULL on `memberof_role_id`, missing V3 indexes. Developers using it get a different schema than production.
  - Fix: Update to match post-V3 schema, or remove the file entirely.
  - Source commands: `db-review`

### OpenAPI — Spurious Query Params on `create_user`

- [ ] **#139 — `params(CreateUserEntry)` in utoipa annotation renders body fields as query parameters in Swagger UI**
  - File: `src/handlers/users.rs` (`create_user` utoipa path annotation)
  - Problem: `CreateUserEntry` derives `IntoParams`. Its fields (firstname, lastname, email, password) appear as query parameters alongside the request body.
  - Fix: Remove `params(CreateUserEntry)` from the annotation. Remove `IntoParams` from the derive.
  - Source commands: `openapi-sync`

### OpenAPI — Spurious Query Params on `update_user`

- [ ] **#140 — `params(("user_id", ...), UpdateUserRequest)` renders body fields as query parameters**
  - File: `src/handlers/users.rs` (`update_user` utoipa path annotation)
  - Problem: Same issue as #139 — `UpdateUserRequest` appears as query params alongside the body.
  - Fix: Change to `params(("user_id", ...))` only. Remove `IntoParams` from `UpdateUserRequest`.
  - Source commands: `openapi-sync`

### OpenAPI — Missing 422 Response on Validated Endpoints

- [ ] **#141 — 12 handlers call `validate(&json)?` but none document 422 in utoipa annotations**
  - Files: `src/handlers/users.rs`, `src/handlers/teams.rs`, `src/handlers/items.rs`, `src/handlers/roles.rs`, `src/handlers/orders.rs`
  - Problem: Validation errors return HTTP 422 via `ErrorResponse`, but Swagger UI consumers don't see this documented response.
  - Fix: Add `(status = 422, description = "Validation error", body = ErrorResponse)` to each handler's `responses(...)`.
  - Source commands: `openapi-sync`

### Security — No Minimum JWT Secret Length in Production

- [ ] **#142 — Operator could set `BREAKFAST_SERVER_JWTSECRET=abc` and the server would accept it**
  - Files: `src/server.rs` (production checks), `config/default.yml`
  - Problem: The server panics on default secret values in production, but imposes no minimum length. HS256 security requires at least 256 bits (32 bytes) of entropy.
  - Fix: Add a runtime check that JWT secret is ≥32 characters in production.
  - Source commands: `security-audit`

### Security — Argon2 Parameters Rely on Crate Defaults

- [ ] **#143 — A dependency update could silently weaken hashing parameters**
  - Files: `src/db/users.rs`, `src/middleware/auth.rs`
  - Problem: `Argon2::default()` is used for hashing and verification. While current defaults match OWASP recommendations, they could change.
  - Fix: Explicitly construct `Argon2::new(Algorithm::Argon2id, Version::V0x13, params)` in a shared constant.
  - Source commands: `security-audit`

### Security — `auth_user` Cache Hit Path Bypasses Password Verification

- [ ] **#144 — Handler generates tokens from cache without re-verifying password; middleware verifies but code path is misleading**
  - File: `src/handlers/users.rs` (`auth_user` handler)
  - Problem: On cache hit, a token pair is generated immediately without password check. The `basic_validator` middleware verifies first, but if middleware ordering changes, this becomes a critical auth bypass.
  - Fix: Remove the redundant cache check in the handler body. Generate token pair from the middleware-authenticated identity.
  - Source commands: `security-audit`, `review`

### Security — No Production Panic for Default DB Credentials

- [ ] **#145 — Default Postgres credentials `actix/actix` used with no startup validation (unlike server/JWT secrets)**
  - Files: `config/default.yml`, `src/server.rs`
  - Problem: A misconfigured production deploy would silently use development credentials.
  - Fix: Add production-panic for default DB credentials similar to the existing secret checks.
  - Source commands: `security-audit`

### Frontend — `.unwrap()` on Event Targets in WASM

- [ ] **#125 — `ev.target().unwrap()` in input handlers could crash the WASM module (upgraded from informational)**
  - File: `frontend/src/app.rs` (UsernameField and PasswordField components)
  - Problem: A panic in WASM kills the entire SPA. The `target()` call returns `Option` and is unwrapped without graceful handling.
  - Fix: Use `let Some(target) = ev.target() else { return; };`.
  - Source commands: `review`

### Code Quality — Double DB Client Acquisition in `revoke_user_token`

- [ ] **#147 — Handler acquires two pool connections when one would suffice**
  - File: `src/handlers/users.rs` (`revoke_user_token`)
  - Problem: The handler acquires a client for the admin check, drops it, then acquires a second for the revocation. The first client could be reused.
  - Fix: Reuse the first `Client` for both the admin check and the token revocation.
  - Source commands: `review`, `practices-audit`, `rbac-rules`

### Code Quality — `Claims.token_type` Uses `String` Instead of Typed Enum

- [ ] **#148 — `token_type` field only ever holds `"access"` or `"refresh"` but uses `String`**
  - Files: `src/models.rs` (`Claims`), `src/middleware/auth.rs`
  - Problem: A typo or invalid value would compile and only fail at runtime. String comparisons are scattered across auth.rs and handlers/users.rs.
  - Fix: Define a `TokenType` enum with serde serialization.
  - Source commands: `review`

### Dependencies — `leptos` Patch Update Available

- [ ] **#149 — `leptos` 0.8.16 resolved, 0.8.17 available**
  - File: `frontend/Cargo.toml`
  - Problem: Patch release likely contains bug fixes.
  - Fix: Run `cargo update -p leptos`.
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

### RBAC — Helpers Return 403 Instead of 401 for Missing Claims

- [ ] **#150 — All six RBAC helpers use `Error::Forbidden("Authentication required")` — should be 401 per RFC 9110**
  - File: `src/handlers/mod.rs` (all RBAC helpers)
  - Problem: "Authentication required" is a 401 concern, not 403. Mitigated by JWT middleware blocking unauthenticated requests first — this code path is unreachable in practice.
  - Fix: Change to `Error::Unauthorized("Authentication required")`.
  - Source commands: `rbac-rules`

### Code Quality — Middleware Auth Uses Inline `json!()` Instead of `ErrorResponse`

- [ ] **#151 — ~15 error responses in auth validators use `json!({"error":"..."})` instead of the `ErrorResponse` struct**
  - File: `src/middleware/auth.rs` (`jwt_validator`, `refresh_validator`, `basic_validator`)
  - Problem: If `ErrorResponse` gains additional fields, these responses would diverge.
  - Fix: Replace `json!({"error":"..."})` with `ErrorResponse { error: "...".into() }` in all auth validators.
  - Source commands: `practices-audit`

### OpenAPI — Unnecessary `IntoParams` Derives on Request Body Structs

- [ ] **#152 — `CreateUserEntry`, `UpdateUserRequest`, `UpdateUserEntry` derive `IntoParams` but are only used as JSON bodies**
  - File: `src/models.rs`
  - Problem: Enables the erroneous `params()` usage in #139/#140. These structs are never used as query parameters.
  - Fix: Remove `IntoParams` from these three derives.
  - Source commands: `openapi-sync`

### OpenAPI — `RevokedResponse` Not Explicitly Registered in Schema Components

- [ ] **#153 — Auto-discovered by utoipa but not listed in `components(schemas(...))`**
  - File: `src/middleware/openapi.rs`
  - Problem: Inconsistent with the convention of explicit schema registration (all other schemas are listed).
  - Fix: Add `RevokedResponse` to the `components(schemas(...))` list.
  - Source commands: `openapi-sync`

### Security — No Maximum Password Length Validation

- [ ] **#154 — `CreateUserEntry.password` enforces `min = 8` but has no maximum; enables HashDoS**
  - Files: `src/models.rs` (`CreateUserEntry`, `validate_optional_password`)
  - Problem: An attacker could submit a multi-megabyte password string, causing excessive CPU during Argon2 hashing.
  - Fix: Add `max = 128` (or 1024) to password validation.
  - Source commands: `security-audit`

### Security — JSON Payload Size Limit Only on API Scope

- [ ] **#155 — `/auth/revoke` endpoint uses actix-web default 256 KiB limit instead of the 64 KiB limit on `/api/v1.0`**
  - File: `src/routes.rs`
  - Problem: The `JsonConfig::default().limit(65_536)` is only applied within the `/api/v1.0` scope.
  - Fix: Apply `JsonConfig` with size limit to the `/auth/revoke` resource as well.
  - Source commands: `security-audit`

### Security — Password Hash Stored in Auth Cache

- [ ] **#156 — `UpdateUserEntry` including the Argon2 hash is stored in the `DashMap` cache**
  - Files: `src/models.rs`, `src/middleware/auth.rs`
  - Problem: Keeping password hashes in memory increases blast radius of memory-disclosure vulnerabilities.
  - Fix: Use a distinct `AuthUser` struct for the cache that is never `Serialize`.
  - Source commands: `security-audit`

### Security — No Rate Limiting on `/auth/revoke`

- [ ] **#157 — `/auth` and `/auth/refresh` have rate limiting but `/auth/revoke` does not**
  - File: `src/routes.rs`
  - Problem: An attacker with a valid token could flood the revocation endpoint, causing excessive DB writes.
  - Fix: Apply the same `auth_rate_limit` governor to `/auth/revoke`.
  - Source commands: `security-audit`

### Code Quality — `get_client` Takes Pool by Value

- [ ] **#158 — `pub async fn get_client(pool: Pool)` forces clone at every call site**
  - File: `src/handlers/mod.rs`
  - Problem: While `Pool` is Arc-based and cheap to clone, idiomatic Rust accepts `&Pool`.
  - Fix: Change signature to `&Pool`.
  - Source commands: `review`

### Code Quality — Commented-Out Error Variant

- [ ] **#159 — Dead `RustlsPEMError` block in `errors.rs`**
  - File: `src/errors.rs`
  - Problem: Commented-out code adds noise.
  - Fix: Remove the dead code.
  - Source commands: `review`

### Code Quality — `check_db` Uses `execute` for `SELECT 1`

- [ ] **#160 — `client.execute(SELECT 1)` returns row count; `query_one` is more idiomatic**
  - File: `src/db/health.rs`
  - Fix: Use `client.query_one(&statement, &[]).await` instead.
  - Source commands: `review`

### Code Quality — Unnecessary `return` Keyword

- [ ] **#161 — `return Err(Error::Unauthorized(...))` in `auth_user` is redundant**
  - File: `src/handlers/users.rs`
  - Fix: Remove the `return` keyword — it's the final expression in the block.
  - Source commands: `review`

### Database — `memberof` Table Lacks `changed` Timestamp

- [ ] **#162 — No audit trail for role changes**
  - File: `migrations/V1__initial_schema.sql` (memberof table)
  - Problem: The table only has `joined`. When a member's role is updated, there's no record of when.
  - Fix: Add `changed timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP` with an update trigger.
  - Source commands: `db-review`

### Documentation — Frontend Test Category Breakdown Sums to 21, Not 23

- [ ] **#163 — CLAUDE.md test category breakdown omits 2 token refresh tests**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Problem: 8 categories total 4+3+3+3+2+1+2+3 = 21, but 23 WASM tests exist. Missing: `test_authed_get_retries_after_401_with_token_refresh` and `test_authed_get_double_failure_falls_back_to_login`.
  - Fix: Add "Token refresh (2 tests)" category to the breakdown.
  - Source commands: `cross-ref-check`

### Documentation — `test-gaps.md` References `gloo_timers` Which Project Doesn't Use

- [ ] **#164 — Command recommends `gloo_timers::future::sleep` but project uses custom `flush()` helper**
  - File: `.claude/commands/test-gaps.md`
  - Problem: Misleading test instruction. The project uses a Promise-based setTimeout wrapper, not `gloo-timers`.
  - Fix: Replace with instruction to use the `flush(ms)` helper.
  - Source commands: `cross-ref-check`

### Documentation — Integration Test Doc Comments Reference Deprecated `database.sql`

- [ ] **#165 — Both `api_tests.rs` and `db_tests.rs` reference `database.sql` for setup**
  - Files: `tests/api_tests.rs`, `tests/db_tests.rs` (line 3 doc comments)
  - Problem: DB is now initialized via Refinery migrations + `database_seed.sql`.
  - Fix: Update doc comments to reference migrations and `database_seed.sql`.
  - Source commands: `cross-ref-check`

### Documentation — `middleware/mod.rs` Missing from CLAUDE.md Structure Tree

- [ ] **#166 — Tree lists `auth.rs` and `openapi.rs` under `middleware/` but omits `mod.rs`**
  - File: `CLAUDE.md` (Project Structure section)
  - Problem: Inconsistent with `db/mod.rs` and `handlers/mod.rs` which are listed.
  - Fix: Add `mod.rs — Module declarations` under `middleware/`.
  - Source commands: `cross-ref-check`

### Code Quality — Missing `#[must_use]` on Auth Functions

- [ ] **#167 — `generate_token_pair`, `verify_jwt`, `invalidate_cache` return values that should not be ignored**
  - File: `src/middleware/auth.rs`
  - Fix: Add `#[must_use]` to these functions.
  - Source commands: `review`

### Dependencies — Redundant `features = ["default"]` on Crates

- [ ] **#168 — `argon2` and `opentelemetry` specify `features = ["default"]` which is a no-op**
  - File: `Cargo.toml`
  - Fix: Simplify to plain version strings.
  - Source commands: `dependency-check`

### Dependencies — Unnecessary Braces on Simple Dependencies

- [ ] **#169 — `actix-web-httpauth`, `tracing-log`, `rustls-pki-types` use `{ version = "..." }` with no other keys**
  - File: `Cargo.toml`
  - Fix: Simplify to plain version strings.
  - Source commands: `dependency-check`

### Security — Missing `X-Frame-Options` Header

- [ ] **#170 — CSP `frame-ancestors 'none'` covers modern browsers but `X-Frame-Options: DENY` is missing for older browsers**
  - File: `src/server.rs` (DefaultHeaders)
  - Fix: Add `.add(("X-Frame-Options", "DENY"))`.
  - Source commands: `security-audit`

### Testing — `AddMemberEntry` and `UpdateMemberRoleEntry` Lack `Validate` Derive

- [ ] **#171 — These models are deserialized from request bodies but `validate()` is a no-op since they don't derive `Validate`**
  - File: `src/models.rs`
  - Problem: Fields are UUIDs (type-enforced by serde), so low risk, but inconsistent with other models.
  - Fix: Add `Validate` derive or remove the `validate()` call in the handlers.
  - Source commands: `test-gaps`

## Informational Items

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

### Testing — `validate_optional_password` Has No Unit Tests

- [ ] **#172 — Custom validator for `UpdateUserRequest.password` has zero test coverage**
  - File: `src/models.rs` (`validate_optional_password`)
  - Problem: If this validator silently passes short passwords, users could set weak passwords via PUT. The function uses a non-standard `&String` signature required by the `validator` crate.
  - Source commands: `test-gaps`
  - Action: Add tests for `Some("short")` → error, `Some("validpass")` → pass, `None` → skip.

### Testing — No API Test for `user_teams` Endpoint

- [ ] **#173 — `GET /api/v1.0/users/{user_id}/teams` has no API-level integration test**
  - Files: `tests/api_tests.rs`, `src/handlers/users.rs`
  - Problem: Tested at DB level but no API test verifies JSON shape, JWT requirement, or empty-array behavior.
  - Source commands: `test-gaps`
  - Action: Add `get_user_teams_returns_empty_array`, `get_user_teams_returns_memberships`, `get_user_teams_requires_jwt`.

### Testing — `check_team_access` Combined RBAC Query Has No Direct Test

- [ ] **#174 — Core RBAC query tested only indirectly through API-level tests**
  - File: `src/db/membership.rs` (`check_team_access`)
  - Problem: Returns `(is_admin, team_role)` tuple via correlated subquery + EXISTS. A subtle SQL bug could be masked.
  - Source commands: `test-gaps`
  - Action: Add 4 direct DB tests: admin in team, member, non-member, admin not in team.

### Testing — No Test for Malformed Path Parameters

- [ ] **#175 — `GET /api/v1.0/users/not-a-uuid` → 400 path is untested**
  - Files: `tests/api_tests.rs`, `src/errors.rs` (`path_error_handler`)
  - Source commands: `test-gaps`
  - Action: Add `get_user_with_invalid_uuid_returns_400`.

### Testing — No Test for JSON Error Handler

- [ ] **#176 — Oversized/malformed JSON body error paths are untested**
  - Files: `tests/api_tests.rs`, `src/errors.rs` (`json_error_handler`)
  - Problem: Three sub-cases: ContentType → 415, deserialization → 422, other → 400. None tested.
  - Source commands: `test-gaps`
  - Action: Add `create_user_with_wrong_content_type_returns_415`, `create_user_with_invalid_json_returns_400`.

### Testing — No API Tests for `update_team` and `update_role` Success Paths

- [ ] **#177 — Admin happy path untested; only rejection path (`non_admin_cannot_*`) exists**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`
  - Action: Add `update_team_as_admin_returns_200`, `update_role_as_admin_returns_200`.

### Testing — No Tests for `Location` Header in Create Responses

- [ ] **#178 — All create handlers build `Location` header via `url_for` but no test verifies it**
  - Files: `tests/api_tests.rs`, `src/handlers/` (all create handlers)
  - Problem: If the named route string drifts, `url_for` silently fails (wrapped in `if let Ok`).
  - Source commands: `test-gaps`
  - Action: Add `create_user_sets_location_header`.

### Testing — No Rate Limiting Behavior Test

- [ ] **#179 — No test verifies the 11th rapid auth request returns 429**
  - Files: `tests/api_tests.rs`, `src/routes.rs` (governor config)
  - Source commands: `test-gaps`
  - Action: Add `auth_endpoint_rate_limits_after_burst`.

### Testing — No Validation Tests for Order-Related Models

- [ ] **#180 — `CreateOrderEntry`, `UpdateOrderEntry`, `CreateTeamOrderEntry`, `UpdateTeamOrderEntry` derive `Validate` but have no tests**
  - File: `src/models.rs`
  - Source commands: `test-gaps`
  - Action: Add basic validation tests to catch regressions if rules are added.

### Testing — No Test for Error Response Body Shape

- [ ] **#181 — Tests verify status codes but never assert response body matches `{"error": "..."}`**
  - File: `src/errors.rs`
  - Problem: A serialization change could break API clients.
  - Source commands: `test-gaps`
  - Action: Add `error_response_body_is_json_with_error_field`.

### Code Quality — `UpdateUserEntry` Serves Dual Purpose

- [ ] **#183 — Struct used for both auth cache and DB row mapping**
  - File: `src/models.rs`
  - Problem: Includes `password` hash (needed for cache verification) and derives `Validate` with password min-length rules (applies to plaintext, not hash).
  - Source commands: `review`
  - Action: Consider a dedicated `CachedUserData` type for the auth cache.

### Frontend — `authed_get` Only Supports GET

- [ ] **#184 — Future pages need `authed_post`, `authed_put`, `authed_delete` variants**
  - File: `frontend/src/app.rs`
  - Source commands: `review`
  - Action: Build generic `authed_request(method, url, body?)` when implementing the next frontend page.

### Deployment — Healthcheck Binary Hardcodes Port 8080

- [ ] **#185 — `let port = 8080;` is hardcoded in the healthcheck binary**
  - File: `src/bin/healthcheck.rs`
  - Problem: Production with a different port would cause healthcheck failures.
  - Source commands: `review`
  - Action: Read port from environment or config.

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of 73 resolved findings.

## Notes

- All 170 backend unit tests pass (148 lib + 22 healthcheck); 67 API integration tests pass; 86 DB integration tests pass; 23 WASM tests pass. Total: 346 tests, 0 failures.
- Backend unit test breakdown: config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, db/migrate: 34, healthcheck: 22 = **170 total**.
- `cargo audit` reports 1 vulnerability: `rsa` 0.9.10 via `jsonwebtoken` (RUSTSEC-2023-0071). **Fixable** — see #132 (switch to `["hmac", "sha2"]` features).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` is clean on both crates.
- RBAC enforcement is correct across all handlers per the policy table.
- OpenAPI spec is synchronized with routes (41 operations) — minor issues documented in #139, #140, #141, #152, #153.
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 3 critical (#130–#132), 17 important (#125, #133–#145, #147–#149), 30 minor (8 carried + 22 new), 29 informational (16 carried + 13 new). Total: 79 open items.
- 73 resolved items moved to `.claude/resolved-findings.md` (4 critical, 22 important, 42 minor, 5 informational).
