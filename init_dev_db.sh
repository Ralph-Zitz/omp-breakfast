#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# Development database initialization script
#
# This script is used by the postgres-setup service in docker-compose.yml to:
# 1. Run the V1 migration SQL to create the schema (tables, indexes, triggers)
# 2. Run the V2 migration SQL to switch UUID defaults from v4 to v7
# 3. Create the refinery_schema_history table (empty — no rows inserted)
# 4. Load seed data for development/testing
#
# On first start the application's refinery migration runner will detect
# that V1 and V2 are "unapplied", re-run them (safe — the SQL is fully
# idempotent), and record them with correct checksums and timestamps.
# ═══════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

echo "==> Running V1 migration (creating schema)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V1__initial_schema.sql

echo "==> Running V2 migration (UUID v7 defaults)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V2__uuid_v7_defaults.sql

echo "==> Creating refinery migration tracking table..."
# The table is created here so the app's migration runner sees it on first
# start.  We do NOT insert rows — refinery will detect that V1 and V2 are
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

echo "==> Loading seed data..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /database_seed.sql

echo "==> Development database initialized successfully!"
