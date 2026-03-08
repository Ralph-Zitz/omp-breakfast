# Assessment Findings

Last assessed: 2026-03-08

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

_No open informational items._

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **CONNECT Design System:** `git pull` on 2026-03-08 reported "Already up to date." No migration needed.
- **`cargo audit`:** Clean (2026-03-08: 0 vulnerabilities in 438 dependencies).
- **`cargo fmt --all --check`:** Passes clean.
- **Test counts and migration version references** are no longer tracked in documentation files to prevent drift. See `migrations/` directory for current migrations. Run test suites to get current counts.
- Open items summary: 0 critical, 0 important, 0 minor, 0 informational.
- 13 new findings in this session: #727–#739.
- Highest finding number: #739.
- **0 regressions** — all 534 previously resolved items cross-checked, none regressed.
- **False positives discarded:** `edition = "2024"` (valid in Rust 1.85+, project uses 1.93.1), bootstrap_first_user DB test (#682 — resolved, API-level coverage sufficient), sessionStorage token storage (intentional design), CSP unsafe-inline for styles (CONNECT DS requirement), config secrets not SecretString (#708 — resolved), auth cache TTL (#710 — resolved), password column CHECK (#711 — resolved), email VARCHAR(254) (#712 — resolved), Docker Compose SSL mode (dev-only), CORS hardcoded port (same-origin), Swagger UI ENV-gated (#112 — resolved, no regression), TLS key in Docker image (documented DEV ONLY), account lockout per-IP (design trade-off), DB ports 127.0.0.1 (dev-only), jwt-compact age (accepted, monitored via cargo audit), color-eyre age (accepted, panic formatting only), frontend `Uuid.clone()` in admin.rs (#731 — false positive: frontend `user_id` is `String` not `Uuid`, clones are required for Leptos `FnMut`/`Fn` closures).

### Re-assessment — 2026-03-08

- **All 10 commands re-run + CONNECT Design System updated:** 13 new findings surfaced (0 critical, 2 important, 7 minor, 4 informational).
- **#714 (Important):** `delete_user`/`delete_user_by_email` will fail with FK violation if user has team orders.
- **#715 (Important):** Command file scope misses `frontend/src/api.rs` and `frontend/src/pages/`.
- **#716 (Minor):** Orders page fetches items/users without pagination limit.
- **#717 (Minor):** `unchecked_into` casts in teams page (3 occurrences).
- **#718 (Minor):** Unused `_is_admin` signal in OrdersPage.
- **#719 (Minor):** CLAUDE.md api.rs description inaccurate.
- **#720 (Minor):** README.md references postgres-setup in dev setup.
- **#721 (Minor):** PaginationParams::sanitize() has no unit tests.
- **#722 (Minor):** rust_decimal pulls ~20 unused transitive crates.
- **#723–#726 (Informational):** Orphaned DB function, CLAUDE.md omission, stale settings entry, Swagger UI without auth.
- **0 regressions** — all 534 resolved items cross-checked, none regressed.

### Re-assessment — 2026-03-15

- **All 10 commands re-run + CONNECT Design System updated:** 16 new findings surfaced (0 critical, 2 important, 8 minor, 6 informational).
- **#698 (Important):** README.md missing V17 migration, still says "sixteen".
- **#699 (Important):** No unit tests for JWT core functions.
- **#700–#701 (Minor):** CLAUDE.md test count drift — "93 tests" stale, sub-category sum is 95 not 97.
- **#702–#703 (Minor):** Frontend -- duplicated is_admin signal, 11 inline styles in order_components.
- **#704 (Minor):** Missing negative-path RBAC integration tests.
- **#705 (Minor):** Command files enumerate V1–V9 specifically.
- **#706 (Minor):** Avatar handler clones `Vec<u8>` on every cache hit despite Arc wrapper.
- **#707 (Minor):** FK DELETE RESTRICT error messages are ambiguous.
- **#708–#710 (Informational):** Security — config secrets not SecretString, CSP unsafe-inline, auth cache TTL.
- **#711–#712 (Informational):** DB — password column no CHECK, email VARCHAR(75) vs RFC 5321.
- **#713 (Informational):** init_dev_db.sh references V1–V9 for idempotent migrations.
- **0 regressions** — all 534 resolved items cross-checked, none regressed.

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
