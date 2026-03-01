#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# Development database initialization script
#
# This script is used by the postgres-setup service in docker-compose.yml to:
# 1. Run the V1 migration SQL to create the schema (tables, indexes, triggers)
# 2. Create the refinery_schema_history table if it doesn't exist
# 3. Mark V1__initial_schema as applied (so the app doesn't try to re-run it)
# 4. Load seed data for development/testing
#
# The application's migration runner will see the schema is already at V1 and
# skip the migration, avoiding conflicts with tables that were already created.
# ═══════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

echo "==> Running V1 migration (creating schema)..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /migrations/V1__initial_schema.sql

echo "==> Creating refinery migration tracking table..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix <<-EOSQL
  -- Create refinery's migration tracking table
  CREATE TABLE IF NOT EXISTS refinery_schema_history (
    version INT4 PRIMARY KEY,
    name VARCHAR(255),
    applied_on VARCHAR(255),
    checksum VARCHAR(255)
  );

  -- Mark V1__initial_schema as applied (so the app skips it at startup)
  INSERT INTO refinery_schema_history (version, name, applied_on, checksum)
  VALUES (1, 'V1__initial_schema', 'manual', 'unused')
  ON CONFLICT (version) DO NOTHING;
EOSQL

echo "==> Loading seed data..."
PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix < /database_seed.sql

echo "==> Development database initialized successfully!"
