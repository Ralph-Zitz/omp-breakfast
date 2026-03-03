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

*All important items have been resolved. See `.claude/resolved-findings.md` for details.*

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

### Documentation — Frontend Test Category Breakdown Sums to 21, Not 23

- [ ] **#163 — CLAUDE.md test category breakdown omits 2 token refresh tests**
  - File: `CLAUDE.md` (Testing → Frontend → Test categories)
  - Problem: 8 categories total 4+3+3+3+2+1+2+3 = 21, but 23 WASM tests exist. Missing: `test_authed_get_retries_after_401_with_token_refresh` and `test_authed_get_double_failure_falls_back_to_login`.
  - Fix: Add "Token refresh (2 tests)" category to the breakdown.
  - Source commands: `cross-ref-check`

### Frontend — Login Shows "Invalid Credentials" for All Non-2xx Errors

- [ ] **#225 — HTTP 500, 429, and 503 responses all display "Invalid username or password"**
  - File: `frontend/src/app.rs` lines 370–373
  - Problem: The login flow's `Ok(_)` catch-all always shows a credentials error. 500 (server error), 429 (rate limited), or 503 (unavailable) should show appropriate messages instead of misleading the user about their credentials.
  - Fix: Match on `response.status()` and provide differentiated messages: 401 → credentials, 429 → rate-limited, 5xx → server error.
  - Source commands: `api-completeness`, `review`

### Dependencies — Frontend `js-sys` Duplicated in Dependencies and Dev-Dependencies

*All dependency findings in this category have been resolved. See `.claude/resolved-findings.md` for details.*

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

### Frontend — Consumes Only 4 of 41 Endpoints

- [ ] **#116 — Frontend only uses auth (3) + user-detail (1) endpoints**
  - File: `frontend/src/app.rs`
  - Problem: 37 backend endpoints are fully implemented but await frontend page development.
  - Source commands: `api-completeness`
  - Action: Documented in CLAUDE.md Frontend Roadmap. Will be consumed as pages are built.

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

### Auth — `revoke_user_token` Returns 403 for Missing Authentication

- [ ] **#243 — `revoke_user_token` uses `Error::Forbidden("Authentication required")` — should be `Error::Unauthorized`**
  - File: `src/handlers/users.rs` line 145
  - Problem: When `requesting_user_id()` returns `None` (no JWT claims in request), the handler returns 403 Forbidden. Missing authentication should produce 401 Unauthorized per HTTP semantics.
  - Fix: Change `Error::Forbidden("Authentication required".to_string())` to `Error::Unauthorized("Authentication required".to_string())`.
  - Source commands: `practices-audit`

### OpenAPI — `get_health` Missing 503 Response Annotation

- [ ] **#244 — `get_health` utoipa annotation only documents 200; handler also returns 503**
  - File: `src/handlers/mod.rs` lines 304–311
  - Problem: The handler returns `HttpResponse::ServiceUnavailable()` when the DB is unreachable, but the OpenAPI spec shows only `(status = 200)`.
  - Fix: Add `(status = 503, description = "Service unavailable — database unreachable", body = StatusResponse)`.
  - Source commands: `openapi-sync`

### OpenAPI — `create_user` Annotates Unreachable 404

- [ ] **#245 — `create_user` utoipa includes `(status = 404, description = "User not created")` but handler never returns 404**
  - File: `src/handlers/users.rs` line 178
  - Problem: `db::create_user` uses INSERT + RETURNING which returns 500 or 409 (unique violation) on failure — never 404. The annotation misleads API consumers.
  - Fix: Remove the `(status = 404, ...)` line from utoipa responses. Add `(status = 409, description = "Email already exists", body = ErrorResponse)` if not already present.
  - Source commands: `openapi-sync`

### Documentation — CLAUDE.md Test Count Stale

- [ ] **#246 — CLAUDE.md says "149 unit tests" — actual count is 171 (149 lib + 22 healthcheck)**
  - File: `CLAUDE.md` line 284
  - Problem: Count doesn't include healthcheck binary tests. The assessment notes already track the correct 171 count.
  - Fix: Update line to "171 unit tests" or separate "149 library + 22 healthcheck binary" to match the notes.
  - Source commands: `cross-ref-check`

### Security — Token Responses Lack `Cache-Control: no-store`

- [ ] **#247 — `/auth` and `/auth/refresh` responses contain JWT tokens but no `Cache-Control` header**
  - Files: `src/server.rs` lines 421–425 (DefaultHeaders), `src/handlers/users.rs` (auth_handler, refresh_handler)
  - Problem: RFC 6749 §5.1 requires token responses to include `Cache-Control: no-store`. Tokens could theoretically be cached by intermediaries or browser disk cache.
  - Fix: Add `Cache-Control: no-store` to `DefaultHeaders` globally (safe — API serves no cacheable public content) or per-response on auth endpoints.
  - Source commands: `security-audit`

### Security — Missing `Referrer-Policy` Header

- [ ] **#248 — `DefaultHeaders` does not include `Referrer-Policy`**
  - File: `src/server.rs` lines 421–425
  - Problem: Without this header, the browser may send full URL (including path and query) in the `Referer` header on navigations, potentially leaking sensitive path segments.
  - Fix: Add `.add(("Referrer-Policy", "strict-origin-when-cross-origin"))` to `DefaultHeaders`.
  - Source commands: `security-audit`

### Deployment — Docker Compose Exposes PostgreSQL on All Interfaces

- [ ] **#249 — `docker-compose.yml` maps port 5432 to `0.0.0.0` by default**
  - File: `docker-compose.yml`
  - Problem: Default `ports: "5432:5432"` binds to all interfaces, exposing the database to the network.
  - Fix: Change to `"127.0.0.1:5432:5432"`.
  - Source commands: `security-audit`

### Documentation — Command Files Reference Stale Migration Range (V1–V3)

- [ ] **#250 — `api-completeness.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/api-completeness.md`
  - Problem: Migration range is stale — V4 and V5 exist but are not mentioned in the scope section.
  - Fix: Update scope to reference V1–V5.
  - Source commands: `cross-ref-check`

- [ ] **#251 — `db-review.md` scope only references V1–V3 migrations**
  - File: `.claude/commands/db-review.md`
  - Problem: Same stale-range issue as #250.
  - Fix: Update scope to reference V1–V5.
  - Source commands: `cross-ref-check`

### Documentation — `database.sql` Stale vs V3–V5

- [ ] **#252 — `database.sql` deprecated script doesn't reflect V3–V5 changes**
  - File: `database.sql`
  - Problem: CLAUDE.md says this file is "deprecated — kept for manual dev resets only", but it doesn't include indexes, constraints from V3, schema hardening from V4, or NOT NULL fixes from V5.
  - Fix: Either regenerate from current schema or add a prominent comment noting it reflects only V1–V2.
  - Source commands: `cross-ref-check`

### Validation — `Validate` Derive Still on 4 No-Rule Structs

- [ ] **#253 — #224 marked resolved but `Validate` derive is still present on `CreateTeamOrderEntry`, `UpdateTeamOrderEntry`, `AddMemberEntry`, `UpdateMemberRoleEntry`**
  - File: `src/models.rs` lines 335–357
  - Problem: The resolved-findings.md entry for #224 claims "Removed `Validate` derive from all 4 structs" but the code still has them. The `validate()` calls WERE removed from handlers, but the derive macro remains — generating dead code. Harmless but misleading vs the resolved record.
  - Fix: Remove `Validate` from the derive macros, or re-open #224 in resolved-findings.md. Either way, the derive is unused.
  - Source commands: `practices-audit`, `review`

### Code Quality — `from_row_ref` Boilerplate Reducible by Macro

- [ ] **#254 — 9 `FromRow` implementations total ~200 lines of repetitive `try_get`/`map_err` per column**
  - File: `src/from_row.rs` lines 52–230
  - Problem: Every `from_row_ref` body repeats the same `row.try_get("col").map_err(|e| map_err("col", e))?` pattern per column.
  - Source commands: `review`
  - Action: Introduce a `macro_rules! impl_from_row` to generate the bodies when the mapping code is next modified.

### Code Quality — Duplicated Row-Mapping Pattern Across 6 DB List Functions

- [ ] **#255 — Identical `filter_map` + `warn` block in `get_users`, `get_teams`, `get_roles`, `get_items`, `get_team_orders`, `get_order_items`**
  - Files: `src/db/users.rs`, `src/db/teams.rs`, `src/db/roles.rs`, `src/db/items.rs`, `src/db/orders.rs`, `src/db/order_items.rs`
  - Problem: All 6 list functions duplicate `rows.iter().filter_map(|row| match T::from_row_ref(row) { Ok(v) => Some(v), Err(e) => { warn!(...); None } }).collect()`.
  - Source commands: `review`
  - Action: Extract to a generic `fn map_rows<T: FromRow>(rows: &[Row]) -> Vec<T>` helper in `src/db/mod.rs`.

### Deployment — `HTTP_REDIRECT_PORT` Hardcoded to 80

- [ ] **#256 — HTTP→HTTPS redirect listener binds to port 80 unconditionally (related to #185)**
  - File: `src/server.rs` line 27
  - Problem: In containers or dev environments, port 80 may be unavailable. Similar to #185 (healthcheck port hardcoded to 8080).
  - Source commands: `review`
  - Action: Add `server.redirect_port` to config YAML or read from env var, falling back to 80.

### Dependencies — `password-hash` Direct Dependency for Feature Activation Only

- [ ] **#257 — `password-hash` is a direct dependency only to activate the `getrandom` feature**
  - File: `Cargo.toml`
  - Problem: The crate is transitively pulled by `argon2`. The direct dependency exists solely to enable `features = ["getrandom"]`.
  - Source commands: `dependency-check`
  - Action: Informational. No action needed unless `argon2` adds the feature gate.

### Security — Missing `Permissions-Policy` Header

- [ ] **#258 — `DefaultHeaders` does not include `Permissions-Policy`**
  - File: `src/server.rs` lines 421–425
  - Problem: Nice-to-have header that disables unused browser features (camera, microphone, geolocation, etc.).
  - Source commands: `security-audit`
  - Action: Add `.add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))` when security headers are next reviewed.

### Deployment — Docker Compose `breakfast` Service Lacks Resource Limits

- [ ] **#259 — No `deploy.resources.limits` for CPU or memory**
  - File: `docker-compose.yml`
  - Problem: In production, an unbounded container could consume all host resources.
  - Source commands: `security-audit`
  - Action: Add resource limits when deploying to production.

### Documentation — `database_seed.sql` Header Only Mentions V1

- [ ] **#260 — Seed data file header references only V1 schema**
  - File: `database_seed.sql`
  - Problem: Minor doc staleness — schema has evolved through V5.
  - Source commands: `cross-ref-check`
  - Action: Update header comment to reference current schema version.

### Testing — No Test for Partial `update_team_order` (COALESCE Preservation)

- [ ] **#261 — No test passes `None` for some update fields and verifies existing values are preserved**
  - File: `tests/db_tests.rs` (`update_team_order_changes_fields`)
  - Problem: Existing test sets all 3 fields simultaneously. A regression that breaks COALESCE partial updates would go undetected.
  - Source commands: `test-gaps`
  - Action: Add `update_team_order_partial_preserves_existing`.

### Testing — No Test for `create_team_order` with FK-Violating `team_id`

- [ ] **#262 — No test creates a team order with non-existent `team_id` to verify FK error handling**
  - Files: `tests/db_tests.rs`, `tests/api_tests.rs`
  - Problem: Related to #238 (FK violation for `add_team_member`) but for a different entity.
  - Source commands: `test-gaps`
  - Action: Add `create_team_order_nonexistent_team_returns_conflict`.

### Testing — No Explicit Refresh Token Revocation → Refresh Rejection Test

- [ ] **#263 — No test explicitly revokes a refresh token via `/auth/revoke` then verifies `/auth/refresh` returns 401**
  - File: `tests/api_tests.rs`
  - Problem: Rotation-based implicit revocation is tested, but explicit revoke→refresh is not.
  - Source commands: `test-gaps`
  - Action: Add `revoke_refresh_token_prevents_refresh`.

### Testing — No Test for Empty Order Items List Response

- [ ] **#264 — No test verifies `GET .../items` returns `200 []` for an order with zero items**
  - File: `tests/api_tests.rs`
  - Problem: Empty-collection 200 behavior is tested for users and teams but not order items.
  - Source commands: `test-gaps`
  - Action: Add `get_order_items_empty_returns_200`.

### Testing — `guard_admin_role_assignment` Non-Existent `role_id` Path Untested

- [ ] **#265 — No test calls `add_team_member` or `update_member_role` with a non-existent `role_id`**
  - File: `src/handlers/mod.rs` lines 146–162
  - Problem: For non-admin callers, the guard calls `db::get_role(client, role_id)` which returns 404 if the role doesn't exist. This error propagation path is never exercised.
  - Source commands: `test-gaps`
  - Action: Add API test with non-existent `role_id` to verify error response.

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- All 184 backend unit tests pass (162 lib + 22 healthcheck); 90 API integration tests pass; 90 DB integration tests pass; 23 WASM tests pass. Total: 387 tests, 0 failures.
- Backend unit test breakdown: config: 7, errors: 16, handlers/mod: 11, validate: 9, routes: 19, server: 17, middleware/auth: 13, middleware/openapi: 14, from_row: 10, db/migrate: 34, models: 12, healthcheck: 22 = **184 total**.
- `cargo audit --ignore RUSTSEC-2023-0071` reports 0 vulnerabilities. RUSTSEC-2023-0071 (`rsa` 0.9.10 via `jsonwebtoken`) is intentionally ignored — **blocked on upstream**, see #132. Re-evaluate periodically whether the `rsa` crate or `jsonwebtoken` has shipped a fix.
- All dependencies are up to date (`cargo outdated -R` shows zero outdated).
- Clippy is clean on both backend and frontend.
- `cargo fmt --check` is clean on both crates.
- RBAC enforcement is correct across all handlers per the policy table.
- OpenAPI spec is synchronized with routes (41 operations), with 3 annotation inaccuracies (#244, #245, and existing #220/#221).
- All 11 assessment commands run: `api-completeness`, `cross-ref-check`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment` (loader only).
- Open items summary: 1 critical (#132 blocked), 3 important (#240–#242), 19 minor, 32 informational. Total: 55 open items.
- 169 resolved items in `.claude/resolved-findings.md`.
