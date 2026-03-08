# Cross-Reference Check

Validate that `CLAUDE.md`, all files in `.claude/commands/`, and migration references are consistent with the actual files on disk.

## Instructions

You are a documentation auditor. Cross-reference every file path, function name, and structural reference in the project documentation against the real codebase. Do NOT modify any files — this is analysis only.

**Note:** Test counts and individual migration version numbers are intentionally NOT tracked in documentation files (CLAUDE.md, README.md, commands) to prevent drift. Do NOT flag missing counts or version-specific migration references as findings.

### Checks to perform

1. **CLAUDE.md Project Structure tree vs disk**
   - Every file and directory listed in the `Project Structure` code block must exist on disk
   - Every source file on disk under `src/`, `frontend/src/`, `config/`, `migrations/`, and `tests/` must appear in the tree
   - File descriptions (function lists, purpose) must match the actual contents

2. **Migration file coverage**
   - List all files in `migrations/` on disk
   - Verify CLAUDE.md references `migrations/` generically (not individual files)
   - Flag any documentation or command file that hardcodes a specific migration count or enumerates individual migration versions (e.g., "V1–V17", "seventeen migrations")

3. **Function inventories**
   - For each `db/*.rs` module listed in CLAUDE.md with a parenthetical function list, verify every `pub async fn` in that file is included
   - For `handlers/mod.rs`, verify the listed RBAC helper functions match the actual `pub async fn` signatures

4. **Test counts**
   - Verify that CLAUDE.md, README.md, and command files do NOT contain specific test counts (these drift and are intentionally omitted)
   - Flag any file that hardcodes a test count (e.g., "248 unit tests", "97 WASM tests")

5. **Command file references**
   - For each file in `.claude/commands/`, extract every file path referenced (e.g., `src/db/`, `migrations/`)
   - Verify each referenced path exists on disk
   - Flag references to deprecated files (e.g., `database.sql` used as a source of truth instead of `migrations/`)
   - Flag any command that enumerates specific migration versions instead of referencing `migrations/` generically

6. **Assessment command list**
   - Verify that every `.md` file in `.claude/commands/` (excluding `resume-assessment.md`) is listed in the Project Assessment section of CLAUDE.md
   - Flag any command file that exists on disk but is missing from the assessment list, or vice versa

7. **README.md accuracy**
   - Verify that README.md does NOT contain specific test counts or individual migration version numbers (these are intentionally omitted to prevent drift)
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
