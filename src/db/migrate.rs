use refinery::embed_migrations;
use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};
use tracing::{info, warn};

embed_migrations!("migrations");

/// Compute the checksum for a migration the same way `refinery-core` does:
/// `SipHasher13::new()` seeded with `(name, version, sql)`.
///
/// `name` is the migration name without the prefix/version (e.g. `"initial_schema"`
/// for `V1__initial_schema.sql`).  `version` is the integer version number.
fn compute_checksum(name: &str, version: i32, sql: &str) -> u64 {
    let mut hasher = SipHasher13::new();
    name.hash(&mut hasher);
    version.hash(&mut hasher);
    sql.hash(&mut hasher);
    hasher.finish()
}

/// Sanitize the `refinery_schema_history` table so that `refinery-core`'s
/// strict parsers do not panic on `.unwrap()` calls.
///
/// This fixes two known bugs in `refinery-core` ≤ 0.9:
///
/// 1. **`applied_on` timestamps** — [`OffsetDateTime::parse(&applied_on, &Rfc3339).unwrap()`][ts]
///    panics when the stored value is not valid RFC 3339 (the `time` crate
///    tightened its parser in 0.3.37+).
///
/// 2. **`checksum` values** — [`checksum.parse::<u64>().expect(…)`][ck] panics
///    when the stored value is not a valid `u64` (e.g. the placeholder
///    `"unused"` inserted by `init_dev_db.sh`).
///
/// For checksums, the correct value is recomputed using the same
/// `SipHasher13(name, version, sql)` algorithm that refinery uses internally.
/// The embedded migration SQL is matched by version number.
///
/// [ts]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L19
/// [ck]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L28
async fn fix_migration_history(
    client: &tokio_postgres::Client,
) -> Result<(), tokio_postgres::Error> {
    // If the table does not exist yet (first run), there is nothing to fix.
    let exists = client
        .query_one(
            "SELECT EXISTS (
                 SELECT 1 FROM information_schema.tables
                 WHERE table_name = 'refinery_schema_history'
             )",
            &[],
        )
        .await?;

    let table_exists: bool = exists.get(0);
    if !table_exists {
        return Ok(());
    }

    // Build a lookup of embedded migrations: version → (name, sql).
    // `migrations::runner().get_migrations()` returns the compiled-in list.
    let embedded = migrations::runner().get_migrations().to_vec();

    let rows = client
        .query(
            "SELECT version, name, applied_on, checksum \
             FROM refinery_schema_history ORDER BY version",
            &[],
        )
        .await?;

    for row in &rows {
        let version: i32 = row.get(0);
        let name: String = row.get(1);
        let applied_on: String = row.get(2);
        let checksum: String = row.get(3);

        let mut needs_update = false;

        // ── Fix applied_on ──────────────────────────────────────────
        // `time` 0.3's Rfc3339 requires:
        //   - exactly 4-digit year
        //   - `T` (or any single char) separator between date and time
        //   - offset as `Z`, `+HH:MM`, or `-HH:MM`
        let fixed_applied_on = if is_valid_rfc3339(&applied_on) {
            applied_on.clone()
        } else {
            needs_update = true;
            match normalize_to_rfc3339(&applied_on) {
                Some(f) => {
                    info!(
                        version,
                        from = applied_on.as_str(),
                        to = f.as_str(),
                        "Rewriting refinery_schema_history.applied_on to valid RFC 3339"
                    );
                    f
                }
                None => {
                    let fallback = chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%S%.fZ")
                        .to_string();
                    warn!(
                        version,
                        applied_on,
                        to = fallback.as_str(),
                        "Cannot normalize applied_on to RFC 3339 — using current time"
                    );
                    fallback
                }
            }
        };

        // ── Fix checksum ────────────────────────────────────────────
        // refinery-core does `checksum.parse::<u64>().expect(…)` which
        // panics on non-numeric values like "unused".
        let fixed_checksum = if checksum.parse::<u64>().is_ok() {
            checksum.clone()
        } else {
            needs_update = true;

            // Try to find the embedded migration with a matching version so
            // we can recompute the correct checksum.
            let recomputed = embedded.iter().find(|m| m.version() == version).map(|m| {
                let correct = compute_checksum(m.name(), version, m.sql().unwrap_or(""));
                info!(
                    version,
                    name = name.as_str(),
                    from = checksum.as_str(),
                    to = correct,
                    "Rewriting refinery_schema_history.checksum to valid u64"
                );
                correct.to_string()
            });

            match recomputed {
                Some(c) => c,
                None => {
                    // Migration is not in the embedded set (e.g. it was removed
                    // from the filesystem).  Use a deterministic placeholder
                    // that is at least a valid u64 so refinery won't panic.
                    let fallback = compute_checksum(&name, version, "");
                    warn!(
                        version,
                        name = name.as_str(),
                        from = checksum.as_str(),
                        to = fallback,
                        "Embedded migration not found — using fallback checksum"
                    );
                    fallback.to_string()
                }
            }
        };

        if needs_update {
            client
                .execute(
                    "UPDATE refinery_schema_history \
                     SET applied_on = $1, checksum = $2 \
                     WHERE version = $3",
                    &[&fixed_applied_on, &fixed_checksum, &version],
                )
                .await?;
        }
    }

    Ok(())
}

/// Minimal structural check for RFC 3339 compliance as understood by the
/// `time` crate's parser (version 0.3.37+).
///
/// Accepted patterns:
///   `YYYY-MM-DDTHH:MM:SS[.fractional]Z`
///   `YYYY-MM-DDTHH:MM:SS[.fractional]+HH:MM`
///   `YYYY-MM-DDTHH:MM:SS[.fractional]-HH:MM`
///
/// The `time` crate actually accepts any single byte as the date/time
/// separator, but we only accept `T` and `t` here because that is what a
/// correct formatter would produce.  Everything else gets normalized.
fn is_valid_rfc3339(s: &str) -> bool {
    let s = s.trim();
    let b = s.as_bytes();

    // Minimum: `YYYY-MM-DDTHH:MM:SSZ` = 20 chars
    if b.len() < 20 {
        return false;
    }

    // Year: exactly 4 digits
    if !b[0..4].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Date separators
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }

    // Month and day: 2 digits each
    if !b[5..7].iter().all(|c| c.is_ascii_digit()) || !b[8..10].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Separator between date and time must be a single char (T or t)
    if b[10] != b'T' && b[10] != b't' {
        return false;
    }

    // Time: HH:MM:SS
    if !b[11..13].iter().all(|c| c.is_ascii_digit())
        || b[13] != b':'
        || !b[14..16].iter().all(|c| c.is_ascii_digit())
        || b[16] != b':'
        || !b[17..19].iter().all(|c| c.is_ascii_digit())
    {
        return false;
    }

    // After seconds: optional fractional part, then offset
    let rest = &s[19..];
    let offset_start = if let Some(stripped) = rest.strip_prefix('.') {
        // Must have at least one digit after the dot
        let frac_digits = stripped.bytes().take_while(|c| c.is_ascii_digit()).count();
        if frac_digits == 0 {
            return false;
        }
        19 + 1 + frac_digits // skip '.' + digits
    } else {
        19
    };

    let offset = s[offset_start..].trim_end();

    // Valid offsets: Z, z, +HH:MM, -HH:MM
    if offset.eq_ignore_ascii_case("Z") {
        return true;
    }

    let ob = offset.as_bytes();
    // +HH:MM or -HH:MM = 6 chars
    if ob.len() != 6 {
        return false;
    }
    if ob[0] != b'+' && ob[0] != b'-' {
        return false;
    }
    if !ob[1..3].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if ob[3] != b':' {
        return false;
    }
    if !ob[4..6].iter().all(|c| c.is_ascii_digit()) {
        return false;
    }

    true
}

/// Attempt to normalize a non-RFC-3339 timestamp string into valid RFC 3339.
///
/// Returns `None` if the input is too mangled to salvage.
fn normalize_to_rfc3339(s: &str) -> Option<String> {
    let s = s.trim();

    // Must have at least `YYYY-MM-DD HH:MM:SS` = 19 chars
    if s.len() < 19 {
        return None;
    }

    // Validate date portion: YYYY-MM-DD
    let date_part = &s[0..10];
    let db = date_part.as_bytes();
    if !db[0..4].iter().all(|c| c.is_ascii_digit())
        || db[4] != b'-'
        || !db[5..7].iter().all(|c| c.is_ascii_digit())
        || db[7] != b'-'
        || !db[8..10].iter().all(|c| c.is_ascii_digit())
    {
        return None;
    }

    // Expect some separator at position 10 (T, t, or space)
    let sep = s.as_bytes()[10];
    if sep != b'T' && sep != b't' && sep != b' ' {
        return None;
    }

    // Validate time portion: HH:MM:SS
    let time_part = &s[11..19];
    let tb = time_part.as_bytes();
    if !tb[0..2].iter().all(|c| c.is_ascii_digit())
        || tb[2] != b':'
        || !tb[3..5].iter().all(|c| c.is_ascii_digit())
        || tb[5] != b':'
        || !tb[6..8].iter().all(|c| c.is_ascii_digit())
    {
        return None;
    }

    // Build the canonical form
    let mut result = format!("{}T{}", date_part, time_part);

    // Fractional seconds
    let rest = &s[19..];
    let after_frac = if let Some(stripped) = rest.strip_prefix('.') {
        let frac_digits = stripped.bytes().take_while(|c| c.is_ascii_digit()).count();
        if frac_digits > 0 {
            result.push('.');
            result.push_str(&stripped[..frac_digits]);
        }
        &stripped[frac_digits..]
    } else {
        rest
    };

    let offset_str = after_frac.trim();

    // If no offset at all, assume UTC
    if offset_str.is_empty() {
        result.push('Z');
        return Some(result);
    }

    // Already Z/z
    if offset_str.eq_ignore_ascii_case("Z") {
        result.push('Z');
        return Some(result);
    }

    // Must start with + or -
    let ob = offset_str.as_bytes();
    if ob[0] != b'+' && ob[0] != b'-' {
        return None;
    }

    let sign = ob[0] as char;
    let digits_part = &offset_str[1..];

    // Possible offset formats after the sign:
    //   HH:MM  (5 chars) — already good
    //   HH     (2 chars) — missing minutes
    //   HHMM   (4 chars) — compact, no colon
    //   HH:MM:SS (8 chars) — has seconds, strip them (time crate doesn't accept offset seconds in RFC 3339 offsets in all versions)
    let normalized_offset = match digits_part.len() {
        5 if digits_part.as_bytes()[2] == b':' => {
            // +HH:MM — already canonical
            format!("{}{}", sign, digits_part)
        }
        2 => {
            // +HH — append :00
            if !digits_part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            format!("{}{}:00", sign, digits_part)
        }
        4 => {
            // +HHMM — insert colon
            if !digits_part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            format!("{}{}:{}", sign, &digits_part[..2], &digits_part[2..])
        }
        8 if digits_part.as_bytes()[2] == b':' && digits_part.as_bytes()[5] == b':' => {
            // +HH:MM:SS — drop seconds
            format!("{}{}", sign, &digits_part[..5])
        }
        _ => return None,
    };

    result.push_str(&normalized_offset);
    Some(result)
}

/// Run all pending database migrations.
///
/// Uses refinery's migration tracking (`refinery_schema_history` table) to
/// determine which migrations have already been applied.
///
/// Before invoking refinery, this function sanitizes any previously stored
/// rows in `refinery_schema_history` that would cause `refinery-core` to
/// panic.  This works around two bugs in `refinery-core` ≤ 0.9:
///
/// 1. [`OffsetDateTime::parse(…, &Rfc3339).unwrap()`][ts] panics on
///    non-RFC-3339 `applied_on` values (the `time` crate tightened its
///    parser in 0.3.37+).
/// 2. [`checksum.parse::<u64>().expect(…)`][ck] panics on non-numeric
///    `checksum` values (e.g. placeholder `"unused"`).
///
/// [ts]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L19
/// [ck]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L28
pub async fn run_migrations(
    client: &mut tokio_postgres::Client,
) -> Result<refinery::Report, refinery::Error> {
    // Fix any previously stored values that would cause refinery to panic.
    if let Err(e) = fix_migration_history(client).await {
        warn!(
            error = %e,
            "Failed to pre-check refinery_schema_history — \
             migration may still succeed if stored values are already valid"
        );
    }

    migrations::runner().run_async(client).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- compute_checksum ----

    #[test]
    fn checksum_is_deterministic() {
        let a = compute_checksum("initial_schema", 1, "CREATE TABLE foo;");
        let b = compute_checksum("initial_schema", 1, "CREATE TABLE foo;");
        assert_eq!(a, b);
    }

    #[test]
    fn checksum_differs_for_different_sql() {
        let a = compute_checksum("initial_schema", 1, "CREATE TABLE foo;");
        let b = compute_checksum("initial_schema", 1, "CREATE TABLE bar;");
        assert_ne!(a, b);
    }

    #[test]
    fn checksum_differs_for_different_version() {
        let a = compute_checksum("initial_schema", 1, "CREATE TABLE foo;");
        let b = compute_checksum("initial_schema", 2, "CREATE TABLE foo;");
        assert_ne!(a, b);
    }

    #[test]
    fn checksum_differs_for_different_name() {
        let a = compute_checksum("initial_schema", 1, "CREATE TABLE foo;");
        let b = compute_checksum("add_index", 1, "CREATE TABLE foo;");
        assert_ne!(a, b);
    }

    // ---- is_valid_rfc3339 ----

    #[test]
    fn valid_rfc3339_utc_z() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00Z"));
    }

    #[test]
    fn valid_rfc3339_utc_lowercase_z() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00z"));
    }

    #[test]
    fn valid_rfc3339_with_offset() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00+00:00"));
    }

    #[test]
    fn valid_rfc3339_negative_offset() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00-05:00"));
    }

    #[test]
    fn valid_rfc3339_with_fractional_seconds() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00.123456Z"));
    }

    #[test]
    fn valid_rfc3339_with_fractional_and_offset() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00.123456+02:00"));
    }

    #[test]
    fn valid_rfc3339_nine_digit_fractional() {
        assert!(is_valid_rfc3339("2025-01-15T12:30:00.123456789Z"));
    }

    #[test]
    fn valid_rfc3339_with_trailing_whitespace() {
        // Trimmed before validation
        assert!(is_valid_rfc3339("2025-01-15T12:30:00Z "));
    }

    #[test]
    fn invalid_space_separator() {
        assert!(!is_valid_rfc3339("2025-01-15 12:30:00Z"));
    }

    #[test]
    fn invalid_offset_missing_minutes() {
        assert!(!is_valid_rfc3339("2025-01-15T12:30:00+00"));
    }

    #[test]
    fn invalid_compact_offset() {
        assert!(!is_valid_rfc3339("2025-01-15T12:30:00+0000"));
    }

    #[test]
    fn invalid_no_offset() {
        assert!(!is_valid_rfc3339("2025-01-15T12:30:00"));
    }

    #[test]
    fn invalid_too_short() {
        assert!(!is_valid_rfc3339("2025-01-15"));
    }

    #[test]
    fn invalid_empty() {
        assert!(!is_valid_rfc3339(""));
    }

    #[test]
    fn invalid_offset_with_seconds() {
        assert!(!is_valid_rfc3339("2025-01-15T12:30:00+00:00:00"));
    }

    // ---- normalize_to_rfc3339 ----

    #[test]
    fn normalize_space_separator() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00+00:00"),
            Some("2025-01-15T12:30:00+00:00".to_string())
        );
    }

    #[test]
    fn normalize_space_and_short_offset() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00+00"),
            Some("2025-01-15T12:30:00+00:00".to_string())
        );
    }

    #[test]
    fn normalize_space_separator_z() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00Z"),
            Some("2025-01-15T12:30:00Z".to_string())
        );
    }

    #[test]
    fn normalize_compact_offset() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15T12:30:00+0530"),
            Some("2025-01-15T12:30:00+05:30".to_string())
        );
    }

    #[test]
    fn normalize_space_and_fractional_and_short_offset() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00.123456+00"),
            Some("2025-01-15T12:30:00.123456+00:00".to_string())
        );
    }

    #[test]
    fn normalize_offset_with_seconds() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15T12:30:00+05:30:00"),
            Some("2025-01-15T12:30:00+05:30".to_string())
        );
    }

    #[test]
    fn normalize_no_offset_assumes_utc() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00"),
            Some("2025-01-15T12:30:00Z".to_string())
        );
    }

    #[test]
    fn normalize_trailing_whitespace() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00.123+00:00 "),
            Some("2025-01-15T12:30:00.123+00:00".to_string())
        );
    }

    #[test]
    fn normalize_already_valid_is_canonical() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15T12:30:00Z"),
            Some("2025-01-15T12:30:00Z".to_string())
        );
    }

    #[test]
    fn normalize_already_valid_with_offset() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15T12:30:00.999+02:00"),
            Some("2025-01-15T12:30:00.999+02:00".to_string())
        );
    }

    #[test]
    fn normalize_negative_short_offset() {
        assert_eq!(
            normalize_to_rfc3339("2025-01-15 12:30:00-05"),
            Some("2025-01-15T12:30:00-05:00".to_string())
        );
    }

    #[test]
    fn normalize_garbage_returns_none() {
        assert_eq!(normalize_to_rfc3339("not-a-timestamp"), None);
    }

    #[test]
    fn normalize_too_short_returns_none() {
        assert_eq!(normalize_to_rfc3339("2025"), None);
    }

    #[test]
    fn normalize_empty_returns_none() {
        assert_eq!(normalize_to_rfc3339(""), None);
    }

    #[test]
    fn normalized_value_passes_validation() {
        let problematic = [
            "2025-01-15 12:30:00+00:00",
            "2025-01-15 12:30:00+00",
            "2025-01-15 12:30:00.123456+00",
            "2025-01-15T12:30:00+0000",
            "2025-01-15 12:30:00",
            "2025-01-15 12:30:00.123456+00:00 ",
            "2025-01-15T12:30:00+05:30:00",
        ];
        for input in &problematic {
            let normalized = normalize_to_rfc3339(input)
                .unwrap_or_else(|| panic!("failed to normalize: {}", input));
            assert!(
                is_valid_rfc3339(&normalized),
                "normalized value '{}' (from '{}') should be valid RFC 3339",
                normalized,
                input
            );
        }
    }
}
