use refinery::embed_migrations;

embed_migrations!("migrations");

/// Run all pending database migrations.
///
/// Uses refinery's migration tracking (`refinery_schema_history` table) to
/// determine which migrations have already been applied. Migration SQL uses
/// `IF NOT EXISTS` / `OR REPLACE` for idempotency, so running against a
/// database that was already set up via `database.sql` (the dev/test reset
/// script) is safe — the statements succeed as no-ops and refinery records
/// them as applied.
pub async fn run_migrations(
    client: &mut tokio_postgres::Client,
) -> Result<refinery::Report, refinery::Error> {
    migrations::runner().run_async(client).await
}
