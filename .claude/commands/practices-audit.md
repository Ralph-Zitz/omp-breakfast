Audit the project against its own documented practices and conventions to ensure everything is consistent and current.

## Instructions

You are a project health auditor. Read `CLAUDE.md` (the source of truth for project conventions), all `.claude/commands/*.md` files, and the actual codebase to verify that documentation, commands, and code are all in sync.

### Audit areas

#### 1. CLAUDE.md accuracy

Read `CLAUDE.md` and verify every factual claim against the actual codebase:

- **Tech Stack** — Are all listed crates/tools still in `Cargo.toml` and `frontend/Cargo.toml`? Have any been added or removed without updating the doc?
- **Build & Run** — Do all listed commands (`cargo build`, `make build`, etc.) still work as described? Are there new commands not documented?
- **Project Structure** — Does the file tree in CLAUDE.md match the actual directory layout? Are there new files or directories not listed?
- **Key Conventions** — For each convention listed (e.g., "Every handler returns `Result<impl Responder, Error>`"), spot-check at least 3 handlers to verify compliance
- **Frontend Architecture** — Does the component hierarchy match the actual code? Are signal patterns described accurately?
- **Testing** — Do the test counts match reality? (Count `#[test]`, `#[actix_web::test]`, and `#[wasm_bindgen_test]` across the codebase)
- **Unfinished Work** — Are items still unfinished, or have some been completed without updating the doc? Are there new unfinished items not listed?
- **Markdown Style Rules** — Does CLAUDE.md itself follow its own markdown rules?

#### 2. Command coverage

Read all `.claude/commands/*.md` files and verify:

- **Full-stack scope** — Does every command that should cover the frontend actually reference `frontend/src/`, `frontend/Cargo.toml`, or `frontend/tests/`?
- **Scope accuracy** — Are the files listed in each command's "Scope" section still the correct files to read?
- **Instruction currency** — Do instructions reference current patterns? (e.g., if a command says "check for `.unwrap()`" but the codebase now uses a custom error type everywhere, the instruction is stale)
- **Cross-command gaps** — Are there aspects of the project not covered by ANY command? (e.g., Dockerfile review, CI/CD, Makefile correctness, config file review)
- **Redundancy** — Are any two commands significantly overlapping in scope?

#### 3. Convention compliance

Spot-check that the codebase actually follows CLAUDE.md conventions:

- **Error handling** — Do all handlers use `Result<impl Responder, Error>`? Do DB functions use `.map_err(Error::Db)?`?
- **Instrumentation** — Do all handlers have `#[instrument(skip(state), level = "debug")]`?
- **Validation** — Is `validate(&json)?` called before DB operations?
- **Logging** — Do 4xx errors use `warn!()` and 5xx errors use `error!()`?
- **Frontend patterns** — Do components use `#[component]` and return `impl IntoView`? Are signals used correctly?
- **Test patterns** — Do integration tests use `#[ignore]` and `test_state()`? Do WASM tests follow the mock pattern?

#### 4. Settings review

Read `.claude/settings.json` and verify:

- Are the allowed bash commands still appropriate for the project?
- Are there commands the project now needs that aren't in the allow list? (e.g., `wasm-pack`, `trunk`, `cd frontend && ...`)
- Are there allowed commands that are no longer relevant?

### Output format

Provide:

1. **CLAUDE.md accuracy report:**

   | Section | Status | Issues Found |
   | ------- | ------ | ------------ |

2. **Command coverage matrix:**

   | Project Area | Commands Covering It | Gap? |
   | ------------ | -------------------- | ---- |

3. **Convention violations** — List each violation with file, line, and the convention it breaks

4. **Stale documentation** — Specific lines in CLAUDE.md or command files that need updating, with suggested corrections

5. **Missing command coverage** — Project areas not covered by any existing command

6. **Settings gaps** — Missing or stale entries in `.claude/settings.json`

7. **Action items** — Prioritized list of fixes, ranked by impact:
   - Critical: Factual errors in CLAUDE.md
   - High: Commands with wrong scope
   - Medium: Missing coverage areas
   - Low: Style or formatting issues

### Scope

Read `CLAUDE.md`, `.claude/settings.json`, all `.claude/commands/*.md`, `Cargo.toml`, `frontend/Cargo.toml`, `Makefile`, and spot-check source files under `src/` and `frontend/src/`. Do NOT modify any files — this is analysis only.
