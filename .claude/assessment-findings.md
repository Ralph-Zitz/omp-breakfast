# Assessment Findings

Last assessed: 2026-03-03

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

### Bug — 5 Update DB Functions Return HTTP 500 Instead of 404 for Missing Resources

- [ ] **#212 — `update_team`, `update_role`, `update_item`, `update_team_order`, `update_order_item` use `query_one` which maps not-found to 500**
  - Files: `src/db/teams.rs` line 124 (`update_team`), `src/db/roles.rs` line 89 (`update_role`), `src/db/items.rs` line 83 (`update_item`), `src/db/orders.rs` line 113 (`update_team_order`), `src/db/order_items.rs` line 176 (`update_order_item`)
  - Problem: These five update functions use `client.query_one(...)` which returns a `tokio_postgres::Error` ("query returned an unexpected number of rows") when the UPDATE affects zero rows (record doesn't exist). This maps to `Error::Db` → HTTP 500. By contrast, `update_user` in `src/db/users.rs` correctly uses `query_opt()` + `.ok_or_else(|| Error::NotFound(...))` to return 404. The OpenAPI specs for these endpoints document 404 responses that can never actually occur.
  - Fix: Change all five functions to use `query_opt(...)` with `.ok_or_else(|| Error::NotFound("... not found"))`, matching the `update_user` pattern.
  - Source commands: `review`, `db-review`

### Security — User Enumeration via Authentication Timing Side-Channel

- [ ] **#213 — Non-existent users return ~1ms vs ~100ms for wrong-password on existing users**
  - File: `src/middleware/auth.rs` lines 378–387 (`basic_validator` cache-miss error branch)
  - Problem: When a non-existent user attempts Basic Auth, the cache miss triggers a `get_user_by_email` DB lookup that fails, and the function returns immediately (~1–5ms). When an existing user provides a wrong password, the lookup succeeds and Argon2id verification runs (~100–300ms) before returning 401. This ~100x timing difference allows an attacker to enumerate valid email addresses by measuring response times.
  - Fix: When `get_user_by_email` returns `Err` (user not found), perform a dummy `argon2_hasher().verify_password(b"dummy", &dummy_hash)` against a pre-computed static hash before returning 401. This equalizes response time regardless of whether the email exists.
  - Source commands: `security-audit`

### Testing — No Test for Admin Role Escalation Guard

- [ ] **#214 — Both `add_team_member` and `update_member_role` have escalation guards but no test exercises them**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs` lines 361–371 (`add_team_member` guard), lines 439–449 (`update_member_role` guard)
  - Problem: Both handlers contain identical logic preventing non-admin users from assigning the "Admin" role. This is a critical security guard — a Team Admin could escalate to global Admin if this logic regressed. **No API or DB test exercises this path.** The existing `team_admin_can_add_and_remove_member` test uses the "Member" role only.
  - Fix: Add `team_admin_cannot_assign_admin_role_via_add_member` (POST with role_id of "Admin" → 403) and `team_admin_cannot_assign_admin_role_via_update_role` (PUT with role_id of "Admin" → 403).
  - Source commands: `test-gaps`, `rbac-rules`

### Testing — No Test for Password Update → Re-Login Round-Trip

- [ ] **#215 — Password change via PUT is never tested with subsequent authentication**
  - Files: `tests/api_tests.rs`, `src/handlers/users.rs` line 305 (`update_user`), `src/db/users.rs`
  - Problem: `create_user_then_authenticate_round_trip` proves Argon2 hashing works for user creation, but no test updates a password via `PUT /api/v1.0/users/{id}` with `"password": "newpassword"` then authenticates with the new password. If the `update_user` DB function's password-hashing branch had a bug, users would be locked out after password changes.
  - Fix: Add test: `update_user_password_then_reauthenticate` — update password, login with new password (200), login with old password (401).
  - Source commands: `test-gaps`

## Minor Items

### Code Quality — Dead S3 Config Fields

- [ ] **#59 — `s3_key_id` and `s3_key_secret` are loaded and stored but never used**
  - Files: `src/models.rs` lines 60–61 (`State` struct), `src/config.rs` lines 13–14 (`ServerConfig` struct), `src/server.rs` lines 351–352 (state construction), `config/default.yml` lines 6–7, `config/development.yml` lines 5–6, `config/production.yml` lines 6–7
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
  - File: `src/routes.rs` line 28
  - Problem: In production, this exposes the complete API schema, aiding attacker reconnaissance.
  - Fix: Conditionally register the Swagger UI scope only when `ENV != production`, or gate behind admin auth.
  - Source commands: `security-audit`

### Performance — Auth Cache Eviction Is O(n log n)

- [ ] **#113 — Cache eviction sorts all entries on every miss at capacity**
  - File: `src/middleware/auth.rs` lines 352–365
  - Problem: When the cache is full (1000 entries), every miss collects all entries into a `Vec`, sorts by timestamp, and removes the oldest 10%. This is O(n log n) per miss.
  - Fix: Use a proper LRU data structure (e.g., `lru` crate) or a min-heap.
  - Source commands: `review`

### Documentation — Frontend Test Category Breakdown Sums to 21, Not 23

- [ ] **#163 — CLAUDE.md test category breakdown omits 2 token refresh tests**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Problem: 8 categories total 4+3+3+3+2+1+2+3 = 21, but 23 WASM tests exist. Missing: `test_authed_get_retries_after_401_with_token_refresh` and `test_authed_get_double_failure_falls_back_to_login`.
  - Fix: Add "Token refresh (2 tests)" category to the breakdown.
  - Source commands: `cross-ref-check`

### Documentation — 4 Stale `localStorage` References in Command Files

- [ ] **#194 — Command files reference `localStorage` but the project uses `sessionStorage`**
  - Files: `.claude/commands/review.md` line 25, `.claude/commands/test-gaps.md` line 30, `.claude/commands/security-audit.md` lines 48 and 50
  - Problem: Four references to `localStorage` in command instructions are stale — the project uses `sessionStorage` for token storage.
  - Fix: Replace all 4 occurrences of `localStorage` with `sessionStorage`.
  - Source commands: `cross-ref-check`

### Database — `INSERT` Trigger on Users Table Should Be `UPDATE` Only

- [ ] **#195 — `update_users_changed_at` fires on `BEFORE INSERT OR UPDATE` — the INSERT trigger is unnecessary**
  - File: `migrations/V1__initial_schema.sql` lines 149–152
  - Problem: The trigger fires on INSERT where `OLD` is NULL, but `ROW(NEW.*) IS DISTINCT FROM ROW(OLD.*)` works "by accident" since NULL comparisons. The INSERT path is redundant because `DEFAULT CURRENT_TIMESTAMP` already sets `changed` on insert. All other tables' triggers are `BEFORE UPDATE` only.
  - Fix: Add a new migration to change the trigger to `BEFORE UPDATE ON users` only.
  - Source commands: `db-review`

### Validation — No Positive-Value Validation on Item Prices

- [ ] **#196 — `CreateItemEntry.price` and `UpdateItemEntry.price` accept negative prices at the API layer**
  - File: `src/models.rs` lines 276–293
  - Problem: `price: rust_decimal::Decimal` has no validation. The DB has `CHECK (price >= 0)` but negative prices from the API would only be caught at the DB layer with an unfriendly error.
  - Fix: Add a custom validator ensuring `price >= 0`, or document the DB constraint as the sole enforcement point.
  - Source commands: `db-review`, `security-audit`

### Validation — No Max Length on Text Fields

- [ ] **#197 — `tname`, `descr`, `title` fields have `min = 1` validation but no `max` length**
  - File: `src/models.rs` (all Create/Update entry structs for teams, roles, items)
  - Problem: Text fields backed by PostgreSQL `text` type have no upper bound. A client could submit arbitrarily long strings.
  - Fix: Add `max = 255` (or another reasonable limit) to the `#[validate(length(...))]` attributes.
  - Source commands: `security-audit`

### Code Quality — `check_db` Can Only Return `Ok(true)` — Dead Code Branch

- [ ] **#198 — `get_health` handler's `Ok(false)` branch is unreachable**
  - Files: `src/db/health.rs` lines 4–12, `src/handlers/mod.rs` lines 289–303
  - Problem: `check_db` does `.map(|_| true)` on success and `.map_err(Error::Db)` on error, so it never returns `Ok(false)`. The handler's `else` branch (503) is dead code.
  - Fix: Change `check_db` to return `Result<(), Error>` or simplify the handler's match.
  - Source commands: `review`

### Code Quality — Commented-Out Code in `get_health`

- [ ] **#199 — Dead commented-out `let client: Client = ...` line in health handler**
  - File: `src/handlers/mod.rs` line 290
  - Problem: Dead code.
  - Fix: Remove the line.
  - Source commands: `review`

### Code Quality — `validate.rs` Only Reports First Error Per Field

- [ ] **#200 — Multiple validation failures per field are silently dropped**
  - File: `src/validate.rs` line 22
  - Problem: `error.1[0]` takes only the first validation error for each field. If a field has both "too short" and "invalid format" errors, only one is returned.
  - Fix: Iterate over all errors in `error.1` or document the behavior.
  - Source commands: `review`

### Code Quality — Missing `#[must_use]` on `validate()` Function

- [ ] **#201 — If a caller omits `?`, validation would be silently skipped**
  - File: `src/validate.rs` line 6
  - Problem: Returns `Result<(), Error>` but isn't marked `#[must_use]`.
  - Fix: Add `#[must_use = "validation result must be checked"]`.
  - Source commands: `review`

### Database — `teamorders.teamorders_user_id` Is Nullable but Never NULL

- [ ] **#202 — No code path creates orders without a user, but the DB allows it**
  - File: `migrations/V1__initial_schema.sql` line 73
  - Problem: `teamorders_user_id` is nullable. No handler creates orders without setting this field. If a NULL-user order is somehow created, it would be orphaned.
  - Fix: Add `NOT NULL` constraint via a new migration, or document why NULL is valid.
  - Source commands: `db-review`

### Code Quality — Admin Role Escalation Guard Duplicated Verbatim

- [ ] **#216 — Identical 11-line guard block in `add_team_member` and `update_member_role`**
  - File: `src/handlers/teams.rs` lines 361–371 and lines 439–449
  - Problem: Both handlers contain identical code that prevents Team Admins from assigning the global Admin role (checks `is_admin`, then `get_role`, then rejects if `role.title == ROLE_ADMIN`).
  - Fix: Extract into a helper `guard_admin_role_assignment(client, req, role_id)` in `handlers/mod.rs`.
  - Source commands: `review`

### Database — `update_team_order` Has Inconsistent Partial-Update Semantics

- [ ] **#217 — COALESCE used only on `closed` but not on `teamorders_user_id` or `duedate`**
  - File: `src/db/orders.rs` lines 103–104
  - Problem: The UPDATE query sets `teamorders_user_id = $1, duedate = $2, closed = COALESCE($3, closed)`. The `closed` field preserves current value when NULL, but `teamorders_user_id` and `duedate` are overwritten unconditionally. If a client sends `{"closed": true}` without resending `teamorders_user_id` and `duedate`, those fields are silently set to NULL — data loss. All three fields are `Option` in `UpdateTeamOrderEntry`, suggesting partial-update intent.
  - Fix: Apply COALESCE to all three: `SET teamorders_user_id = COALESCE($1, teamorders_user_id), duedate = COALESCE($2, duedate), closed = COALESCE($3, closed)`.
  - Source commands: `db-review`

### Practices — `add_team_member` and `update_member_role` Skip `validate(&json)?`

- [ ] **#218 — Two handlers accept JSON body without calling validate(), violating project convention**
  - File: `src/handlers/teams.rs` lines 355 (`add_team_member`), 432 (`update_member_role`)
  - Problem: Both handlers call `json.into_inner()` without prior `validate(&json)?`. Both model structs (`AddMemberEntry`, `UpdateMemberRoleEntry`) derive `Validate`. Per CLAUDE.md: "Validation uses `validate(&json)?` before any DB call." 12 of 14 JSON-accepting handlers follow this convention. Additionally, `update_member_role`'s utoipa annotation documents `(status = 422, description = "Validation error")` which can never be produced.
  - Fix: Add `validate(&json)?;` before `json.into_inner()` in both handlers. Add 422 response to `add_team_member`'s utoipa annotation.
  - Source commands: `practices-audit`, `openapi-sync`

### API — Three Create Handlers Missing `Location` Header

- [ ] **#219 — `create_team_order`, `create_order_item`, `add_team_member` return 201 without `Location` header**
  - Files: `src/handlers/teams.rs` line 246 (`create_team_order`), `src/handlers/orders.rs` line 91 (`create_order_item`), `src/handlers/teams.rs` line 376 (`add_team_member`)
  - Problem: These three handlers return `HttpResponse::Created().json(...)` without a `Location` header. The other four create handlers (`create_user`, `create_team`, `create_item`, `create_role`) all build a `Location` header via `req.url_for()`. This inconsistency violates RFC 7231 §6.3.2.
  - Fix: Add `url_for`-based `Location` headers matching the pattern in existing create handlers.
  - Note: Corrects characterization in #178 which stated "All create handlers build `Location` header" — only 4 of 7 do.
  - Source commands: `api-completeness`, `review`

### OpenAPI — `revoke_user_token` Documents 400 but Returns 500 on Invalid Token

- [ ] **#220 — utoipa annotation for `POST /auth/revoke` documents unreachable 400 response**
  - File: `src/handlers/users.rs` line 130 (utoipa annotation)
  - Problem: The annotation documents `(status = 400, description = "Invalid token")`. However, when `verify_jwt()` fails on the submitted token, it returns `Error::Jwt` which maps to HTTP 500. The documented 400 can never occur.
  - Fix: Either remove the 400 response from the utoipa annotation, or catch the JWT validation error and return 400.
  - Source commands: `openapi-sync`

### OpenAPI — `team_users` Documents Unreachable 404

- [ ] **#221 — utoipa annotation for `GET /api/v1.0/teams/{team_id}/users` documents 404 that never occurs**
  - File: `src/handlers/teams.rs` line 163 (utoipa annotation)
  - Problem: The handler always returns `HttpResponse::Ok().json(users)`, even for an empty array. CLAUDE.md states `get_team_users` returns an empty `[]` (200 OK). The 404 annotation is misleading.
  - Fix: Remove the `(status = 404, ...)` line from the utoipa annotation.
  - Source commands: `openapi-sync`

### Code Quality — Missing `#[must_use]` on `requesting_user_id`

- [ ] **#222 — `requesting_user_id` returns `Option<Uuid>` but lacks `#[must_use]`**
  - File: `src/handlers/mod.rs` line 23
  - Problem: If a caller writes `requesting_user_id(&req);` without binding the result, the JWT subject is silently discarded. All current call sites are correct, but there's no compiler guard.
  - Fix: Add `#[must_use = "caller must handle the case where no JWT claims are present"]`.
  - Source commands: `review`

### Performance — Auth Validator Redundant DashMap Lookup for TTL Eviction

- [ ] **#223 — Double DashMap lookup in `basic_validator` TTL-eviction path**
  - File: `src/middleware/auth.rs` lines 341–347
  - Problem: After the initial `cache.get().filter(TTL check)` returns `None`, the code does a second `cache.get()` to check TTL-expired, drops the guard, then calls `cache.remove()` — two extra lookups and a TOCTOU gap.
  - Fix: Use `cache.remove_if(key, |_, cached| expired(cached))` or restructure to capture the TTL decision in the initial lookup.
  - Source commands: `review`

### Validation — 4 Models Derive `Validate` with Zero Validation Rules

- [ ] **#224 — `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, `UpdateMemberRoleEntry` have no `#[validate]` attributes**
  - File: `src/models.rs` lines 311–338
  - Problem: Handlers call `validate(&json)?` (or should — see #218) on these structs, but validation always succeeds because no rules are defined. This gives a false sense of input validation.
  - Fix: Either add meaningful validation rules or remove the `Validate` derive and corresponding `validate()` calls.
  - Source commands: `review`, `practices-audit`

### Frontend — Login Shows "Invalid Credentials" for All Non-2xx Errors

- [ ] **#225 — HTTP 500, 429, and 503 responses all display "Invalid username or password"**
  - File: `frontend/src/app.rs` lines 370–373
  - Problem: The login flow's `Ok(_)` catch-all always shows a credentials error. 500 (server error), 429 (rate limited), or 503 (unavailable) should show appropriate messages instead of misleading the user about their credentials.
  - Fix: Match on `response.status()` and provide differentiated messages: 401 → credentials, 429 → rate-limited, 5xx → server error.
  - Source commands: `api-completeness`, `review`

### Dependencies — `rust_decimal` Redundant `tokio-postgres` Feature

- [ ] **#226 — `features = ["db-tokio-postgres", "serde-with-str", "tokio-postgres"]` — the third feature is unnecessary**
  - File: `Cargo.toml` (rust_decimal dependency)
  - Problem: `db-tokio-postgres` already provides `FromSql`/`ToSql` implementations. The separate `tokio-postgres` feature only adds `tokio-postgres` as a dependency of `rust_decimal`, which is redundant since the project already depends on `tokio-postgres` directly.
  - Fix: Remove `"tokio-postgres"` from the feature list → `features = ["db-tokio-postgres", "serde-with-str"]`.
  - Source commands: `dependency-check`

### Dependencies — Frontend `gloo-net` Compiles Unused WebSocket/EventSource Support

- [ ] **#227 — `gloo-net` default features not disabled — pulls unused `websocket` and `eventsource`**
  - File: `frontend/Cargo.toml` (gloo-net dependency)
  - Problem: `gloo-net = { version = "0.6", features = ["http"] }` without `default-features = false` compiles `websocket`, `eventsource`, `futures-channel`, `futures-core`, `futures-sink`, `pin-project`, and extra `web-sys` bindings — increasing WASM binary size.
  - Fix: Change to `gloo-net = { version = "0.6", default-features = false, features = ["http", "json"] }`.
  - Source commands: `dependency-check`

### Dependencies — Frontend `js-sys` Duplicated in Dependencies and Dev-Dependencies

- [ ] **#228 — `js-sys = "0.3"` appears in both `[dependencies]` and `[dev-dependencies]`**
  - File: `frontend/Cargo.toml`
  - Problem: Since it's already a runtime dependency (used in `app.rs` for `js_sys::Date::now()`), the `[dev-dependencies]` entry is redundant.
  - Fix: Remove `js-sys = "0.3"` from `[dev-dependencies]`.
  - Source commands: `dependency-check`

### Database — `memberof.joined` Column Lacks NOT NULL Constraint

- [ ] **#229 — V4 hardening added NOT NULL to `created`/`changed` but missed `joined`**
  - Files: `migrations/V1__initial_schema.sql` line 64, `migrations/V4__schema_hardening.sql`
  - Problem: `joined` has `DEFAULT CURRENT_TIMESTAMP` like other timestamp columns but was not hardened with NOT NULL in V4. A direct INSERT with explicit NULL would succeed.
  - Fix: Add a V5 migration: `UPDATE memberof SET joined = CURRENT_TIMESTAMP WHERE joined IS NULL; ALTER TABLE memberof ALTER COLUMN joined SET NOT NULL;`.
  - Source commands: `db-review`

## Informational Items

### Documentation — Test Count Maintenance Burden

- [ ] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md`
  - Problem: Hard-coded test counts go stale every time tests are added or removed.
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

### API — `memberof.joined` and `memberof.changed` Timestamps Not Exposed

- [ ] **#115 — `joined` and `changed` columns stored in DB but not returned by API**
  - Files: `src/models.rs` (`UsersInTeam`, `UserInTeams`), `src/db/teams.rs`
  - Problem: `memberof.joined` and `memberof.changed` timestamps are stored but neither model struct includes them, and DB queries don't select them.
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
  - File: `src/middleware/auth.rs` lines 335–340
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

- [ ] **#178 — Only 4 of 7 create handlers build `Location` header via `url_for` but no test verifies it**
  - Files: `tests/api_tests.rs`, `src/handlers/` (create handlers)
  - Problem: If the named route string drifts, `url_for` silently fails (wrapped in `if let Ok`). Additionally, 3 create handlers lack the `Location` header entirely (see #219).
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

### OpenAPI — `UpdateUserEntry` Has Dead `ToSchema` Derive

- [ ] **#203 — `UpdateUserEntry` derives `ToSchema` but is not registered in OpenAPI schemas and is not used by any handler**
  - File: `src/models.rs` lines 93–131
  - Problem: Superseded by `UpdateUserRequest` but still compiled with `ToSchema` derive.
  - Source commands: `openapi-sync`
  - Action: Remove `ToSchema` derive, or remove `UpdateUserEntry` entirely if not needed for internal use.

### Testing — Bulk Delete Team Orders Has No API Test

- [ ] **#204 — `DELETE /api/v1.0/teams/{id}/orders` RBAC and response untested at API level**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs` (`delete_team_orders`)
  - Problem: DB test exists but no API test verifies RBAC enforcement (require_team_admin) or HTTP response.
  - Source commands: `test-gaps`
  - Action: Add `bulk_delete_team_orders_as_team_admin`, `bulk_delete_team_orders_as_member_returns_403`.

### Testing — Update Member Role Has No API Test

- [ ] **#205 — `PUT /api/v1.0/teams/{id}/users/{id}` untested at API level**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs` (`update_member_role`)
  - Problem: DB test exists but no API test verifies endpoint, RBAC, or response shape.
  - Source commands: `test-gaps`
  - Action: Add `update_member_role_as_team_admin_returns_200`, `update_member_role_as_member_returns_403`.

### Testing — Delete User by Email Success Path Untested

- [ ] **#206 — `DELETE /api/v1.0/users/email/{email}` success path has no API test**
  - Files: `tests/api_tests.rs`, `src/handlers/users.rs` (`delete_user_by_email`)
  - Problem: Only edge cases tested. The successful delete round-trip is not tested.
  - Source commands: `test-gaps`
  - Action: Add `admin_delete_user_by_email_returns_200`.

### Testing — Token Revocation Ownership Check Untested

- [ ] **#207 — No test verifies that User A cannot revoke User B's token**
  - Files: `tests/api_tests.rs`, `src/handlers/users.rs` (`revoke_user_token`)
  - Problem: Only self-revocation happy path tested. Cross-user revocation rejection untested.
  - Source commands: `test-gaps`
  - Action: Add `revoke_other_users_token_returns_403`, `admin_can_revoke_other_users_token`.

### Testing — Team Users Has No API Test

- [ ] **#208 — `GET /api/v1.0/teams/{id}/users` has no API-level integration test**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs` (`team_users`)
  - Problem: DB test exists but no API test verifies JWT requirement, JSON shape, or empty-team behavior.
  - Source commands: `test-gaps`
  - Action: Add `get_team_users_returns_members`, `get_team_users_requires_jwt`.

### Code Quality — Redundant `Client` Import in Handler Files

- [ ] **#209 — `use deadpool_postgres::Client;` redundant in `handlers/users.rs` and `handlers/roles.rs`**
  - Files: `src/handlers/users.rs` line 17, `src/handlers/roles.rs` line 11
  - Problem: `Client` is already re-exported via `use crate::handlers::*` from `handlers/mod.rs`.
  - Source commands: `review`
  - Action: Remove the duplicate import.

### Frontend — Inconsistent `spawn_local` Import

- [ ] **#210 — Session restore uses `wasm_bindgen_futures::spawn_local` while logout uses `leptos::task::spawn_local`**
  - File: `frontend/src/app.rs`
  - Problem: Both work but inconsistent API usage.
  - Source commands: `review`
  - Action: Standardize on `leptos::task::spawn_local` throughout.

### Frontend — Form Has Redundant Double Validation

- [ ] **#211 — `<form>` has both native HTML5 validation (`required`) and custom JavaScript validation**
  - File: `frontend/src/app.rs`
  - Problem: Users may see both native browser popups and custom error messages.
  - Source commands: `review`
  - Action: Add `novalidate` attribute and rely on custom validation, or remove the custom empty-field checks.

### Performance — `get_team_users` Query Has Unnecessary `teams` JOIN

- [ ] **#230 — Query joins `teams` table but no columns from `teams` are selected**
  - File: `src/db/teams.rs` lines 138–139
  - Problem: The query `join teams on teams.team_id = memberof.memberof_team_id` and `where teams.team_id = $1` could be simplified to `where memberof.memberof_team_id = $1` without the join. The `teams` join adds no value since no `teams` columns are in the SELECT.
  - Fix: Remove the `teams` join and filter directly on `memberof.memberof_team_id = $1`.
  - Source commands: `review`

### Frontend — Loading Page Spinner Not Announced to Screen Readers

- [ ] **#231 — Loading spinner container lacks `role="status"` and `aria-live`**
  - File: `frontend/src/app.rs` (LoadingPage component)
  - Problem: The loading page has `<div class="loading-spinner">` and `<p class="loading-text">"Loading…"</p>` but the container has no `role="status"` or `aria-live="polite"`. Screen readers won't announce the loading state.
  - Fix: Add `role="status"` and `aria-live="polite"` to the loading card container div.
  - Source commands: `review`

### Code Quality — `ErrorResponse::Display` Fallback Doesn't Escape JSON

- [ ] **#232 — If `serde_json::to_string` fails, the fallback `format!` produces invalid JSON for strings with quotes**
  - File: `src/errors.rs` lines 55–62
  - Problem: The `Display` impl fallback uses `format!(r#"{{"error":"{}"}}"#, self.error)` — if `self.error` contains `"` or `\`, the resulting JSON is syntactically invalid. The primary path (serde_json) correctly escapes, but the fallback doesn't.
  - Fix: Remove the fallback since `ErrorResponse` serialization should never fail, or properly escape the string.
  - Source commands: `review`

### Frontend — Redundant `session_storage()` Calls in Logout Handler

- [ ] **#233 — `session_storage()` called 3 times in the `on_logout` closure**
  - File: `frontend/src/app.rs` (on_logout closure)
  - Problem: Each call goes through `web_sys::window() → .session_storage()`. Should bind once and reuse.
  - Fix: Bind `let storage = session_storage();` once and reuse the result.
  - Source commands: `review`

### Code Quality — `from_row.rs` Error Classification Uses Fragile String Matching

- [ ] **#234 — `map_err` helper checks for `"column"` or `"not found"` in error messages**
  - File: `src/from_row.rs` lines 37–43
  - Problem: The `map_err` function classifies `tokio_postgres::Error` as `ColumnNotFound` vs `Conversion` by checking whether the error message contains `"column"` or `"not found"`. If `tokio_postgres` changes error wording, classification could silently flip.
  - Fix: No immediate action. Document the fragility with a comment. Revisit if `tokio_postgres` adds structured error accessors.
  - Source commands: `review`

### Database — `closed` Column Read as `Option<bool>` Despite `NOT NULL` Constraint

- [ ] **#235 — `is_team_order_closed` and `guard_open_order` use `Option<bool>` for a NOT NULL column**
  - File: `src/db/order_items.rs` lines 31 and 55
  - Problem: Both functions read the `closed` column with `row.get::<_, Option<bool>>("closed").unwrap_or(false)`. The column is `boolean NOT NULL DEFAULT FALSE`, so the value can never be NULL. The `Option<bool>` and `.unwrap_or(false)` are unnecessary.
  - Fix: Change to `row.get::<_, bool>("closed")`.
  - Source commands: `db-review`

### Testing — Non-Member GET Rejection Untested for Order Endpoints

- [ ] **#236 — All order-related GET handlers call `require_team_member` but no test verifies GET rejection for non-members**
  - Files: `tests/api_tests.rs`, `src/handlers/orders.rs`, `src/handlers/teams.rs`
  - Problem: The `non_member_cannot_create_order_item` test verifies POST/PUT/DELETE rejection (403). But GET endpoints (`get_team_orders`, `get_team_order`, `get_order_items`, `get_order_item`) that also call `require_team_member` have no non-member rejection test.
  - Source commands: `test-gaps`
  - Action: Add `non_member_cannot_get_team_orders` and `non_member_cannot_get_order_items`.

### Testing — No API Test for GET Single Team Order by ID

- [ ] **#237 — `GET /api/v1.0/teams/{team_id}/orders/{order_id}` never called in tests**
  - Files: `tests/api_tests.rs`, `src/handlers/teams.rs` (`get_team_order`)
  - Problem: `create_and_list_team_orders` creates an order and lists all, but never calls the single-order GET. This endpoint has a two-column `WHERE` clause — if parameterization were swapped, no test would catch it.
  - Source commands: `test-gaps`
  - Action: Add GET-by-ID assertion to existing test or create `get_single_team_order_returns_200`.

### Testing — `add_team_member` with FK-Violating IDs Untested

- [ ] **#238 — Adding a member with non-existent `user_id` or `role_id` → error quality untested**
  - Files: `tests/api_tests.rs`, `tests/db_tests.rs`
  - Problem: No test verifies the HTTP status or error message quality when FK constraints are violated. The error might bubble as raw SQL.
  - Source commands: `test-gaps`
  - Action: Add `add_member_with_nonexistent_user_returns_error`, `add_member_with_nonexistent_role_returns_error`.

### Testing — No Frontend Test for Non-401/Non-Network HTTP Errors

- [ ] **#239 — No WASM test mocks 500 or 429 responses for the login flow**
  - File: `frontend/tests/ui_tests.rs`
  - Problem: Only 200 (success) and 401 (credentials) responses are mocked. HTTP 500 or 429 currently show "Invalid username or password" (see #225). Once fixed, tests should verify corrected behavior.
  - Source commands: `test-gaps`
  - Action: Add `test_login_with_500_response_shows_server_error`.

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 170 backend unit tests pass (148 lib + 22 healthcheck); 67 API integration tests pass; 86 DB integration tests pass; 23 WASM tests pass. Total: 346 tests, 0 failures.
- Backend unit test breakdown: config: 7, errors: 15, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 12, middleware/openapi: 14, from_row: 10, db/migrate: 34, healthcheck: 22 = **170 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` 0.9.10 via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. Re-evaluate periodically whether the `rsa` crate or `jsonwebtoken` has shipped a fix.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` is clean on both crates.
- RBAC enforcement is correct across all handlers per the policy table.
- OpenAPI spec is synchronized with routes (41 operations), with 2 minor annotation inaccuracies (#220, #221).
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 1 critical (#132 blocked), 4 important (#212, #213, #214, #215), 32 minor, 39 informational. Total: 76 open items.
- 121 resolved items in `.claude/resolved-findings.md`.
