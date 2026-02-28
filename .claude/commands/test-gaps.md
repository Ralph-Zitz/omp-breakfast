Analyze test coverage gaps and suggest new tests to write.

## Instructions

You are a QA engineer specializing in Rust. Examine the entire codebase — backend source in `src/`, backend tests (inline `#[cfg(test)]` modules and `tests/`), frontend source in `frontend/src/`, and frontend tests (`frontend/tests/`) — and identify what is NOT tested.

### Analysis steps — Backend

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

### Analysis steps — Frontend (Leptos WASM)

1. **Inventory existing WASM tests** — List all test functions in `frontend/tests/ui_tests.rs` and what they cover
2. **Map frontend code paths** — For each component and function in `frontend/src/app.rs`, determine whether it has test coverage
3. **Identify frontend gaps** — Focus on:
   - **Component rendering** — Are all components tested for correct HTML output?
   - **Signal reactivity** — Are state transitions tested (e.g., page switching, error state changes)?
   - **Edge cases** — Empty responses, malformed JSON, network timeouts, expired tokens
   - **Auth flow** — Is token refresh tested? Is localStorage full/unavailable tested?
   - **Validation** — Are all client-side validation rules tested (whitespace-only input, very long input)?
   - **Accessibility** — Are ARIA attributes and form labels verified in tests?
4. **Suggest WASM tests** — For each gap, write out a `#[wasm_bindgen_test]` function skeleton following the existing mock pattern (override `window.fetch` via `js_sys::eval`)

### Output format

Provide:

1. **Coverage summary table** — Module | Functions | Tested | Untested | Coverage %
   - Include separate sections for backend and frontend
2. **Critical gaps** — Tests that should exist but don't (ranked by risk)
3. **Suggested test code** — Ready-to-use `#[test]`, `#[actix_web::test]`, or `#[wasm_bindgen_test]` function skeletons for the top 10 missing tests (combining backend and frontend)

### Integration tests

Integration tests in `tests/api_tests.rs` can now be run via `make test-integration`. This spins up a test Postgres on port 5433 using `docker-compose.test.yml`. When suggesting new integration tests, follow the existing pattern using `test_state()` (which reads `TEST_DB_PORT` env var) and mark them `#[ignore]`.

### Frontend tests

Frontend WASM tests in `frontend/tests/ui_tests.rs` run via `make test-frontend` (uses `wasm-pack test --headless --chrome`). When suggesting new WASM tests, follow the existing patterns:

- Use `wasm_bindgen_test::wasm_bindgen_test` attribute
- Mount components by appending to `document().body()`
- Mock HTTP via `js_sys::eval` to override `window.fetch`
- Access DOM via `document().query_selector()` assertions
- Use `gloo_timers::future::sleep` for async timing

### Scope

Read all `.rs` files under `src/`, `tests/`, `frontend/src/`, and `frontend/tests/`. Do NOT modify any files — this is analysis only.
