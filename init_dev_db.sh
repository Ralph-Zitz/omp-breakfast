#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# Development database initialization script
#
# This script is used by the postgres-setup service in docker-compose.yml to:
# 1. Run the V1 migration SQL to create the schema (tables, indexes, triggers)
# 2. Run the V2 migration SQL to switch UUID defaults from v4 to v7
# 3. Run the V3 migration SQL to add indexes, fix FK actions, add NOT NULL
# 4. Run the V4 migration SQL for schema hardening (NOT NULL, timestamps)
# 5. Run the V5 migration SQL to fix users trigger and add NOT NULL
# 6. Run the V6 migration SQL for order constraint and covering index
# 7. Run the V7 migration SQL to drop redundant indexes
# 8. Run the V8 migration SQL for avatars table and users.avatar_id
# 9. Run the V9 migration SQL for avatar index and revoked_at NOT NULL
# 10. Create the refinery_schema_history table (empty — no rows inserted)
#
# Migrations V10–V12 are NOT run here because they contain non-idempotent
# DDL (e.g. ADD CONSTRAINT) that fails on re-run. Refinery handles them
# automatically on first application startup.
#
# On first start the application's refinery migration runner will detect
# that V1–V9 are "unapplied", re-run them (safe — the SQL is fully
# idempotent), and record them with correct checksums and timestamps.
# It will then apply V10–V12 for the first time.
# ═══════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

echo "==> Running V1 migration (creating schema)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V1__initial_schema.sql

echo "==> Running V2 migration (UUID v7 defaults)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V2__uuid_v7_defaults.sql

echo "==> Running V3 migration (indexes, constraints)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V3__indexes_constraints.sql

echo "==> Running V4 migration (schema hardening)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V4__schema_hardening.sql

echo "==> Running V5 migration (trigger + NOT NULL fixes)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V5__trigger_and_notnull_fixes.sql

echo "==> Running V6 migration (order constraint + index)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V6__order_constraint_and_index.sql

echo "==> Running V7 migration (drop redundant indexes)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V7__drop_redundant_indexes.sql

echo "==> Running V8 migration (avatars)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V8__avatars.sql

echo "==> Running V9 migration (avatar index + revoked_at NOT NULL)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V9__avatar_index_and_revoked_not_null.sql

echo "==> Creating refinery migration tracking table..."
# The table is created here so the app's migration runner sees it on first
# start.  We do NOT insert rows — refinery will detect that V1–V9 are
# "unapplied", re-run them (safe because the SQL is idempotent), and record
# them with correct checksums and timestamps.
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix <<-EOSQL
  CREATE TABLE IF NOT EXISTS refinery_schema_history (
    version INT4 PRIMARY KEY,
    name VARCHAR(255),
    applied_on VARCHAR(255),
    checksum VARCHAR(255)
  );
EOSQL

echo "==> Development database initialized successfully!"
