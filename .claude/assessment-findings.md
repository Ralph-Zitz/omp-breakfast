# Assessment Findings

Last assessed: 2026-03-07

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will move completed items to `.claude/resolved-findings.md`, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

### RBAC — Team Admin Can Reset Global Admin Password (Account Takeover)

- [ ] **#667 — `update_user` handler does not call `guard_admin_demotion` — Team Admin can reset a Global Admin's password**
  - File: `src/handlers/users.rs`, `update_user` handler (~line 395–453)
  - Problem: `update_user` calls `require_self_or_admin_or_team_admin` but does NOT call `guard_admin_demotion`. A Team Admin who shares a team with a Global Admin can call `PUT /api/v1.0/users/{admin_id}` with a new `password`. Since `is_self_update` is `false`, `current_password` verification is skipped. The Team Admin can then log in as the Global Admin with the new password — full account takeover.
  - Fix: Add `guard_admin_demotion(&client, &req, uid).await?;` after the `require_self_or_admin_or_team_admin` call and before the password-change logic. This prevents non-admin users from modifying a Global Admin's user record (including password). Alternatively, refine the guard to only block password changes by non-admins against admin users, while still allowing Team Admins to update non-sensitive fields (name, email) of admin users they share a team with.
  - Source commands: `rbac-rules`

## Important Items

### RBAC — Team Admin Can Delete a Global Admin's Account

- [ ] **#668 — `delete_user` and `delete_user_by_email` do not call `guard_admin_demotion` — Team Admin can delete a Global Admin**
  - File: `src/handlers/users.rs`, `delete_user` handler (~line 282–310), `delete_user_by_email` handler (~line 332–370)
  - Problem: Neither handler calls `guard_admin_demotion`. A Team Admin sharing a team with a Global Admin can delete the Admin's entire user account. This is more destructive than removing the Admin from a team (which IS guarded by `guard_admin_demotion` in `remove_team_member`). The existing last-admin self-deletion check only triggers for self-deletions, not cross-user deletions by a Team Admin.
  - Fix: Add `guard_admin_demotion(&client, &req, uid).await?;` after `require_self_or_admin_or_team_admin` in both `delete_user` and `delete_user_by_email`. This ensures only a Global Admin can delete another Global Admin's account.
  - Source commands: `rbac-rules`

### Database — `memberof.memberof_user_id` ON DELETE CASCADE Bypasses Last-Admin Guard

- [ ] **#669 — Deleting the last global admin user silently cascades through `memberof`, bypassing `guard_last_admin_membership`**
  - File: `migrations/V1__initial_schema.sql` (memberof FK definition)
  - Problem: `memberof.memberof_user_id` references `users(user_id) ON DELETE CASCADE`. If the last global admin is deleted via `delete_user()`, the CASCADE silently removes all their `memberof` rows — including the Admin role membership — without triggering `guard_last_admin_membership` (which only runs in membership-change handlers). The DB itself does not enforce a "≥1 admin" invariant.
  - Fix: Create a new migration changing the FK to `ON DELETE RESTRICT`: `ALTER TABLE memberof DROP CONSTRAINT memberof_memberof_user_id_fkey; ALTER TABLE memberof ADD CONSTRAINT memberof_memberof_user_id_fkey FOREIGN KEY (memberof_user_id) REFERENCES users(user_id) ON DELETE RESTRICT;`. This forces the application to explicitly remove team memberships before deleting a user, giving guards a chance to fire. Update `delete_user` handler to explicitly remove memberships first (which triggers the last-admin guard).
  - Source commands: `db-review`

### Database — `teamorders` and `orders` ON DELETE CASCADE FKs Violate Documented RESTRICT Convention

- [ ] **#670 — `teamorders.teamorders_team_id` and `orders.orders_team_id` use ON DELETE CASCADE despite documented convention of RESTRICT**
  - File: `migrations/V1__initial_schema.sql` (teamorders and orders FK definitions)
  - Problem: While the `delete_team` handler has an application-level 409 guard (#607), the DB-level CASCADE FKs remain on `teamorders.teamorders_team_id` and `orders.orders_team_id`. A direct DB operation or future code path bypassing the handler guard would destroy all order history silently. The documented convention in CLAUDE.md states "FK constraints use ON DELETE RESTRICT."
  - Fix: Create a new migration changing both FKs to `ON DELETE RESTRICT`. The handler-level guard already prevents team deletion when orders exist, so this change adds defense-in-depth.
  - Source commands: `db-review`

### Testing — Frontend First-User Registration Flow Has Zero Test Coverage

- [ ] **#671 — Login page dual-mode detection (`setup_required: true` → registration form) is completely untested**
  - File: `frontend/src/pages/login.rs`, `frontend/tests/ui_login.rs`
  - Problem: No WASM test mocks `/health` with `setup_required: true`. The registration form rendering, name/password validation in registration mode, and successful `POST /auth/register` flow are all untested.
  - Fix: Add WASM tests: (1) Mock `/health` → `{"up": true, "setup_required": true}`, verify registration form renders with name fields. (2) Submit with short password, verify validation error. (3) Submit valid data, mock `/auth/register` success, verify redirect to dashboard.
  - Source commands: `test-gaps`

### Testing — `authed_request` POST/PUT/DELETE Methods Untested

- [ ] **#672 — Only `authed_get` is tested — token refresh retry with body-forwarding for mutations has no coverage**
  - File: `frontend/src/api.rs`, `frontend/tests/ui_session.rs`
  - Problem: The `authed_request` function with `HttpMethod::Post`, `Put`, and `Delete` — including body-forwarding and 401 retry logic — has no test coverage. Every CRUD page depends on these functions.
  - Fix: Add WASM tests: (1) Mock fetch accepting POST with Auth header and body, verify correct transmission. (2) Mock 401-then-refresh-then-200 for POST with body, verify retry includes body.
  - Source commands: `test-gaps`

## Minor Items

### Database — `reopen_team_order` Uses `FOR SHARE` Instead of `FOR UPDATE`

- [ ] **#673 — Concurrent reopens of the same closed order could both succeed, creating duplicate copies**
  - File: `src/db/orders.rs`, `reopen_team_order` function (~line 195)
  - Problem: The source order row is locked with `FOR SHARE` (compatible with other `FOR SHARE` locks). Two concurrent reopen requests could both succeed, creating two separate copies with duplicated line items.
  - Fix: Change `FOR SHARE` to `FOR UPDATE` to serialize concurrent reopens.
  - Source commands: `db-review`

### Database — `users.email` VARCHAR(75) vs CHECK(≤255) Mismatch

- [ ] **#674 — V14 CHECK constraint on email is redundant and misleading**
  - File: `migrations/V14__user_text_check_constraints.sql`
  - Problem: The `email` column is `varchar(75)`, which already limits length to 75. V14 adds `CHECK (char_length(email) <= 255)`, which can never fire. The CHECK misleadingly suggests 255 is the limit.
  - Fix: In a new migration, either change the CHECK to `<= 75` to match the column type, or drop the redundant CHECK entirely.
  - Source commands: `db-review`

### Database — `items.price` DB CHECK Allows 0 While API Requires > 0

- [ ] **#675 — DB constraint `CHECK (price >= 0)` is weaker than API validator `validate_positive_price()` which requires `> 0`**
  - File: `migrations/V1__initial_schema.sql` (items CHECK constraint)
  - Problem: A direct DB insert could create a zero-price item. The V6 migration fixed a similar gap for `orders.amt` (changed `>= 0` to `>= 1`) but didn't address `price`.
  - Fix: In a new migration: `ALTER TABLE items DROP CONSTRAINT items_price_check; ALTER TABLE items ADD CONSTRAINT items_price_check CHECK (price > 0);`
  - Source commands: `db-review`

### Database — `memberof.memberof_team_id` ON DELETE CASCADE Allows Silent Membership Loss

- [ ] **#676 — Deleting a team silently removes all memberships via CASCADE, bypassing potential application guards**
  - File: `migrations/V1__initial_schema.sql` (memberof FK to teams)
  - Problem: While the handler has a 409 guard for orders, membership removal is unguarded at the DB level. A direct DB operation would silently remove all members.
  - Fix: In a new migration, change the FK to `ON DELETE RESTRICT`. Lower priority than #669 and #670.
  - Source commands: `db-review`

### Documentation — README.md Unit Test Count Stale (238 → 248)

- [ ] **#677 — README.md says "238 tests" but actual unit test count is 248**
  - File: `README.md`
  - Fix: Update "238 tests" to "248 tests".
  - Source commands: `cross-ref-check`

### Documentation — README.md Integration Test Counts Wrong

- [ ] **#678 — README.md says "279 tests: 167 API + 112 DB" but actual is 288: 168 API + 120 DB**
  - File: `README.md`
  - Fix: Update counts to "(288 tests: 168 API + 120 DB)".
  - Source commands: `cross-ref-check`

### Documentation — README.md Migration Count Missing V14

- [ ] **#679 — README.md says "Thirteen migrations" but disk has 14 (V1–V14). Missing V14 row in table.**
  - File: `README.md`
  - Fix: Change "Thirteen" to "Fourteen" and add V14 row to the migration table.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md "Planned Pages" Section Describes Already-Implemented Pages

- [ ] **#680 — "Frontend Roadmap" → "Planned Pages" lists 6 pages as future work, but all 6 are fully implemented**
  - File: `CLAUDE.md` (Frontend Roadmap section)
  - Problem: Lists Team Management, Order Management, Item Catalog, User Profile, Admin Dashboard, Role Management as planned — all exist in `frontend/src/pages/`. Misleading for developers reading the doc.
  - Fix: Rename "Planned Pages" to "Implemented Pages" or integrate into the existing Frontend Architecture section. Update the "Frontend Roadmap" intro text to reflect the SPA is complete.
  - Source commands: `practices-audit`

### Testing — `delete_team` With Existing Orders Returns 409 — No API Test

- [ ] **#681 — Handler-level guard preventing team deletion when orders exist has zero test coverage**
  - File: `src/handlers/teams.rs` `delete_team` handler, `tests/api_teams.rs`
  - Fix: Add API integration test: create team, create order, attempt `DELETE /api/v1.0/teams/{id}` → assert 409. Delete the order, retry → assert 200.
  - Source commands: `test-gaps`

### Testing — `bootstrap_first_user` DB Function Has No Direct DB Test

- [ ] **#682 — Multi-step transactional function tested only indirectly via API tests**
  - File: `src/db/users.rs` `bootstrap_first_user`, `tests/db_users.rs`
  - Fix: Add DB test: call `bootstrap_first_user` directly → verify user created, 4 roles seeded, "Default" team exists, user is Admin. Call again → assert `Error::Forbidden`.
  - Source commands: `test-gaps`

### Testing — `register_first_user` Validation Error Path Not Tested at API Level

- [ ] **#683 — 422 validation path (short password, missing name) has no API integration test**
  - File: `src/handlers/users.rs` `register_first_user`, `tests/api_auth.rs`
  - Fix: On fresh DB, POST to `/auth/register` with a 3-char password → assert 422. POST with missing firstname → assert 422.
  - Source commands: `test-gaps`

### Testing — Teams Page CRUD Interactions Not Tested in WASM

- [ ] **#684 — 905 lines of code with only 2 rendering tests — no dialog open/submit/toast tests**
  - File: `frontend/src/pages/teams.rs`, `frontend/tests/ui_pages.rs`
  - Fix: Add WASM tests for create-team dialog, edit-team dialog, add-member dialog, remove-member confirmation, toast notifications.
  - Source commands: `test-gaps`

### Testing — Items Page CRUD Interactions Not Tested in WASM

- [ ] **#685 — No create/edit/delete/validation tests for items page**
  - File: `frontend/src/pages/items.rs`, `frontend/tests/ui_pages.rs`
  - Fix: Add WASM tests for create item (form, price input), edit item (pre-populated dialog), delete item (confirmation modal).
  - Source commands: `test-gaps`

### Testing — Orders Page Detail and Line Item Management Not Tested in WASM

- [ ] **#686 — 1260 lines across 2 files with only 1 test (dialog opens)**
  - File: `frontend/src/pages/orders.rs`, `frontend/src/pages/order_components.rs`, `frontend/tests/ui_pages.rs`
  - Fix: Add WASM tests for selecting an order, viewing detail, adding/removing line items, closing/reopening orders, order total display.
  - Source commands: `test-gaps`

## Informational Items

### Security — Account Lockout Enables DoS Against Any User

- [ ] **#687 — Lockout is per-email only with no IP component — any unauthenticated attacker can lock any account**
  - File: `src/middleware/auth.rs` (~line 232–257)
  - Problem: An attacker who knows a valid email can send 5 incorrect login attempts to lock that account for 15 minutes. `actix-governor` rate-limits per-IP, but the lockout itself is per-email from any IP.
  - Note: In a small team deployment (FabuLab internal), the risk is lower. Consider combining lockout key with client IP, or implementing CAPTCHA after N failures.
  - Source commands: `security-audit`

### Security — `login_attempts` DashMap Grows Unbounded

- [ ] **#688 — Unlike `token_blacklist`, no periodic cleanup task for stale login attempt entries**
  - File: `src/middleware/auth.rs` (~line 247–257), `src/models.rs` (~line 117)
  - Problem: Old entries are only pruned when that specific email is checked again. Entries for targeted unique emails accumulate indefinitely.
  - Note: Add a periodic background task (similar to `spawn_token_cleanup_task`) to sweep stale entries, or add a max-size eviction policy.
  - Source commands: `security-audit`

### Security — JWT Secret Stored as Plain String

- [ ] **#689 — JWT secret in `State.jwtsecret` is a plain `String` — no zeroization on drop**
  - File: `src/models.rs` (~line 113)
  - Note: Consider `secrecy::SecretString` for zeroization on drop and `Debug` as `[REDACTED]`. Low priority — requires core dump access to exploit.
  - Source commands: `security-audit`

### Security — Swagger UI Enabled by Default on Non-Production

- [ ] **#690 — If a non-production environment is publicly accessible, full OpenAPI spec is exposed**
  - File: `src/routes.rs` (~line 29–32)
  - Note: The `ENABLE_SWAGGER` env var override exists, but default-on for non-production is the concern. Consider defaulting to off and requiring explicit opt-in.
  - Source commands: `security-audit`

### Database — No CHECK Constraints on `avatars` Text Columns

- [ ] **#691 — `name` and `content_type` columns have no length constraints**
  - File: `migrations/V8__avatars.sql`
  - Note: Low risk — avatars are only seeded from `minifigs/` at startup, not user-provided. Add `CHECK (char_length(name) <= 255)` and `CHECK (char_length(content_type) <= 100)` for consistency.
  - Source commands: `db-review`

### Database — `orders.orders_team_id` CASCADE Creates Redundant Deletion Path

- [ ] **#692 — Two independent CASCADE paths from `teams` to `orders` exist**
  - File: `migrations/V1__initial_schema.sql`
  - Note: PostgreSQL handles this correctly, but it makes deletion behavior harder to audit. Moot if #670 is addressed.
  - Source commands: `db-review`

### Testing — Duplicate Role Creation at API Level Not Tested

- [ ] **#693 — DB test exists for duplicate role, but no API test verifies 409 through HTTP stack**
  - File: `tests/api_roles.rs`
  - Fix: Create role "TestRole", then create "TestRole" again → assert 409 Conflict.
  - Source commands: `test-gaps`

### Testing — Pickup User RBAC Edge Cases Not Fully Covered

- [ ] **#694 — Admin changing pickup user, clearing pickup user (`null`) not tested**
  - File: `tests/api_orders.rs`
  - Fix: Test admin can change existing pickup user. Test admin can clear pickup user with `pickup_user_id: null`.
  - Source commands: `test-gaps`

### Testing — Delete Role/Item Referenced by Existing Records Not Tested at API Level

- [ ] **#695 — FK constraint 409 responses not tested through HTTP stack**
  - File: `tests/api_roles.rs`, `tests/api_items.rs`
  - Fix: Create role + assign to member, delete role → assert 409. Create item + add to order, delete item → assert 409.
  - Source commands: `test-gaps`

### Testing — Profile Page Form Submission Not Tested in WASM

- [ ] **#696 — Edit/save profile and password-change flow have no WASM tests**
  - File: `frontend/src/pages/profile.rs`, `frontend/tests/ui_pages.rs`
  - Fix: Test edit-mode submit (PUT), password change flow (current password → new password → confirm), error states.
  - Source commands: `test-gaps`

### Testing — Admin/Roles Page CRUD Submission Flows Not Tested in WASM

- [ ] **#697 — Dialog fields tested but submit success/error paths not tested**
  - File: `frontend/src/pages/admin.rs`, `frontend/src/pages/roles.rs`, `frontend/tests/ui_admin_dialogs.rs`
  - Fix: Test create-user submit → toast success. Test delete-user confirm modal → toast success. Test create-role submit flow.
  - Source commands: `test-gaps`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **CONNECT Design System:** `git pull` on 2026-03-07 reported "Already up to date." No migration needed.
- **`cargo audit`:** Clean — 0 vulnerabilities in 437 dependencies.
- **`cargo fmt --all --check`:** Passes clean.
- **Test counts verified (2026-03-07):** 248 unit (226 lib + 22 healthcheck), 168 API integration (ignored), 120 DB integration (ignored), 79 WASM.
- Open items summary: 1 critical, 5 important, 14 minor, 11 informational.
- 31 new findings in this session: #667–#697.
- Highest finding number: #697.
- **0 regressions** — all 517 previously resolved items cross-checked, none regressed.
- **False positives discarded:** Avatar cache clone (#638), frontend `expect_context` panics, hardcoded role strings in frontend, Duration::expect() safety, TOCTOU on `guard_last_admin_membership` (resolved in #643).

### Re-assessment — 2026-03-14

- **All 10 commands re-run + CONNECT Design System updated:** 24 new findings surfaced (0 critical, 2 important, 12 minor, 10 informational).
- **#643 (Important):** TOCTOU race in last-admin guard for `remove_team_member` — guard runs outside transaction.
- **#644 (Important):** No JSON body size limit on `/auth/refresh` endpoint.
- **#645–#650 (Minor):** Documentation drift — CLAUDE.md project structure, test counts, function inventories; command files reference deleted/renamed files.
- **#651–#653 (Minor):** Frontend bugs — wrong CSS class on Remove Avatar button, missing maxlength on profile first name, `is_valid_price` accepts scientific notation.
- **#654–#656 (Minor):** Backend consistency — `count_avatars` skips prepared statement, misleading function name, unused component prop.
- **#657–#666 (Informational):** UI race condition in password reset, missing DB CHECK constraints, dependency notes, test coverage gaps, API completeness notes.
- **False positives discarded:** Avatar cache clone (#638), frontend unchecked DOM cast (#639), DB price constraint (#519), bootstrap race (#633) — all verified still resolved in current code.
- **0 regressions** — all 493 resolved items cross-checked, none regressed.

### Re-assessment — 2026-03-07

- **All 10 commands re-run + CONNECT Design System updated:** 10 new findings surfaced (0 critical, 2 important, 8 minor, 0 new informational).
- **#633 (Important):** Bootstrap race condition — concurrent first-user registrations could both succeed.
- **#634 (Important):** Avatars feature has zero test coverage (API + DB).
- **#635–#637 (Minor):** Documentation drift — CLAUDE.md test count, README.md counts/migration, CLAUDE.md function inventories.
- **#638 (Minor):** Avatar cache clones bytes inefficiently.
- **#639 (Minor):** Frontend unchecked DOM cast.
- **#640–#642 (Minor):** Missing API tests for closed order enforcement, reopen endpoint, pickup user RBAC.
- **False positive discarded:** Practices audit flagged `edition = "2024"` as invalid — this is valid in Rust 1.85+ (project uses 1.93.1).
- **Security audit note:** TLS keys in Dockerfile already documented as DEV ONLY with production instructions in comments — not a new finding.

### Re-assessment — 2026-03-06

- **All 11 commands re-run:** 8 new findings surfaced (0 critical, 1 important, 3 minor, 4 informational).
- **#515 (Important):** README.md migration table missing V8 — says "Seven" but 8 exist on disk.
- **#513 (Minor):** `get_avatar` utoipa annotation falsely claims JWT auth, but endpoint is public.
- **#514, #516 (Minor):** Test count drift in CLAUDE.md (198→199) and README.md (193→199).
- **#517–#520 (Informational):** `database.sql` missing V8, missing FK index on `users.avatar_id`, price CHECK allows zero, frontend lacks price format validation.
- **0 regressions** — all 354+ resolved items spot-checked, none regressed.
- **Unit tests:** 199 passing (177 lib + 22 healthcheck). `cargo fmt`: clean. `cargo audit`: exit 0.
- **CONNECT Design System:** Updated. New commits pulled. No breaking changes to components used by frontend.
