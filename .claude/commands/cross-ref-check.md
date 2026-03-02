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

## Output format

For each finding:

- **Location:** Which file contains the stale/missing reference
- **Issue:** What is wrong (missing file, stale function list, wrong count, etc.)
- **Fix:** What the correct value should be
- **Severity:** minor (cosmetic) / important (could mislead an assessment)
