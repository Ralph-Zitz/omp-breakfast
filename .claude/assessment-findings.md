# Assessment Findings

Last assessed: 2026-03-01 (resume-assessment session, resolved #57 and #58)

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will preserve completed items, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Important Items

No open important items. All important findings have been resolved and moved to "Completed Items" below.

## Minor Items

No open minor items. All minor findings have been resolved and moved to "Completed Items" below.

## Informational Items

### Dependencies — Unfixable RSA Advisory

- [ ] **#55 — `rsa` 0.9.10 has an unfixable timing side-channel advisory (RUSTSEC-2023-0071)**
  - Problem: The `rsa` crate (transitive dependency via `jsonwebtoken` 10.3.0) is affected by the Marvin Attack (CVSS 5.9, medium severity), a timing side-channel that could allow key recovery during RSA decryption. No patched version is available upstream. Since this project uses HMAC-based JWT signing (not RSA key exchange), the practical risk is negligible. This item will remain open until `jsonwebtoken` updates its `rsa` dependency or a patched `rsa` release is published.
  - Source command: `dependency-check` (cargo audit)
  - Action: No code changes possible — waiting on upstream fix. Monitor `jsonwebtoken` releases.

### Documentation — Test Count Maintenance Burden

- [ ] **#54 — Test counts in CLAUDE.md will drift as tests are added**
  - File: `CLAUDE.md` lines 256–258, 264
  - Problem: CLAUDE.md hard-codes specific test counts (79 unit, 65 API integration, 86 DB, 22 WASM). These go stale every time a test is added or removed. The counts were just verified and are currently accurate, but they will drift again with the next code change that adds tests.
  - Source command: `practices-audit`
  - Action: No code change needed. This is an inherent maintenance cost of documenting exact counts. The assessment process updates them each time it runs.

## Completed Items

Items moved here after being resolved:

### Documentation — CLAUDE.md CSP Policy Not Documented

- [x] **#57 — CLAUDE.md Key Conventions should document the CSP header on static files**
  - File: `CLAUDE.md` line 118 (Key Conventions section)
  - Resolution: Added a new bullet to Key Conventions documenting the full `Content-Security-Policy` header value and explaining why `'unsafe-inline'` in `script-src` is required for Trunk's inline WASM bootstrap `<script type="module">`. Notes that removing it causes a white-screen failure in Chrome.
  - Source commands: `practices-audit`, `security-audit`

### Frontend — Loading Page Spinner CSS Missing

- [x] **#58 — `LoadingPage` component references undefined CSS classes**
  - File: `frontend/style/main.css`
  - Resolution: Added CSS rules for `.loading-page` (background), `.loading-card` (flex column centered layout with padding), `.loading-spinner` (40×40px animated spinner using `var(--color-border)` and `var(--color-primary)` with the existing `spin` keyframes), and `.loading-text` (muted text styling). All 22 WASM frontend tests pass.
  - Source commands: `review`, `practices-audit`

### Security — actix-files CVE (Verified Patched)

- [x] **#56 — `actix-files` had 2 known CVEs (GHSA-8v2v-wjwg-vx6r, GHSA-gcqf-3g44-vc9p)**
  - Problem: `actix-files` versions prior to 0.6.10 are vulnerable to information exposure when serving a non-existing folder (GHSA-8v2v-wjwg-vx6r, medium severity) and panic on empty `Range` header (GHSA-gcqf-3g44-vc9p, medium severity).
  - Resolution: Verified that `Cargo.lock` already pins `actix-files` at version 0.6.10 (the fixed version). The `Cargo.toml` spec `"0.6"` (equivalent to `>=0.6.0, <0.7.0`) resolved to the patched version via `cargo update` in a prior session. No code changes required.
  - Source commands: `dependency-check`, `security-audit`

### Security — Password Hashing at User Creation

- [x] **#40 — `create_user` stores plaintext password instead of Argon2 hash**
  - File: `src/db.rs` lines 68–93
  - Resolution: Already fixed in a prior session — `db::create_user` now hashes the password with `Argon2::default()` + `SaltString::generate(&mut OsRng)` before inserting, matching the pattern in `db::update_user`. Verified by the new `create_user_then_authenticate_round_trip` integration test (#44).
  - Source commands: `db-review`, `security-audit`

### Documentation — CLAUDE.md Stale After Recent Changes

- [x] **#41 — Test counts in CLAUDE.md are stale**
  - File: `CLAUDE.md`
  - Resolution: Updated all test count references — 79 unit tests, 65 API integration tests, 22 WASM tests. Counts now match actual test output.
  - Source command: `practices-audit`

- [x] **#42 — `Error::Unauthorized` variant not documented in CLAUDE.md**
  - File: `CLAUDE.md` Key Conventions section
  - Resolution: Added bullet: "`Error::Unauthorized` variant maps to HTTP 401 for authentication failures" after the existing `Error::Forbidden` bullet.
  - Source command: `practices-audit`

- [x] **#43 — Unfinished Work section does not reflect frontend token revocation**
  - File: `CLAUDE.md` Unfinished Work + Frontend Architecture sections
  - Resolution: Updated Unfinished Work to mention "token revocation" alongside "token refresh". Updated Auth flow description in Frontend Architecture to note: "On logout, both access and refresh tokens are revoked server-side via `POST /auth/revoke` (fire-and-forget)."
  - Source commands: `practices-audit`, `api-completeness`

### Test Gaps

- [x] **#44 — No integration test for create-user → authenticate round-trip**
  - File: `tests/api_tests.rs` — `create_user_then_authenticate_round_trip`
  - Resolution: Added integration test that: (1) creates a user via `POST /api/v1.0/users`, (2) authenticates via `POST /auth` with Basic Auth, (3) asserts 200 with valid token pair, (4) uses the token to fetch the user's own profile, (5) cleans up. All 65 API integration tests pass.
  - Source command: `test-gaps`

### Backend — Error Response Consistency

- [x] **#15 — `auth_user` returns bare string instead of `ErrorResponse`**
  - File: `src/handlers/users.rs`
  - Resolution: Replaced `Ok(HttpResponse::Unauthorized().json("Unauthorized"))` with `Err(Error::Unauthorized("Unauthorized".to_string()))`, routing through the centralized `ResponseError` impl with proper `warn!()` logging.
  - Source command: `review`

- [x] **#16 — `refresh_token` handler bypasses centralized error handling**
  - File: `src/handlers/users.rs`
  - Resolution: Added `Error::Unauthorized(String)` variant to `errors::Error` enum (maps to HTTP 401 with `warn!()` logging). Replaced both `Ok(HttpResponse::Unauthorized().json(ErrorResponse { ... }))` returns (invalid token type and revoked token cases) with `Err(Error::Unauthorized(...))`. Added unit test `unauthorized_error_returns_401`.
  - Source command: `review`

### Frontend — Token Revocation on Logout

- [x] **#1 — Frontend logout does not revoke tokens server-side**
  - File: `frontend/src/app.rs` — `on_logout` closure in `DashboardPage`
  - Resolution: Added `revoke_token_server_side` async helper that POSTs to `/auth/revoke` with the bearer token and token-to-revoke body. Updated `on_logout` to grab both tokens before clearing `sessionStorage`, then fire-and-forget revocation of both access and refresh tokens via `leptos::task::spawn_local`. Storage is cleared immediately so logout always succeeds regardless of network outcome.
  - Source commands: `api-completeness`, `security-audit`

### Database — Inconsistent Row Mapping Pattern

- [x] **#6 — `get_team_users` uses `.map()` instead of `filter_map` + `warn!()`**
  - File: `src/db.rs` — `get_team_users` function
  - Resolution: Changed `.map(|row| UsersInTeam { ... })` with infallible `row.get()` to `.filter_map()` with `row.try_get()` tuple matching. Failed rows now log `warn!("Failed to map users-in-team row — skipping")` and are skipped, consistent with all other list queries.
  - Source commands: `db-review`, `practices-audit`

- [x] **#7 — `get_user_teams` has the same `.map()` issue**
  - File: `src/db.rs` — `get_user_teams` function
  - Resolution: Same approach as #6 — changed to `filter_map` with `try_get()` and `warn!()` on failure, matching the project convention.
  - Source commands: `db-review`, `practices-audit`

### Architecture — Defence-in-Depth Notes

- [x] **#49 — RBAC, OpenAPI sync, and dependency health all verified correct**
  - Resolution: `cargo-audit` is now installed and integrated. Ran `cargo update` to resolve 4 of 5 vulnerabilities (aes-gcm, bytes, ring, time) and 6 of 7 warnings (adler, paste, pest/pest_derive/pest_generator/pest_meta yanked versions). Migrated from unmaintained `rustls-pemfile` to `rustls-pki-types` `PemObject` trait (removed direct dependency). Only 1 unfixable advisory remains: `rsa` 0.9.10 via `jsonwebtoken` (RUSTSEC-2023-0071, no upstream fix available — tracked as #55). RBAC and OpenAPI sync remain correct.
  - Source commands: `rbac-rules`, `openapi-sync`, `dependency-check`

### Test Gaps (Earlier Round)

- [x] **#37 — No integration test for closed-order enforcement**
  - File: `tests/api_tests.rs`
  - Resolution: Already resolved — tests `closed_order_rejects_add_item`, `closed_order_rejects_update_item`, `closed_order_rejects_delete_item`, and `reopened_order_allows_item_mutations` were already present in the codebase at the time the finding was recorded. Confirmed present and passing (64/64 API integration tests pass).
  - Source command: `test-gaps`

- [x] **#38 — No integration test for `delete_user_by_email` RBAC fallback**
  - File: `tests/api_tests.rs`
  - Resolution: Added two integration tests: `non_admin_delete_by_email_nonexistent_returns_403` (verifies non-admin gets 403 for nonexistent email, preventing info leakage) and `admin_delete_by_email_nonexistent_returns_404` (verifies admin gets proper 404). Both pass against the test database.
  - Source command: `test-gaps`

- [x] **#39 — No WASM test for `authed_get` token refresh retry**
  - File: `frontend/tests/ui_tests.rs`
  - Resolution: Added `test_authed_get_retries_after_401_with_token_refresh` with a stateful fetch mock that tracks `/api/v1.0/users/` call count — returns 401 on first call, 200 on retry. Mock also handles `POST /auth/refresh` returning new tokens. Test verifies: dashboard renders with refreshed user details, `sessionStorage` contains updated tokens, and the user endpoint was called exactly twice (initial 401 + retry 200). All 22 WASM tests pass in headless Chrome.
  - Source command: `test-gaps`

### Backend — Redundant Token-Type Check

- [x] **#45 — `refresh_token` handler duplicates token-type check already enforced by middleware**
  - File: `src/handlers/users.rs` lines 93–98
  - Resolution: Kept the check as defence-in-depth and added a comment explaining that `refresh_validator` middleware already rejects non-refresh tokens before the handler is reached, so this check is a safety net. All 79 unit tests and 65 API integration tests pass.
  - Source commands: `review`, `security-audit`

### Frontend — Clippy Warning in Test File

- [x] **#46 — Useless `format!` in frontend test `ui_tests.rs`**
  - File: `frontend/tests/ui_tests.rs` line 842
  - Resolution: Replaced `format!(r#"..."#)` with `r#"..."#.to_string()` and un-escaped the double braces (`{{`/`}}`) to single braces (`{`/`}`) since format string escaping is no longer needed.
  - Source command: `review`

### Testing — Flaky DB Test

- [x] **#47 — `cleanup_expired_tokens_removes_old_entries` is flaky under parallel test execution**
  - File: `tests/db_tests.rs` lines 1987–2007
  - Resolution: Changed expiry from 1 hour ago to 1 day ago to avoid timing edge cases. Removed the `assert!(deleted >= 1)` global count assertion that was affected by parallel tests. Now only asserts on the specific JTI being removed after cleanup — the meaningful check. All 86 DB integration tests pass.
  - Source command: `test-gaps`

### Security — Missing CSP Headers for Static Files

- [x] **#48 — No Content-Security-Policy header on static file responses**
  - File: `src/server.rs` lines 293–303
  - Resolution: Wrapped the `actix-files` static file service in a `web::scope("")` with `DefaultHeaders` middleware that sets `Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'`. Policy allows WASM execution, inline scripts (required by Trunk's bootstrap), inline styles, and data URIs for images while restricting everything else to same-origin. Initially deployed without `'unsafe-inline'` in `script-src`, which caused a white-screen bug (fixed in commit `5ae1f07`). All tests pass.
  - Source commands: `security-audit`

### Security — Credentials Logged via `#[instrument]`

- [x] **#50 — `#[instrument]` on auth handlers doesn't skip credential parameters**
  - File: `src/handlers/users.rs` lines 65, 85, 129
  - Resolution: Updated all three `#[instrument]` annotations to skip credential parameters:
    - `auth_user`: `#[instrument(skip(basic, state), level = "debug")]`
    - `refresh_token`: `#[instrument(skip(credentials, state), level = "debug")]`
    - `revoke_user_token`: `#[instrument(skip(state, json), level = "debug")]`
  - Source commands: `security-audit`, `review`

### Documentation — CLAUDE.md `handlers/mod.rs` Description Incomplete

- [x] **#51 — `handlers/mod.rs` description in CLAUDE.md omits newer RBAC helpers**
  - File: `CLAUDE.md` line 62
  - Resolution: Updated the Project Structure description to list all current RBAC helpers: `require_admin, require_admin_or_team_admin, require_team_admin, require_team_member, require_self_or_admin, require_self_or_admin_or_team_admin, requesting_user_id`.
  - Source command: `practices-audit`

### Database — Missing DROP TABLE for token_blacklist

- [x] **#52 — `database.sql` missing `DROP TABLE IF EXISTS token_blacklist`**
  - File: `database.sql` line 16
  - Resolution: Added `DROP TABLE IF EXISTS token_blacklist;` after the existing `DROP TABLE IF EXISTS items;` block, before `CREATE EXTENSION`. Verified by `make test-integration` which re-runs the schema script on a fresh database (output shows `NOTICE: table "token_blacklist" does not exist, skipping`).
  - Source command: `db-review`

### Code Quality — Unused `require_self_or_admin` Helper

- [x] **#53 — `require_self_or_admin` helper is retained but never called**
  - File: `src/handlers/mod.rs` line 92
  - Resolution: Added `#[deprecated(note = "Use require_self_or_admin_or_team_admin instead")]` attribute to make the intent explicit. The function is kept for potential future use but callers will now see a deprecation warning. CLAUDE.md already documents the legacy status.
  - Source command: `review`

## Notes

- All 79 unit tests pass; 65 API integration tests pass; 86 DB integration tests pass; 22 WASM tests pass.
- Clippy is clean on both backend and frontend.
- `cargo-audit` reports 1 unfixable vulnerability (`rsa` 0.9.10 via `jsonwebtoken`, RUSTSEC-2023-0071) and 0 warnings. All other advisories resolved via `cargo update` and the `rustls-pemfile` → `rustls-pki-types` migration.
- `actix-files` CVE (GHSA-8v2v-wjwg-vx6r, GHSA-gcqf-3g44-vc9p) verified patched — `Cargo.lock` resolves to 0.6.10 (fixed version).
- CSP white-screen bug fixed in commit `5ae1f07`: added `'unsafe-inline'` to `script-src` so Trunk's inline WASM bootstrap script is not blocked by Chrome.
- All 25 completed items (#1, #6, #7, #15, #16, #37, #38, #39, #40, #41, #42, #43, #44, #45, #46, #47, #48, #49, #50, #51, #52, #53, #56, #57, #58) confirmed in place. No regressions.
- No critical, important, or minor findings remain open. Only 2 informational items (#54, #55) are open — both require no code changes.
- All 10 assessment commands verified: `api-completeness`, `db-review`, `dependency-check`, `openapi-sync`, `practices-audit`, `rbac-rules`, `review`, `security-audit`, `test-gaps`, `resume-assessment`.
- RBAC enforcement is correct across all handlers per the policy table in `rbac-rules.md`.
- OpenAPI spec (`middleware/openapi.rs`) is fully synchronized with `routes.rs` — all 37 handler paths present, all request/response schemas registered.
- CLAUDE.md conventions are followed consistently: error handling pattern, `#[instrument]` annotations, validation before DB calls, logging severity levels.
- CSP header documented in CLAUDE.md Key Conventions (completed item #57) and enforced in `server.rs` (completed item #48).