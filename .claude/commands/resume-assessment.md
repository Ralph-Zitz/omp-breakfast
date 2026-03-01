# Resume Assessment

Pick up where the last project assessment left off.

## Context

The file `.claude/assessment-findings.md` is produced and maintained by the **project assessment process** defined in `CLAUDE.md` § "Project Assessment". Each time `assess the project` is run, critical and important findings are written there. This command reads that file to continue work in a new session.

## Instructions

Read `.claude/assessment-findings.md` and present the current status of all tracked items.

### Steps

1. **Load findings** — Read `.claude/assessment-findings.md` in full
2. **Identify open items** — List every item still marked `[ ]` (unchecked)
3. **Identify completed items** — List every item marked `[x]` or moved to the "Completed Items" section
4. **Verify completed items** — For each completed item, spot-check the referenced file and line range to confirm the fix is actually in place. If the fix is missing or incomplete, flag it and move it back to open.
5. **Summarize** — Present a table:

   | # | Title | Status | File | Notes |
   | - | ----- | ------ | ---- | ----- |

6. **Recommend next action** — Suggest which open item to tackle first based on:
   - Severity and risk (Important > Minor)
   - Dependency order (e.g., fix error types before writing tests that assert on them)
   - Effort (quick wins first when severities are equal)

### After presenting the summary

Ask the user which item(s) they want to work on. When they choose, read the relevant source files and implement the fix described in the findings file. After each fix:

1. Run `cargo test` (and `make test-integration` or `make test-frontend` if applicable) to verify nothing broke
2. Update `.claude/assessment-findings.md` — change `[ ]` to `[x]` and move the item to the "Completed Items" section with a one-line note on what was done
3. Ask if the user wants to continue to the next item

### Scope

Read `.claude/assessment-findings.md` and the source files referenced by each finding. Modify source files only when the user approves a fix. Always update the findings file after completing an item.