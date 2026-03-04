# Cross-Reference Check

Validate that `CLAUDE.md`, all files in `.claude/commands/`, and migration references are consistent with the actual files on disk.

## Instructions

You are a documentation auditor. Cross-reference every file path, function name, test count, and structural reference in the project documentation against the real codebase. Do NOT modify any files — this is analysis only.

### Checks to perform

1. **CLAUDE.md Project Structure tree vs disk**
   - Every file and directory listed in the `Project Structure` code block must exist on disk
   - Every source file on disk under `src/`, `frontend/src/`, `config/`, `migrations/`, and `tests/` must appear in the tree
   - File descriptions (function lists, purpose) must match the actual contents

2. **Migration file coverage**
   - List all files in `migrations/` on disk
   - Verify every migration file is listed in the CLAUDE.md Project Structure tree
   - Verify every `.claude/commands/` file that references migrations includes **all** migration files (not just V1)
   - Flag any command that hardcodes a specific migration filename instead of referencing the `migrations/` directory generically

3. **Function inventories**
   - For each `db/*.rs` module listed in CLAUDE.md with a parenthetical function list, verify every `pub async fn` in that file is included
   - For `handlers/mod.rs`, verify the listed RBAC helper functions match the actual `pub async fn` signatures

4. **Test counts**
   - Run `cargo test --lib 2>&1 | grep "test result"` and compare the count against the number stated in the Testing section of CLAUDE.md
   - Flag if the count has drifted

5. **Command file references**
   - For each file in `.claude/commands/`, extract every file path referenced (e.g., `src/db/`, `database.sql`, `migrations/V1__initial_schema.sql`)
   - Verify each referenced path exists on disk
   - Flag references to deprecated files (e.g., `database.sql` used as a source of truth instead of `migrations/`)
   - Verify role string references match the current approach (constants vs hardcoded strings)

6. **Assessment command list**
   - Verify that every `.md` file in `.claude/commands/` (excluding `resume-assessment.md`) is listed in the Project Assessment section of CLAUDE.md
   - Flag any command file that exists on disk but is missing from the assessment list, or vice versa

7. **README.md accuracy**
   - Compare test counts in README.md against actual `cargo test`, `make test-integration`, and `make test-frontend` output — flag any drift
   - Verify the migration table lists every file in `migrations/` on disk (same check as step 2 but for README.md)
   - Verify the Make Targets table includes every non-internal target defined in `Makefile`
   - Verify the Prerequisites list matches the current build requirements (e.g., Trunk, wasm-pack, Docker, mkcert)
   - Verify the Tech Stack section reflects the current dependencies (framework versions, database version, auth approach)
   - Verify the Configuration table lists all environment variable prefixes and key variables actually used in `config/default.yml`
   - Verify the Setup instructions still work with the current `docker-compose.yml` service names and commands
   - **Tone:** README.md is aimed at a broader audience (new contributors, external developers). Fixes should keep language approachable and avoid internal jargon. Do not add implementation details that belong only in CLAUDE.md.

## Output format

For each finding:

- **Location:** Which file contains the stale/missing reference
- **Issue:** What is wrong (missing file, stale function list, wrong count, etc.)
- **Fix:** What the correct value should be
- **Severity:** minor (cosmetic) / important (could mislead an assessment)
