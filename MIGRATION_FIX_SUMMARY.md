# Database Migration Conflict Resolution - Summary

## Problem
The application had two competing database initialization mechanisms causing migration failures:
1. **Refinery migrations** (`migrations/V1__initial_schema.sql`) - ran by the Rust app at startup
2. **Direct SQL script** (`database.sql`) - ran by docker-compose's `postgres-setup` service

When `docker compose up` ran, `postgres-setup` would execute `database.sql` (which drops and recreates all tables), then the application would try to run migrations and fail because:
- Tables already existed but refinery's tracking table was missing or inconsistent
- Triggers and functions had definition conflicts

## Solution Implemented

### 1. Separated Concerns
Created three distinct files with clear purposes:

- **`migrations/V1__initial_schema.sql`** (184 lines)
  - Idempotent DDL using `CREATE IF NOT EXISTS` and `OR REPLACE`
  - No seed data
  - Used by Refinery migration runner in production and development

- **`database_seed.sql`** (NEW - 220 lines)
  - Seed data only (INSERT statements)
  - Uses `ON CONFLICT DO NOTHING` for idempotency
  - Used only in development/testing via docker-compose

- **`database.sql`** (373 lines - DEPRECATED)
  - Kept for manual database resets only
  - No longer used by docker-compose
  - Updated with deprecation warning

### 2. Created Initialization Script
**`init_dev_db.sh`** (NEW):
- Creates `refinery_schema_history` table
- Marks V1 migration as already applied
- Loads seed data from `database_seed.sql`
- Prevents migration conflicts

### 3. Updated Docker Compose
**`docker-compose.yml`** changes:
- `postgres-setup` service now runs `init_dev_db.sh` instead of `database.sql`
- Mounts both `database_seed.sql` and `init_dev_db.sh`

### 4. Updated Documentation
- **`README.md`**: Added "Database Initialization" section explaining different workflows
- **`.claude/commands/db-review.md`**: Updated to reference migration architecture

## Workflow Changes

### Development (docker-compose)
```bash
docker compose up -d
```
**What happens:**
1. `postgres` container starts
2. `postgres-setup` runs `init_dev_db.sh`:
   - Creates refinery tracking table
   - Marks V1 as applied
   - Loads seed data
3. `breakfast` app starts
4. App's migration runner sees V1 already applied → skips migration ✅
5. App starts successfully with seed data

### Production
```bash
# App starts and runs migrations automatically
cargo run --release
```
**What happens:**
1. App connects to database
2. Runs `migrations/V1__initial_schema.sql` via Refinery
3. Refinery tracks migration in `refinery_schema_history`
4. No seed data is loaded
5. App starts successfully

### Manual Database Reset (Development)
```bash
# Complete reset if needed
docker compose down -v
PGPASSWORD=actix psql -h localhost -p 5432 -U actix actix < database.sql
# Or just restart docker-compose
docker compose up -d
```

## Benefits

1. **No more conflicts**: Migration tracking is properly initialized before app starts
2. **Separation of concerns**: Schema evolution vs test fixtures are distinct
3. **Production-ready**: Refinery migrations work correctly in all environments
4. **Idempotent**: Seed data can be re-run without errors
5. **Backwards compatible**: Existing test workflows continue to work
6. **Clear documentation**: Each file's purpose is explicit

## Files Created
- ✅ `database_seed.sql` - Seed data with ON CONFLICT clauses
- ✅ `init_dev_db.sh` - Development database initialization script

## Files Modified
- ✅ `docker-compose.yml` - Updated postgres-setup service
- ✅ `database.sql` - Added deprecation notice
- ✅ `README.md` - Added Database Initialization section
- ✅ `.claude/commands/db-review.md` - Updated for migration architecture

## Testing
To verify the fix works:
```bash
# Clean slate
docker compose down -v

# Start everything
docker compose up -d

# Check logs - should see no migration errors
docker compose logs breakfast

# Should see:
# "Database schema is up to date (no pending migrations)"
# OR "Applied database migrations" with applied = 0 or 1
```
