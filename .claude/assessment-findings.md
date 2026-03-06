# Assessment Findings

Last assessed: 2026-03-08

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

_No open important items._

## Minor Items

_No open minor items._

## Informational Items

### Frontend — Uses `String` for UUIDs Everywhere

- [ ] **#321 — No type safety for UUID fields in frontend API types**
  - File: `frontend/src/api.rs`
  - Problem: All ID fields are `String`. A typo or wrong field could silently produce invalid requests.
  - Source commands: `review`

### Database — `orders.orders_team_id` May Be Missing NOT NULL

- [ ] **#325 — Advisory: verify that `orders_team_id` FK column has NOT NULL**
  - Files: `migrations/V1__initial_schema.sql`, `src/models.rs`
  - Source commands: `db-review`

### OpenAPI — Order-Item Endpoint 403 Descriptions Are Imprecise

- [ ] **#326 — `create_order_item`, `update_order_item`, `delete_order_item` utoipa 403 descriptions do not match actual RBAC guards**
  - File: `src/handlers/orders.rs`
  - Fix: Update 403 descriptions to match actual RBAC policy once #302/#303 are fixed.
  - Source commands: `openapi-sync`

### Security — Account Lockout State In-Memory Only

- [ ] **#339 — Login attempt tracking stored in `DashMap`, not shared across instances**
  - File: `src/middleware/auth.rs` (lines ~189–213)
  - Problem: In multi-instance deployment, attacker can distribute brute-force attempts across instances.
  - Source commands: `security-audit`

### Testing — `basic_validator` Malformed Password Hash Path Untested

- [ ] **#350 — When DB stores a corrupted/non-Argon2 hash, `PasswordHash::new()` fails and returns 500 — no test**
  - File: `src/middleware/auth.rs` (lines ~484–498)
  - Source commands: `test-gaps`

### API Completeness — Frontend `ItemEntry.price` Typed as `String`

- [ ] **#366 — Frontend `ItemEntry` uses `pub price: String` instead of a numeric type**
  - File: `frontend/src/api.rs`
  - Problem: Backend returns `numeric(10,2)` as a JSON number; frontend deserializes as `String` which works but loses type safety for display and arithmetic.
  - Source commands: `api-completeness`

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

### Testing — `auth_user` Cache Miss Path Untested

- [ ] **#389 — No test verifies the code path when the auth cache has no entry for a user (first login or after TTL expiry)**
  - File: `src/middleware/auth.rs`
  - Source commands: `test-gaps`

### Testing — Health Endpoint 503 Response Never Tested

- [ ] **#394 — No integration test verifies that `/health` returns HTTP 503 when the database is unreachable**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Testing — `refresh_token` `DateTime::from_timestamp` Fallback Untested

- [ ] **#395 — The `DateTime::from_timestamp(exp, 0).unwrap_or_default()` fallback in `refresh_token` handler is never tested**
  - File: `src/handlers/users.rs`
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

### Code Quality — Auth Cache Eviction O(n)

- [ ] **#453 — `evict_oldest_if_full` iterates all 1000 entries to find oldest; fine at current scale**
  - File: `src/middleware/auth.rs`
  - Source commands: `review`

### Frontend — `fetch_user_details` Silently Drops Non-401 Errors

- [ ] **#510 — When `authed_get` returns a non-OK response (403, 500), `fetch_user_details` returns `None` with no logging**
  - File: `frontend/src/api.rs` (`fetch_user_details` function)
  - Problem: During session restore, a 403 or 500 silently drops the user to the login page with no explanation. Other page-level fetches at least log errors to the console.
  - Fix: Add `web_sys::console::warn_1(...)` for non-OK responses before returning `None`.
  - Source commands: `review`

### Database — `database.sql` Not Updated for V7 Migration

- [ ] **#512 — `database.sql` still creates `idx_users_email` and `idx_teams_name` indexes dropped by V7 migration**
  - File: `database.sql` (lines ~56, ~69)
  - Problem: V7 drops these indexes as redundant (duplicated by UNIQUE constraints). The deprecated dev-reset script still creates them, causing schema drift.
  - Fix: Remove the two `CREATE INDEX` statements from `database.sql`.
  - Source commands: `db-review`

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

### Database — `database.sql` Missing Avatar Support from V8

- [ ] **#517 — `database.sql` lacks avatars table and `users.avatar_id` FK column added in V8 migration**
  - File: `database.sql`
  - Problem: Running `database.sql` for a manual dev reset would lose avatar functionality. Subsequent avatar operations would fail with FK errors or missing table.
  - Fix: Add avatars table CREATE and ALTER TABLE for `users.avatar_id` to `database.sql`.
  - Source commands: `db-review`

### Database — Missing Index on `users.avatar_id` FK

- [ ] **#518 — V8 adds FK from `users` to `avatars` but does not create an index on `users.avatar_id`**
  - File: `migrations/V8__avatars.sql`
  - Problem: FK constraint verification and JOIN performance may require sequential scan on `users` table when deleting avatars or querying user-avatar joins.
  - Fix: Add `CREATE INDEX IF NOT EXISTS idx_users_avatar ON users (avatar_id);` in a future migration.
  - Source commands: `db-review`

### Database — `items.price` CHECK Constraint Allows Zero

- [ ] **#519 — `items.price CHECK (price >= 0)` permits items with zero cost; a breakfast ordering system likely doesn't intend free items**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: No business-level protection against inserting cost-free items; the `validate_non_negative_price` custom validator also accepts zero.
  - Source commands: `db-review`

### Frontend — No Client-Side Validation for Item Price Format

- [ ] **#520 — Frontend items page accepts free-form text for price without validating it's a valid decimal number**
  - File: `frontend/src/pages/items.rs`
  - Problem: Backend validates, but poor UX if user enters non-numeric text — they get a generic server error instead of inline form feedback.
  - Source commands: `api-completeness`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **Test counts verified (2026-03-06):** 234 unit (212 lib + 22 healthcheck), 145 API integration (ignored), 104 DB integration (ignored), 79 WASM.
- **`cargo audit` (2026-03-06):** Exit code 0. No new vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) remains intentionally ignored — **blocked on upstream**, see #132.
- **CONNECT Design System (2026-03-06):** `git pull` fetched new commits (3.0.0-RC1, 2.9.0, etc.). CSS changes to checkbox-group, dropdown, inline-alert, menu, radio-group, tag, text-field, utility-button — **none used by our frontend**. Token additions (opacity variants) are non-breaking. SCSS removal has no impact (we only use CSS imports). **No migration required.**
- Open items summary: 1 critical (#132 blocked), 0 important, 0 minor, 12 informational (down from 22 — 10 security-audit findings resolved).
- 8 new findings in this assessment: #513–#520. 0 regressions found. 4 items resolved this session (#513, #514, #515, #516). 26 test-gap items resolved in session of 2026-03-07. 10 security-audit items resolved in session of 2026-03-08 (#336, #337, #338, #340, #379, #380, #448, #449, #450, #451).
- Highest finding number: #520.

### Re-assessment — 2026-03-06

- **All 11 commands re-run:** 8 new findings surfaced (0 critical, 1 important, 3 minor, 4 informational).
- **#515 (Important):** README.md migration table missing V8 — says "Seven" but 8 exist on disk.
- **#513 (Minor):** `get_avatar` utoipa annotation falsely claims JWT auth, but endpoint is public.
- **#514, #516 (Minor):** Test count drift in CLAUDE.md (198→199) and README.md (193→199).
- **#517–#520 (Informational):** `database.sql` missing V8, missing FK index on `users.avatar_id`, price CHECK allows zero, frontend lacks price format validation.
- **0 regressions** — all 354+ resolved items spot-checked, none regressed.
- **Unit tests:** 199 passing (177 lib + 22 healthcheck). `cargo fmt`: clean. `cargo audit`: exit 0.
- **CONNECT Design System:** Updated. New commits pulled. No breaking changes to components used by frontend.
