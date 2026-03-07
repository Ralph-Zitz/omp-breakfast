# Assessment Findings

Last assessed: 2026-03-14

This file is **generated and maintained by the project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, findings of all severities (critical, important, minor, and informational) are written here. The `/resume-assessment` command reads this file in future sessions to continue work.

**Do not edit manually** unless you are checking off a completed item. The assessment process will move completed items to `.claude/resolved-findings.md`, update open items (file/line references may shift), remove items no longer surfaced, and append new findings.

## How to use

- Run `/resume-assessment` in a new session to pick up where you left off
- Or say: "Read `.claude/assessment-findings.md` and help me work through the remaining open items."
- Check off items as they are completed by changing `[ ]` to `[x]`

## Critical Items

_No open critical items._

## Important Items

_No open important items._

## Minor Items

_No open minor items._

## Informational Items

### Frontend Design

- [ ] **#657 — `do_reset_password` sends all user fields — could overwrite concurrent admin edits**
  - **File:** `frontend/src/pages/admin.rs` L103–118
  - **Problem:** Admin password reset PUT sends `firstname`, `lastname`, `email` alongside the new password. If another admin changed these fields between page load and password reset, the old values would overwrite the changes.
  - **Risk:** Very low — requires concurrent admin editing of the same user during a password reset.
  - **Source:** review

- [ ] **#658 — Missing DB CHECK constraints on `users.firstname`/`users.lastname`**
  - **File:** `migrations/V11__text_column_check_constraints.sql`
  - **Problem:** V11 added CHECK constraints on `teams.tname`, `teams.descr`, `roles.title`, and `items.descr` but did not add constraints on `users.firstname`, `users.lastname`, or `users.email`. The Rust validator enforces max 50 chars for names and max 255 for email, but the DB does not mirror this.
  - **Note:** This is a defence-in-depth gap, not a functional bug (API validation catches oversized values).
  - **Source:** db-review

- [ ] **#665 — Frontend order total uses f64 instead of Decimal**
  - **File:** `frontend/src/pages/orders.rs`
  - **Problem:** Order total calculation in the frontend uses f64 arithmetic, which can produce floating-point rounding errors for monetary values. The backend uses `rust_decimal::Decimal`.
  - **Note:** For breakfast-order totals the rounding error is negligible, but it's architecturally inconsistent.
  - **Source:** review

### Dependencies

- [ ] **#659 — `password-hash` direct dependency may be redundant**
  - **File:** `Cargo.toml` L32
  - **Problem:** `password-hash = "0.5.0"` with `getrandom` feature is listed as a direct dependency, but all usage goes through `argon2::password_hash` re-exports. The `argon2` crate already enables `password-hash/default` and `password-hash/rand_core`.
  - **Note:** The `getrandom` feature may be needed to ensure `OsRng` works — needs careful testing before removal.
  - **Source:** dependency-check

- [ ] **#660 — Unused `MediaQueryList` web-sys feature**
  - **File:** `frontend/Cargo.toml` L14
  - **Problem:** `MediaQueryList` is listed in web-sys features but is never used in any frontend source file.
  - **Fix:** Remove `"MediaQueryList"` from the web-sys features list.
  - **Source:** dependency-check

- [ ] **#661 — `refinery` pulls native-tls despite project using rustls**
  - **Problem:** The `refinery` migration crate transitively pulls in `native-tls` even though the project exclusively uses `rustls` for TLS. This is a `refinery` limitation — no `rustls` feature flag is available.
  - **Note:** No practical fix available. Monitor for future `refinery` releases with rustls support.
  - **Source:** dependency-check

### Test Coverage Gaps

- [ ] **#662 — Auth validators have no unit tests**
  - **File:** `src/middleware/auth.rs`
  - **Problem:** `jwt_validator`, `basic_validator`, and `refresh_validator` are covered only by integration tests. No unit-level tests exist for token parsing, claim extraction, or error paths.
  - **Source:** test-gaps

- [ ] **#663 — Several DB functions untested at DB level**
  - **Files:** `src/db/orders.rs`, `src/db/order_items.rs`, `src/db/roles.rs`, `src/db/users.rs`
  - **Problem:** `count_team_orders`, `reopen_team_order`, `seed_default_roles`, `count_users`, and `get_order_total` have no dedicated DB-level tests. Some are exercised indirectly via API tests.
  - **Source:** test-gaps

### API Completeness

- [ ] **#664 — `get_order_total` DB function not exposed as API endpoint**
  - **File:** `src/db/order_items.rs`
  - **Problem:** The `get_order_total` function exists but is called only internally (by the `get_team_order` handler to populate the total field). There is no standalone `GET /api/v1.0/teams/{id}/orders/{id}/total` endpoint.
  - **Note:** Current design computes the total inline — no separate endpoint needed. Informational only.
  - **Source:** api-completeness

- [ ] **#666 — 8 documented API endpoints not consumed by frontend**
  - **Problem:** `DELETE /api/v1.0/teams/{id}/orders` (bulk delete), `GET /api/v1.0/roles` (list), individual role CRUD, and `GET /api/v1.0/avatars` (list) are not called from the frontend.
  - **Note:** Expected — these are admin/programmatic endpoints. No action needed.
  - **Source:** api-completeness

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **CONNECT Design System:** `git pull` on 2026-03-14 reported "Already up to date." No migration needed.
- **`cargo audit`:** Clean — 0 vulnerabilities in 437 dependencies.
- **`cargo fmt --all --check`:** Passes clean.
- **Test counts verified (2026-03-14):** 240 unit (218 lib + 22 healthcheck), 168 API integration (ignored), 112 DB integration (ignored), 79 WASM.
- Open items summary: 0 critical, 0 important, 0 minor, 10 informational.
- 24 new findings in session of 2026-03-14: #643–#666. 14 fixed (2 important + 12 minor), 10 informational remaining.
- Highest finding number: #666.

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
