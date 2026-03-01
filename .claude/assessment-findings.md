# Assessment Findings

Last assessed: 2025-07-14

This file is **generated and maintained by the project assessment process** defined in the "Project Assessment" section of `CLAUDE.md`. Each time `assess the project` is run, critical and important findings are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will preserve completed items, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Important Items

### Backend — Error Response Consistency

- [ ] **#15 — `auth_user` returns bare string instead of `ErrorResponse`**
  - File: `src/handlers/users.rs` lines 70–73
  - Problem: When the user is not in the auth cache, the handler returns
    `Ok(HttpResponse::Unauthorized().json("Unauthorized"))` — a bare JSON string.
    All other error paths return `ErrorResponse { error: "..." }`.
  - Fix: Return `Ok(HttpResponse::Unauthorized().json(ErrorResponse { error: "Unauthorized".to_string() }))`,
    or better, return `Err(Error::ActixAuth(...))` to use the centralized error handler.
  - Source command: `review`

- [ ] **#16 — `refresh_token` handler bypasses centralized error handling**
  - File: `src/handlers/users.rs` lines 81–96
  - Problem: Invalid token type and revoked token cases return
    `Ok(HttpResponse::Unauthorized().json(ErrorResponse { ... }))` instead of using
    `Err(Error::...)`. This bypasses tracing/logging in the `ResponseError` impl.
  - Fix: Introduce an `Error::Unauthorized(String)` variant or use the existing
    `Error::ActixAuth` to route through `ResponseError`. Apply the same fix to
    `revoke_user_token` if applicable.
  - Source command: `review`

### Frontend — Token Revocation on Logout

- [ ] **#1 — Frontend logout does not revoke tokens server-side**
  - File: `frontend/src/app.rs` — `on_logout` closure in `DashboardPage`
  - Problem: The logout handler clears `sessionStorage` but never calls
    `POST /auth/revoke`. The old access token and refresh token remain valid
    on the server until they expire naturally (15 min / 7 days).
  - Fix: Before clearing storage, send the access token (and optionally the
    refresh token) to `POST /auth/revoke` with the current bearer token in
    the `Authorization` header. The revoke endpoint expects a JSON body
    `{ "token": "<token_to_revoke>" }`. Fire-and-forget is acceptable if the
    user experience should not block on the network call.
  - Source commands: `api-completeness`, `security-audit`

### Database — Inconsistent Row Mapping Pattern

- [ ] **#6 — `get_team_users` uses `.map()` instead of `filter_map` + `warn!()`**
  - File: `src/db.rs` lines 301–331
  - Problem: Other list queries (`get_users`, `get_teams`, `get_roles`,
    `get_items`, `get_team_orders`, `get_order_items`) use `filter_map` with
    a `warn!()` log when a row fails to map. `get_team_users` uses a plain
    `.map()` which would panic on an unexpected row shape.
  - Fix: Change `.map(|row| UsersInTeam { ... })` to
    `.filter_map(|row| match ... { Ok(v) => Some(v), Err(e) => { warn!(...); None } })`
    following the pattern in `get_users`. Note: `UsersInTeam` is not derived
    with `PostgresMapper`, so either add the derive or use a manual
    `TryFrom`/fallible extraction with `row.try_get()`.
  - Source commands: `db-review`, `practices-audit`

- [ ] **#7 — `get_user_teams` has the same `.map()` issue**
  - File: `src/db.rs` lines 182–211
  - Problem: Same as #6 — uses `.map()` instead of `filter_map` + `warn!()`.
    Applies to `UserInTeams` struct.
  - Fix: Same approach as #6.
  - Source commands: `db-review`, `practices-audit`

### Test Gaps

- [ ] **#37 — No integration test for closed-order enforcement**
  - File: `tests/api_tests.rs` (new tests to add)
  - Problem: `create_order_item`, `update_order_item`, and `delete_order_item`
    check `is_team_order_closed` and return 403 when the order is closed, but
    no integration test covers this path.
  - Fix: Add three integration tests that create a team order, close it via
    `PUT`, then attempt to add/update/delete an order item and assert 403.
  - Source command: `test-gaps`

- [ ] **#38 — No integration test for `delete_user_by_email` RBAC fallback**
  - File: `tests/api_tests.rs` (new test to add)
  - Problem: When the target email doesn't exist, the handler falls back to
    `require_admin` to prevent information leakage (non-admins can't discover
    whether an email exists). This path is not tested.
  - Fix: Add a test where a non-admin user calls
    `DELETE /api/v1.0/users/email/nonexistent@example.com` and assert 403.
    Add a second test where an admin calls the same endpoint and assert 404.
  - Source command: `test-gaps`

- [ ] **#39 — No WASM test for `authed_get` token refresh retry**
  - File: `frontend/tests/ui_tests.rs` (new test to add)
  - Problem: `authed_get` retries a request with a refreshed token when the
    initial response is 401, but no WASM test verifies this retry behavior.
  - Fix: Add a `wasm_bindgen_test` that mocks `window.fetch` to return 401
    on the first call, then return a valid refresh response, then return 200
    on the retry. Assert that the final result succeeds and that
    `sessionStorage` contains the new token.
  - Source command: `test-gaps`

## Completed Items

Items moved here after being resolved:

_(none yet)_

## Notes

- Full assessment also identified Minor and Informational findings not tracked here.
- To see the complete assessment, run the project assessment again or check git history.
- RBAC enforcement, OpenAPI sync, and settings were all verified correct — no issues found.
- All 78 unit tests pass; 62 API and 86 DB integration tests are correctly `#[ignore]`d.