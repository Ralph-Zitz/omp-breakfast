# Assessment Findings

Last assessed: 2026-03-07

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will move completed items to `.claude/resolved-findings.md`, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

_No open critical items._

## Important Items

### Database — Race Condition in First-User Bootstrap

- [ ] **#633 — `bootstrap_first_user` allows concurrent first-user registrations with different emails**
  - File: `src/db/users.rs` (`bootstrap_first_user` function)
  - Problem: The bootstrap transaction uses default READ COMMITTED isolation. Two concurrent `POST /auth/register` requests could both see `count_users() == 0` and both create a "first user" with different emails. This bypasses the single-bootstrap design.
  - Fix: Add `SELECT pg_advisory_xact_lock(0)` at the start of the transaction, or use `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE`, or add `SELECT count(*) FROM users FOR UPDATE` to prevent concurrent readers from both seeing zero.
  - Source commands: `db-review`

### Testing — Avatars Feature Completely Untested

- [ ] **#634 — Zero API integration tests and zero DB integration tests for the avatars feature**
  - Files: `src/handlers/avatars.rs`, `src/db/avatars.rs`, `tests/api_tests.rs`, `tests/db_tests.rs`
  - Problem: All 4 avatar API endpoints (list, serve, set, remove) and all 5 avatar DB functions (get_avatars, get_avatar, insert_avatar, count_avatars, set_user_avatar) have no test coverage. Cache behavior, 404 handling, and RBAC are untested.
  - Fix: Add API integration tests for get_avatars, get_avatar (hit/miss), set_avatar (self, admin, forbidden), remove_avatar. Add DB tests for insert_avatar, get_avatar, count_avatars, set_user_avatar.
  - Source commands: `test-gaps`

## Minor Items

### Documentation — CLAUDE.md API Integration Test Count Stale (156 → 160)

- [ ] **#635 — CLAUDE.md states 156 API integration tests but actual count is 160**
  - File: `CLAUDE.md` (Testing section)
  - Fix: Update "156 API integration tests" to "160 API integration tests".
  - Source commands: `cross-ref-check`, `practices-audit`

### Documentation — README.md Test Counts and Migration Count Stale

- [ ] **#636 — README.md unit test count (236→238), API integration count (145→160), and migration count (12→13) diverge from reality**
  - File: `README.md`
  - Problem: README says 236 unit tests (actual 238), 145 API tests (actual 160), "twelve migrations" (actual 13). V13__pickup_user.sql is missing from the migration table.
  - Fix: Update all three counts, add V13 row to migration table.
  - Source commands: `cross-ref-check`

### Documentation — CLAUDE.md Missing 3 DB Functions in Inventory

- [ ] **#637 — `bootstrap_first_user`, `reopen_team_order`, `get_order_total` not listed in CLAUDE.md function inventories**
  - File: `CLAUDE.md` (Project Structure, db module descriptions)
  - Fix: Add `bootstrap_first_user` to users.rs list, `reopen_team_order` to orders.rs list, `get_order_total` to order_items.rs list.
  - Source commands: `cross-ref-check`

### Performance — Avatar Cache Clones Bytes on Every Request

- [ ] **#638 — Avatar image bytes are `Vec<u8>::clone()`d per response from the DashMap cache**
  - File: `src/handlers/avatars.rs`
  - Problem: Avatars are pre-resized PNGs (~10–50 KB each). Cloning on every serve negates some caching benefit. Under load, this creates unnecessary allocations.
  - Fix: Change `DashMap<Uuid, (Vec<u8>, String)>` to `DashMap<Uuid, (Arc<Vec<u8>>, String)>` for cheap reference-counted cloning.
  - Source commands: `review`

### Safety — Frontend `unchecked_into()` DOM Cast in order_components.rs

- [ ] **#639 — `target.unchecked_into::<HtmlInputElement>()` can panic if DOM element type doesn't match**
  - File: `frontend/src/pages/order_components.rs`
  - Problem: A WASM panic kills the entire SPA. While unlikely in practice (the target is always an input), the unchecked cast is fragile.
  - Fix: Use `target.dyn_ref::<web_sys::HtmlInputElement>()` with a guard clause instead.
  - Source commands: `review`

### Testing — Closed Order Update/Delete Enforcement Untested

- [ ] **#640 — No API test verifies that update/delete of order items is blocked on closed orders**
  - File: `tests/api_tests.rs`
  - Problem: Only adding items to closed orders is tested. Updating or deleting items on closed orders has no test coverage.
  - Fix: Add `cannot_update_item_in_closed_order` and `cannot_delete_item_from_closed_order` API integration tests.
  - Source commands: `test-gaps`

### Testing — Reopen Order Endpoint Untested

- [ ] **#641 — `POST /api/v1.0/teams/{team_id}/orders/{order_id}/reopen` has no API integration test**
  - File: `tests/api_tests.rs`
  - Problem: The reopen endpoint exists and is functional but has zero test coverage.
  - Fix: Add `reopen_closed_order_creates_new_open_order` API integration test.
  - Source commands: `test-gaps`

### Testing — Pickup User Assignment RBAC Untested

- [ ] **#642 — No test verifies pickup user validation (team membership check) or RBAC (admin/team-admin required for changes)**
  - File: `tests/api_tests.rs`
  - Problem: Assigning a non-team-member as pickup user and changing an existing pickup assignment by a non-admin are both untested.
  - Fix: Add `assign_pickup_user_not_in_team_returns_error` and `only_team_admin_can_change_pickup_user` API integration tests.
  - Source commands: `test-gaps`

## Informational Items

### Frontend — Uses `String` for UUIDs Everywhere

- [ ] **#321 — No type safety for UUID fields in frontend API types**
  - File: `frontend/src/api.rs`
  - Problem: All ID fields are `String`. A typo or wrong field could silently produce invalid requests.
  - Source commands: `review`

### Security — Account Lockout State In-Memory Only

- [ ] **#339 — Login attempt tracking stored in `DashMap`, not shared across instances**
  - File: `src/middleware/auth.rs` (lines ~189–213)
  - Problem: In multi-instance deployment, attacker can distribute brute-force attempts across instances.
  - Source commands: `security-audit`

### API Completeness — Frontend `ItemEntry.price` Typed as `String`

- [ ] **#366 — Frontend `ItemEntry` uses `pub price: String` instead of a numeric type**
  - File: `frontend/src/api.rs`
  - Problem: Backend returns `numeric(10,2)` as a JSON number; frontend deserializes as `String` which works but loses type safety for display and arithmetic.
  - Source commands: `api-completeness`

### Code Quality — Identical Create/Update Model Pairs in `models.rs`

- [ ] **#375 — `CreateTeamEntry`/`UpdateTeamEntry`, `CreateRoleEntry`/`UpdateRoleEntry`, `CreateItemEntry`/`UpdateItemEntry` have identical fields**
  - File: `src/models.rs`
  - Problem: 3 pairs of structs are field-identical. Could be unified or type-aliased to reduce boilerplate.
  - Source commands: `review`

### Security — JWT Validator Performs DB Lookup on Every Request

- [ ] **#381 — `jwt_validator` calls `db::get_user_by_email` on every authenticated request after cache miss**
  - File: `src/middleware/auth.rs`
  - Problem: The auth cache mitigates this for warm paths. Cold requests hit the DB. Not a bug, just a performance observation.
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

### Testing — `auth_user` Cache Miss Path Untested

- [ ] **#389 — No test verifies the code path when the auth cache has no entry for a user (first login or after TTL expiry)**
  - File: `src/middleware/auth.rs`
  - Source commands: `test-gaps`

### Dependencies — `jwt-compact` Stale Maintenance

- [ ] **#628 — Last release Oct 2023 (>2 years); no CVEs but maintenance risk grows**
  - File: `Cargo.toml`
  - Source commands: `dependency-check`

### Dependencies — `color-eyre` Stale Release

- [ ] **#629 — Last release Dec 2022 (>3 years); still functional but gap is growing**
  - File: `Cargo.toml`
  - Source commands: `dependency-check`

### Dependencies — OpenTelemetry Stack Always Compiled

- [ ] **#630 — 4 OTel crates pull ~30 transitive deps; could be feature-gated for developers without an OTel collector**
  - File: `Cargo.toml`
  - Source commands: `dependency-check`

### Testing — Health Endpoint 503 Response Never Tested

- [ ] **#394 — No integration test verifies that `/health` returns HTTP 503 when the database is unreachable**
  - File: `tests/api_tests.rs`
  - Source commands: `test-gaps`

### Database — `SET timezone` in V1 Is Session-Scoped Dead Code

- [ ] **#438 — `SET timezone = 'Europe/Copenhagen'` only affects the migration connection session, not application connections**
  - File: `migrations/V1__initial_schema.sql` (line ~10)
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

### Security — JWT HS256 With No Key Rotation Mechanism

- [ ] **#447 — No `kid` claim or multi-key support; compromised secret requires full restart**
  - File: `src/middleware/auth.rs` (lines ~65–70)
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

### Database — `items.price` CHECK Constraint Allows Zero

- [ ] **#519 — `items.price CHECK (price >= 0)` permits items with zero cost; a breakfast ordering system likely doesn't intend free items**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: No business-level protection against inserting cost-free items; the `validate_non_negative_price` custom validator also accepts zero.
  - Source commands: `db-review`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **CONNECT Design System:** `git pull` on 2026-03-07 reported "Already up to date." No migration needed.
- **`cargo audit`:** Clean — 0 vulnerabilities in 437 dependencies.
- **`cargo fmt --all --check`:** Passes clean.
- **Test counts verified (2026-03-07):** 238 unit (216 lib + 22 healthcheck), 160 API integration (ignored), 109 DB integration (ignored), 79 WASM.
- Open items summary: 0 critical, 2 important, 8 minor, 23 informational.
- 10 new findings in session of 2026-03-07: #633–#642.
- Highest finding number: #642.

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
