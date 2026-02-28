Analyze test coverage gaps and suggest new tests to write.

## Instructions

You are a QA engineer specializing in Rust. Examine the entire codebase — both the source in `src/` and existing tests (inline `#[cfg(test)]` modules and `tests/`) — and identify what is NOT tested.

### Analysis steps

1. **Inventory existing tests** — List all test functions, what module they're in, and what they cover
2. **Map code paths** — For each public function in `src/`, determine whether it has test coverage
3. **Identify gaps** — Focus on:
   - `db.rs` — No unit tests exist (DB-dependent). Suggest how to test with mocks or test containers
   - `handlers/*.rs` — No handler-level tests. Suggest actix-web test harness patterns
   - `errors.rs` — Check if all `Error` variants are tested in `ResponseError` impl
   - `middleware/auth.rs` — Are edge cases covered (malformed tokens, empty passwords, expired by 1 second)?
   - `validate.rs` — Are boundary values tested (exactly min length, exactly max length)?
   - `routes.rs` — Is the routing configuration tested (correct paths, correct HTTP methods)?
4. **Suggest specific tests** — For each gap, write out the test function signature and a brief description of what it should assert

### Output format

Provide:
1. **Coverage summary table** — Module | Functions | Tested | Untested | Coverage %
2. **Critical gaps** — Tests that should exist but don't (ranked by risk)
3. **Suggested test code** — Ready-to-use `#[test]` or `#[actix_web::test]` function skeletons for the top 10 missing tests

### Integration tests

Integration tests in `tests/api_tests.rs` can now be run via `make test-integration`. This spins up a test Postgres on port 5433 using `docker-compose.test.yml`. When suggesting new integration tests, follow the existing pattern using `test_state()` (which reads `TEST_DB_PORT` env var) and mark them `#[ignore]`.

### Scope

Read all `.rs` files. Do NOT modify any files — this is analysis only.
