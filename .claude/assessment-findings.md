# Assessment Findings

Last assessed: 2026-03-10

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

### Frontend — Uses `String` for UUIDs Everywhere

- [x] **#321 — No type safety for UUID fields in frontend API types**
  - File: `frontend/src/api.rs`
  - Problem: All ID fields are `String`. A typo or wrong field could silently produce invalid requests.
  - Resolution: Accepted — a type alias (`type Id = String`) adds no real safety; only a newtype wrapper would, which requires invasive changes throughout the frontend. In WASM context, UUID serde roundtrips through strings regardless. No code change needed.
  - Source commands: `review`

### Security — Account Lockout State In-Memory Only

- [x] **#339 — Login attempt tracking stored in `DashMap`, not shared across instances**
  - File: `src/middleware/auth.rs` (lines ~189–213)
  - Problem: In multi-instance deployment, attacker can distribute brute-force attempts across instances.
  - Resolution: Accepted — this is a single-instance internal app (LEGO FabuLab). Shared lockout state (e.g. Redis) would add infrastructure complexity disproportionate to the threat model.
  - Source commands: `security-audit`

### API Completeness — Frontend `ItemEntry.price` Typed as `String`

- [x] **#366 — Frontend `ItemEntry` uses `pub price: String` instead of a numeric type**
  - File: `frontend/src/api.rs`
  - Problem: Backend returns `numeric(10,2)` as a JSON number; frontend deserializes as `String` which works but loses type safety for display and arithmetic.
  - Resolution: Accepted — the backend uses `rust_decimal` with `serde-with-str` feature, which serializes prices as JSON strings (`"9.99"`) not numbers. The frontend `String` type is correct and avoids floating-point precision issues.
  - Source commands: `api-completeness`

### Code Quality — Identical Create/Update Model Pairs in `models.rs`

- [x] **#375 — `CreateTeamEntry`/`UpdateTeamEntry`, `CreateRoleEntry`/`UpdateRoleEntry`, `CreateItemEntry`/`UpdateItemEntry` have identical fields**
  - File: `src/models.rs`
  - Problem: 3 pairs of structs are field-identical. Could be unified or type-aliased to reduce boilerplate.
  - Resolution: Accepted — separate types are intentional for distinct OpenAPI schema names. If type-aliased, `utoipa` would reference only the Create variant in the spec, confusing API consumers. The ~10 lines of duplication per pair is a reasonable trade-off for API documentation clarity.
  - Source commands: `review`

### Security — JWT Validator Performs DB Lookup on Every Request

- [x] **#381 — `jwt_validator` calls `db::get_user_by_email` on every authenticated request after cache miss**
  - File: `src/middleware/auth.rs`
  - Problem: The auth cache mitigates this for warm paths. Cold requests hit the DB. Not a bug, just a performance observation.
  - Resolution: Accepted — by design. The auth cache (5-min TTL, 1000 entries with O(n) partial-sort eviction) covers the warm path. Cold requests must verify the user still exists in the DB; this is a correct security behavior.
  - Source commands: `security-audit`

### Security — No Rate Limiting on Password Change Endpoint

- [x] **#382 — `PUT /api/v1.0/users/{id}` has no rate limiter for password changes**
  - File: `src/routes.rs`
  - Problem: An attacker with a valid session could brute-force test many passwords via the update endpoint. Low risk because the endpoint requires authentication.
  - Resolution: Accepted — the endpoint requires a valid JWT, the `current_password` field must match the stored Argon2 hash (slow to verify), and the PUT route shares a resource with GET/DELETE making selective rate-limiting architecturally complex. The combination of authentication + Argon2 cost makes brute-forcing impractical.
  - Source commands: `security-audit`

### Security — `delete_user_by_email` Email Existence Oracle

- [x] **#383 — DELETE endpoint returns 404 vs 204, revealing whether an email exists in the system**
  - File: `src/handlers/users.rs`
  - Problem: Low risk — endpoint is admin-gated. But the response difference is observable.
  - Resolution: Fixed — changed the not-found case to return HTTP 200 with `deleted: false` instead of 404, suppressing the email existence oracle. Admin check still enforced before the response. Updated OpenAPI spec and integration test.
  - Source commands: `security-audit`

### Testing — `auth_user` Cache Miss Path Untested

- [x] **#389 — No test verifies the code path when the auth cache has no entry for a user (first login or after TTL expiry)**
  - File: `src/middleware/auth.rs`
  - Resolution: Fixed — added two unit tests: `cache_miss_returns_none_for_unknown_user` (verifies fresh cache returns None) and `cache_miss_after_ttl_expiry` (verifies TTL-expired entries are treated as misses and evicted atomically).
  - Source commands: `test-gaps`

### Dependencies — `jwt-compact` Stale Maintenance

- [x] **#628 — Last release Oct 2023 (>2 years); no CVEs but maintenance risk grows**
  - File: `Cargo.toml`
  - Resolution: Accepted — no CVEs, no functional issues. Will reconsider if a vulnerability is disclosed or a superior alternative emerges. Monitoring via `cargo audit`.
  - Source commands: `dependency-check`

### Dependencies — `color-eyre` Stale Release

- [x] **#629 — Last release Dec 2022 (>3 years); still functional but gap is growing**
  - File: `Cargo.toml`
  - Resolution: Accepted — still functional, no CVEs. Used only for panic reports and developer error context. Will reconsider if it becomes incompatible with newer Rust editions.
  - Source commands: `dependency-check`

### Dependencies — OpenTelemetry Stack Always Compiled

- [x] **#630 — 4 OTel crates pull ~30 transitive deps; could be feature-gated for developers without an OTel collector**
  - File: `Cargo.toml`
  - Resolution: Fixed — made all 4 OTel crates optional behind a `telemetry` Cargo feature (default = on). Developers can build with `--no-default-features` to skip OpenTelemetry. `tracing-actix-web/opentelemetry_0_31` is also conditionally enabled. Server code in `server.rs` wrapped with `#[cfg(feature = "telemetry")]`.
  - Source commands: `dependency-check`

### Testing — Health Endpoint 503 Response Never Tested

- [x] **#394 — No integration test verifies that `/health` returns HTTP 503 when the database is unreachable**
  - File: `tests/api_tests.rs`
  - Resolution: Fixed — added `health_returns_503_when_db_unreachable` integration test. Creates a State with a pool pointing to unreachable port 1 with 200ms connect timeout, verifies the health endpoint returns 503 with `up: false`.
  - Source commands: `test-gaps`

### Database — `SET timezone` in V1 Is Session-Scoped Dead Code

- [x] **#438 — `SET timezone = 'Europe/Copenhagen'` only affects the migration connection session, not application connections**
  - File: `migrations/V1__initial_schema.sql` (line ~10)
  - Resolution: Accepted — cannot modify an already-applied Refinery migration (V1). The statement is harmless dead code. Application connections use UTC timestamps via `chrono::Utc` on the Rust side regardless.
  - Source commands: `db-review`

### Dependencies — `rustls` `tls12` Feature May Be Unnecessary

- [x] **#442 — Internal app could enforce TLS 1.3 only by removing `tls12` feature**
  - File: `Cargo.toml` (rustls dependency)
  - Resolution: Fixed — removed `tls12` from rustls features. The internal app now enforces TLS 1.3 only, which is simpler and avoids legacy cipher negotiation.
  - Source commands: `dependency-check`

### Dependencies — Three Versions of `getrandom` Compiled

- [x] **#443 — `getrandom` v0.2, v0.3, and v0.4 all compiled due to ecosystem version split**
  - File: `Cargo.toml` (transitive)
  - Resolution: Accepted — transitive dependency version split across the Rust ecosystem (uuid, argon2, password-hash, etc.). Cannot be fixed without upstream crate releases. Will consolidate naturally as the ecosystem converges.
  - Source commands: `dependency-check`

### Dependencies — `refinery` Pulls `toml` 0.8 Alongside `config`'s 0.9

- [x] **#444 — Duplicates the TOML parser; will resolve when `refinery` upgrades upstream**
  - File: `Cargo.toml` (transitive)
  - Resolution: Accepted — transitive dependency conflict between `refinery` (toml 0.8) and `config` (toml 0.9). Cannot be fixed without upstream `refinery` release.
  - Source commands: `dependency-check`

### Security — JWT HS256 With No Key Rotation Mechanism

- [x] **#447 — No `kid` claim or multi-key support; compromised secret requires full restart**
  - File: `src/middleware/auth.rs` (lines ~65–70)
  - Resolution: Accepted — for a single-instance internal app, the JWT secret is loaded from config/env and a full restart on secret compromise is acceptable. Key rotation with `kid` headers is an enterprise-grade feature beyond the current scope.
  - Source commands: `security-audit`

### Code Quality — Auth Cache Eviction O(n)

- [x] **#453 — `evict_oldest_if_full` iterates all 1000 entries to find oldest; fine at current scale**
  - File: `src/middleware/auth.rs`
  - Resolution: Accepted — the eviction already uses O(n) `select_nth_unstable_by_key` partial sort (not O(n log n) full sort) and evicts 10% at a time. At CACHE_MAX_SIZE=1000, iterating 1000 entries is sub-microsecond on modern hardware. No optimization needed.
  - Source commands: `review`

### API Completeness — `OrderItemEntry` vs Backend `OrderEntry` Naming Inconsistency

- [x] **#457 — Frontend renames the struct for clarity but creates naming mismatch with backend**
  - File: `frontend/src/api.rs` (lines ~96–104)
  - Resolution: Accepted — the frontend name `OrderItemEntry` is intentionally more descriptive than the backend's `OrderEntry`. The backend struct represents individual line items within a team order, so `OrderItemEntry` is the more accurate name. No rename needed.
  - Source commands: `api-completeness`

### API Completeness — Bulk Team Order Delete Endpoint Not Consumed

- [x] **#458 — `DELETE /api/v1.0/teams/{team_id}/orders` exists but has no frontend UI trigger**
  - File: `src/routes.rs`
  - Resolution: Accepted — kept for API completeness. External API consumers (scripts, tools, future admin UI features) may use these endpoints. Not every API endpoint needs a UI counterpart.
  - Source commands: `api-completeness`

### API Completeness — Delete-User-by-Email Endpoint Not Consumed

- [x] **#459 — AdminPage deletes by user_id only; the by-email endpoint is unreachable from UI**
  - File: `src/routes.rs`
  - Resolution: Accepted — kept for API completeness. The by-email endpoint serves administrative scripting use cases where email is more convenient than UUID lookup.
  - Source commands: `api-completeness`

### API Completeness — Single-Resource GET Endpoints Not Consumed (×5)

- [x] **#460 — Frontend always fetches via list endpoints; `GET /teams/{id}`, `GET /items/{id}`, `GET /roles/{id}`, single order, single order item all unused**
  - File: `src/routes.rs`
  - Resolution: Accepted — standard REST API design includes single-resource GET endpoints. They may be used by future features (deep linking, direct navigation) or external API consumers. Removing them would break API completeness.
  - Source commands: `api-completeness`

### Database — `items.price` CHECK Constraint Allows Zero

- [x] **#519 — `items.price CHECK (price >= 0)` permits items with zero cost; a breakfast ordering system likely doesn't intend free items**
  - File: `migrations/V1__initial_schema.sql`
  - Problem: No business-level protection against inserting cost-free items; the `validate_non_negative_price` custom validator also accepts zero.
  - Resolution: Fixed — changed `validate_non_negative_price` in `src/models.rs` to `price > 0` (strictly positive). Updated error message to "price must be positive" and updated the test from `accepts_zero` to `rejects_zero`. DB CHECK constraint cannot be altered (applied migration) but the application-level validator now prevents zero-price items.
  - Source commands: `db-review`

## Completed Items

Resolved items are maintained in [`.claude/resolved-findings.md`](.claude/resolved-findings.md), organized by original severity.
See that file for the full history of resolved findings.

## Notes

- **CONNECT Design System:** `git pull` on 2026-03-07 reported "Already up to date." No migration needed.
- **`cargo audit`:** Clean — 0 vulnerabilities in 437 dependencies.
- **`cargo fmt --all --check`:** Passes clean.
- **Test counts verified (2026-03-10):** 240 unit (218 lib + 22 healthcheck), 168 API integration (ignored), 112 DB integration (ignored), 79 WASM.
- Open items summary: 0 critical, 0 important, 0 minor, 0 informational.
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
