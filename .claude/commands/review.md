Review the Rust codebase for quality, idioms, and improvements.

## Instructions

You are a senior Rust developer performing a code review. Analyze the entire `src/` directory (backend) and `frontend/src/` directory (Leptos WASM frontend) and report findings organized by severity.

### What to check — Backend (`src/`)

1. **Rust idioms** — Are there non-idiomatic patterns? Unnecessary `.clone()`, `.unwrap()`, manual implementations that could use derive macros, or missing `#[must_use]` attributes?
2. **Error handling** — Are errors propagated correctly with `?`? Are `.unwrap()` or `.expect()` used in production code paths (not tests)? Are error messages descriptive?
3. **Code duplication** — Are there repeated patterns that could be extracted into shared functions or macros?
4. **Type safety** — Are there places where `String` is used but a newtype or enum would be more appropriate?
5. **Performance** — Unnecessary allocations, missing `&str` vs `String` optimizations, or repeated DB connection acquisitions?
6. **Dead code** — Unused imports, functions, or structs?
7. **Documentation** — Are public functions missing doc comments?

### What to check — Frontend (`frontend/src/`)

1. **Leptos idioms** — Are components using `#[component]` correctly? Are signals created and consumed idiomatically (no unnecessary cloning of signals, proper use of closures)?
2. **Reactive patterns** — Are `ReadSignal`/`WriteSignal` pairs used correctly? Are there derived signals that should use `Memo` or `derived` instead? Are signals scoped appropriately (not leaked)?
3. **Component structure** — Is the component hierarchy well-organized? Are components too large and should be split? Are props used where signals are passed around manually?
4. **Error handling** — Are fetch errors handled gracefully? Are `.unwrap()` calls in WASM code guarded (panics crash the WASM module)?
5. **Client-side security** — Is localStorage access safe? Is user input sanitized before display? Are tokens handled securely?
6. **API integration** — Do HTTP calls match the backend's expected request/response shapes? Are API URLs consistent with backend routes?
7. **Dead code** — Unused components, signals, or imports?
8. **Accessibility** — Are form inputs properly labeled? Are interactive elements keyboard-accessible?

### Output format

For each finding, report:
- **File and line(s)**
- **Severity:** Critical / Warning / Suggestion
- **Description:** What the issue is
- **Recommendation:** How to fix it, with a code snippet if helpful

Group findings by file, with backend files first and frontend files second. Start with a brief overall assessment, then list findings, then end with a prioritized action list of the top 5 improvements to make.

### Scope

Read all `.rs` files under `src/` and `frontend/src/`. Also reference `frontend/Trunk.toml` for build configuration and `CLAUDE.md` for project conventions. Do NOT modify any files — this is analysis only.
