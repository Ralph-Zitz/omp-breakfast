# Assessment Findings

Last assessed: 2026-03-01 (updated)

This file is **generated and maintained by the project assessment process** defined in the "Project Assessment" section of `CLAUDE.md`. Each time `assess the project` is run, findings of all severities (critical, important, minor, informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will preserve completed items, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Important Items

No open important items. All important findings have been resolved and moved to "Completed Items" below.

## Minor Items

### Backend — Redundant Token-Type Check

- [ ] **#45 — `refresh_token` handler duplicates token-type check already enforced by middleware**
  - File: `src/handlers/users.rs` lines 95–99, `src/middleware/auth.rs` lines 223–230
  - Problem: The `refresh_token` handler manually checks `claims.claims.token_type != "refresh"` and returns `Error::Unauthorized`. However, the `refresh_validator` middleware (which guards the `/auth/refresh` route) already performs the same check and rejects non-refresh tokens before the handler is ever reached. The handler-level check is dead code under normal request flow.
  - Fix: Remove the `token_type` check from the `refresh_token` handler (lines 95–99 of `src/handlers/users.rs`) since `refresh_validator` already enforces it. Alternatively, keep it as defence-in-depth but add a comment explaining the redundancy.
  - Source commands: `review`, `security-audit`

### Frontend — Clippy Warning in Test File

- [ ] **#46 — Useless `format!` in frontend test `ui_tests.rs`**
  - File: `frontend/tests/ui_tests.rs` line 842
  - Problem: `cargo clippy --tests` warns about a `format!()` call that contains no format arguments — it should be a plain string with `.to_string()`. This is the only clippy warning in the frontend crate.
  - Fix: Replace `format!(r#"..."#)` with `r#"..."#.to_string()` as suggested by clippy, or run `cargo clippy --fix --test "ui_tests" -p breakfast-frontend`.
  - Source command: `review`

### Testing — Flaky DB Test

- [ ] **#47 — `cleanup_expired_tokens_removes_old_entries` is flaky under parallel test execution**
  - File: `tests/db_tests.rs` lines 1987–2011
  - Problem: This test inserts an expired token, runs `cleanup_expired_tokens`, and asserts `deleted >= 1`. When other tests running in parallel also insert expired tokens, the count may vary. More critically, if the test database retains expired tokens from a previous failed run, the assertion `!revoked_after` can fail because `cleanup_expired_tokens` uses `DELETE ... WHERE expires_at < NOW()` and timing can be tight with a 1-hour-ago expiry.
  - Fix: Use a unique `jti` (already done via `Uuid::now_v7()`) and assert specifically on whether that token was cleaned up, rather than relying on global count. Alternatively, wrap the test in a transaction that rolls back, isolating it from other tests.
  - Source command: `test-gaps`

### Security — Missing CSP Headers for Static Files

- [ ] **#48 — No Content-Security-Policy header on static file responses**
  - File: `src/server.rs` line 295
  - Problem: `actix-files` serves the frontend SPA from `frontend/dist/` without any `Content-Security-Policy` header. While the app uses `sessionStorage` (not cookies) and has no inline scripts in the built output, adding a CSP header would provide defence-in-depth against XSS. This is a hardening measure, not a vulnerability.
  - Fix: Add a middleware (e.g., `actix_web::middleware::DefaultHeaders`) that sets a `Content-Security-Policy` header with at minimum `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'`. Tune the policy to match the frontend's actual resource loading patterns.
  - Source commands: `security-audit`

## Informational Items

### Architecture — Defence-in-Depth Notes

- [ ] **#49 — RBAC, OpenAPI sync, and dependency health all verified correct**
  - Problem: No issues found. The RBAC enforcement matches the documented role policy table. OpenAPI spec is in sync with routes and frontend API usage. Dependencies are current with no known vulnerabilities (note: `cargo-audit` is not installed locally, so automated vulnerability scanning is skipped by `make test-all`).
  - Source commands: `rbac-rules`, `openapi-sync`, `dependency-check`
  - Action: No code changes needed. Consider installing `cargo-audit` for automated vulnerability scanning.

## Completed Items

Items moved here after being resolved:

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

### Test Gaps

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

## Notes

- All 79 unit tests pass; 65 API integration tests pass; 86 DB integration tests pass (1 flaky — see #47); 22 WASM tests pass.
- Clippy is clean on the backend; 1 minor warning on the frontend test file (see #46).
- `cargo-audit` is not installed locally; `make test-all` skips the audit step gracefully (see #49).