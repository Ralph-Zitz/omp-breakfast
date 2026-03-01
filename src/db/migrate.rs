use refinery::embed_migrations;
use tracing::{info, warn};

embed_migrations!("migrations");

/// Ensure every `applied_on` value in `refinery_schema_history` is valid
/// RFC 3339 so that `refinery-core`'s strict `time` crate parser does not
/// panic on [`OffsetDateTime::parse(&applied_on, &Rfc3339).unwrap()`][1].
///
/// Known problematic formats that PostgreSQL or older `time` versions may
/// have produced:
///
/// | Stored value                        | Problem                              |
/// | ----------------------------------- | ------------------------------------ |
/// | `2025-01-15 12:30:00+00:00`         | Space separator instead of `T`       |
/// | `2025-01-15T12:30:00+00`            | Offset missing minutes               |
/// | `2025-01-15 12:30:00+00`            | Both of the above                    |
/// | `2025-01-15 12:30:00.123456+00`     | Both + microseconds                  |
/// | `2025-01-15T12:30:00.123456+0000`   | Compact offset (no colon)            |
/// | `2025-01-15 12:30:00.123456+00:00 ` | Trailing whitespace                  |
///
/// The function rewrites every non-conforming value into canonical RFC 3339
/// (`YYYY-MM-DDTHH:MM:SS[.f]+HH:MM` or `…Z`).  Rows that already parse
/// correctly are left untouched.
///
/// [1]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L19
async fn fix_migration_timestamps(
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

    let rows = client
        .query(
            "SELECT version, applied_on FROM refinery_schema_history ORDER BY version",
            &[],
        )
        .await?;

    for row in &rows {
        let version: i32 = row.get(0);
        let applied_on: String = row.get(1);

        // Quick validation: try the same parse that refinery-core will attempt.
        // `time` 0.3's Rfc3339 requires:
        //   - exactly 4-digit year
        //   - `T` (or any single char) separator between date and time
        //   - offset as `Z`, `+HH:MM`, or `-HH:MM`
        //
        // Rather than pulling in the `time` crate ourselves, we do a
        // lightweight structural check and rewrite when needed.
        if is_valid_rfc3339(&applied_on) {
            continue;
        }

        let fixed = match normalize_to_rfc3339(&applied_on) {
            Some(f) => f,
            None => {
                warn!(
                    version,
                    applied_on,
                    "Cannot normalize refinery_schema_history.applied_on to RFC 3339 — \
                     replacing with current time to unblock migrations"
                );
                // Last resort: replace with the current UTC time in a known-good format.
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.fZ")
                    .to_string()
            }
        };

        info!(
            version,
            from = applied_on.as_str(),
            to = fixed.as_str(),
            "Rewriting refinery_schema_history.applied_on to valid RFC 3339"
        );

        client
            .execute(
                "UPDATE refinery_schema_history SET applied_on = $1 WHERE version = $2",
                &[&fixed, &version],
            )
            .await?;
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
/// determine which migrations have already been applied. Migration SQL uses
/// `IF NOT EXISTS` / `OR REPLACE` for idempotency, so running against a
/// database that was already set up via `database.sql` (the dev/test reset
/// script) is safe — the statements succeed as no-ops and refinery records
/// them as applied.
///
/// Before invoking refinery, this function sanitizes any previously stored
/// `applied_on` timestamps in `refinery_schema_history` that are not valid
/// RFC 3339.  This works around a bug in `refinery-core` ≤ 0.9 where
/// [`OffsetDateTime::parse(…, &Rfc3339).unwrap()`][bug] panics when the
/// stored value doesn't match the `time` crate's strict parser (tightened
/// in `time` 0.3.37+).
///
/// [bug]: https://github.com/rust-db/refinery/blob/0.8.16/refinery_core/src/drivers/tokio_postgres.rs#L19
pub async fn run_migrations(
    client: &mut tokio_postgres::Client,
) -> Result<refinery::Report, refinery::Error> {
    // Fix any previously stored timestamps that would cause refinery to panic.
    if let Err(e) = fix_migration_timestamps(client).await {
        warn!(
            error = %e,
            "Failed to pre-check refinery_schema_history timestamps — \
             migration may still succeed if timestamps are already valid"
        );
    }

    migrations::runner().run_async(client).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
