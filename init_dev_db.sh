#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════
# Test database initialization script
#
# Used by the postgres-setup service (defined in docker-compose.test.yml)
# to prepare the schema for integration tests. The script:
#
# 1. Resets the public schema (DROP + CREATE) so re-runs are safe even if
#    a prior test run left data behind (e.g. after Ctrl-C).
# 2. Auto-discovers and applies all V*__*.sql migration files in version
#    order. Adding a new migration file is all that is needed — no script
#    changes required.
#
# This script is NOT used by the application container in development or
# production. The breakfast app runs refinery migrations on startup
# (see src/db/migrate.rs), which provides checksum tracking, rollback
# detection, and proper migration history.
# ═══════════════════════════════════════════════════════════════════════════

set -e  # Exit on error

PGCMD="PGPASSWORD=actix psql -h postgres -p 5432 -U actix actix"

echo "==> Resetting public schema (clean slate)..."
eval "$PGCMD" <<-'EOSQL'
  DROP SCHEMA public CASCADE;
  CREATE SCHEMA public;
EOSQL

echo "==> Applying migrations..."
for f in $(ls /migrations/V*__*.sql | sort -t'V' -k2 -n); do
  name=$(basename "$f")
  echo "  -> $name"
  eval "$PGCMD" < "$f"
done

echo "==> Test database initialized successfully!"
