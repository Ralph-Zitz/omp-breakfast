# Assessment Findings

Last assessed: 2026-03-05

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

### Code Quality — Auth Cache Eviction O(n)

- [ ] **#453 — `evict_oldest_if_full` iterates all 1000 entries to find oldest; fine at current scale**
  - File: `src/middleware/auth.rs`
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

- **Test counts verified (2026-03-05):** 193 unit (171 lib + 22 healthcheck), 117 API integration (ignored), 103 DB integration (ignored), 41 WASM — all match CLAUDE.md and README.md.
- **`cargo audit` (2026-03-05):** Exit code 0. No new vulnerabilities. RUSTSEC-2023-0071 (`rsa` via `jsonwebtoken`) remains intentionally ignored — **blocked on upstream**, see #132. Re-evaluate periodically.
- **CONNECT Design System (2026-03-05):** `git pull` reports "Already up to date" — no migration needed.
- Open items summary: 1 critical (#132 blocked), 0 important, 0 minor, 90+ informational.
- 5 new findings in this assessment: #500–#504 (all documentation, all fixed). 0 regressions found.
- Highest finding number: #504.
- 354 resolved items in `.claude/resolved-findings.md`.

### Re-assessment — 2026-03-05 (docker fix session)

- **Changes since last run:** Two Docker bug fixes applied — `config/docker-base.yml` (added `db_ca_cert: localhost_ca.pem` + `ssl_mode: Require` under `pg`) and `docker-compose.yml` (added `BREAKFAST_PG_USER=actix`, `BREAKFAST_PG_PASSWORD=actix`). Both fix startup errors when running `docker compose up`.
- **All commands re-run (api-completeness, cross-ref-check, db-review, dependency-check, openapi-sync, practices-audit, rbac-rules, review, security-audit, test-gaps):** 0 new findings.
- **0 regressions** — all 354 resolved items checked, none regressed.
- **Unit tests:** 193 passing (171 lib + 22 healthcheck). `cargo audit` exit 0.
- **CONNECT Design System:** Already up to date.
