# Database Review

Review all database queries, schema design, and data access patterns for correctness and performance.

## Instructions

You are a database engineer reviewing a PostgreSQL-backed Rust application. The database uses **Refinery migrations** for schema management.

### Database Architecture

The application uses different initialization strategies:

- **Schema:** `migrations/` directory — all Refinery migration files (V1 initial schema, V2 UUID v7 defaults, V3 indexes/constraints, and any newer migrations)
- **Seed data (dev/test only):** `database_seed.sql` - INSERT statements with ON CONFLICT DO NOTHING for test fixtures
- **Manual reset (deprecated):** `database.sql` - Full DROP/CREATE script, kept for manual dev resets only
- **Initialization:** `init_dev_db.sh` - Sets up refinery tracking table and loads seed data for docker-compose

**Production:** Application runs migrations at startup via `src/db/migrate.rs`  
**Development:** docker-compose runs `init_dev_db.sh` which marks V1 as applied and loads seeds

### Schema review (`migrations/`)

1. **Indexing** — Are all foreign keys indexed? Are frequently queried columns (used in WHERE clauses in `src/db/`) indexed? Are there missing or redundant indexes?
2. **Data types** — Are column types appropriate? (e.g., `money` type vs `numeric`, `text` vs `varchar` consistency)
3. **Constraints** — Are NOT NULL, UNIQUE, CHECK constraints sufficient? Are there missing constraints that could prevent data corruption?
4. **Cascading** — Review ON DELETE behavior for all foreign keys. Are there cases where CASCADE could cause unintended data loss?
5. **Defaults** — Are defaults appropriate? Is `uuid_generate_v4()` the best choice vs application-generated UUIDs (the app uses `uuid::Uuid::now_v7()`)?
6. **Triggers** — Is the `update_users_timestamp` trigger correct? Should similar triggers exist for other tables?

### Query review (`src/db/`)

1. **Prepared statements** — Are all queries using parameterized prepared statements?
2. **N+1 queries** — Are there patterns where multiple queries could be combined into one?
3. **Error mapping** — Are DB errors correctly mapped to application errors?
4. **Connection usage** — Is the connection pool used efficiently? Are connections held for too long?
5. **Missing queries** — Based on the schema, what query functions are missing?
6. **SELECT * usage** — Are queries selecting more columns than needed?
7. **Transaction safety** — Are multi-step operations (e.g., create user + hash password) wrapped in transactions when they should be?

### Output format

For each finding:

- **Location:** File and line(s), or table/column name
- **Severity:** Critical / Warning / Suggestion
- **Description:** The issue
- **Recommendation:** How to fix it, with SQL or Rust code

End with:

1. **Schema improvement script** — SQL ALTER statements for recommended changes
2. **Query optimization list** — Prioritized changes to `src/db/`
3. **Missing functionality** — DB functions needed for incomplete features

### Scope

Read the following files:
- `migrations/` - All migration files (V1 schema, V2 UUID defaults, V3 indexes/constraints, and any newer)
- `database_seed.sql` - Seed data for development/testing
- `src/db/` - All database query modules
- `src/db/migrate.rs` - Migration runner

Reference `src/models.rs` for struct-to-table alignment. The `database.sql` file is deprecated and should not be used as the source of truth for schema review. Do NOT modify any files — this is analysis only.
