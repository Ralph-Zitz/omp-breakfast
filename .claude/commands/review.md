Review the Rust codebase for quality, idioms, and improvements.

## Instructions

You are a senior Rust developer performing a code review. Analyze the entire `src/` directory and report findings organized by severity.

### What to check

1. **Rust idioms** — Are there non-idiomatic patterns? Unnecessary `.clone()`, `.unwrap()`, manual implementations that could use derive macros, or missing `#[must_use]` attributes?
2. **Error handling** — Are errors propagated correctly with `?`? Are `.unwrap()` or `.expect()` used in production code paths (not tests)? Are error messages descriptive?
3. **Code duplication** — Are there repeated patterns that could be extracted into shared functions or macros?
4. **Type safety** — Are there places where `String` is used but a newtype or enum would be more appropriate?
5. **Performance** — Unnecessary allocations, missing `&str` vs `String` optimizations, or repeated DB connection acquisitions?
6. **Dead code** — Unused imports, functions, or structs?
7. **Documentation** — Are public functions missing doc comments?

### Output format

For each finding, report:
- **File and line(s)**
- **Severity:** Critical / Warning / Suggestion
- **Description:** What the issue is
- **Recommendation:** How to fix it, with a code snippet if helpful

Group findings by file. Start with a brief overall assessment, then list findings, then end with a prioritized action list of the top 5 improvements to make.

### Scope

Read all `.rs` files under `src/`. Do NOT modify any files — this is analysis only.
