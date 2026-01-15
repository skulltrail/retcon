use crate::error::{HistError, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};

/// Validate an email address format
pub fn validate_email(email: &str) -> Result<()> {
    // Basic email validation - must have @ with something on both sides
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(HistError::InvalidEmail(email.to_string()));
    }

    // Domain must have at least one dot (but not at start/end)
    let domain = parts[1];
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return Err(HistError::InvalidEmail(email.to_string()));
    }

    // No spaces allowed
    if email.contains(' ') {
        return Err(HistError::InvalidEmail(email.to_string()));
    }

    Ok(())
}

/// Parse and validate a date string
/// Accepts formats:
/// - "2024-01-15 14:30:00 +0000" (full with timezone)
/// - "2024-01-15 14:30:00" (assumes UTC)
/// - "2024-01-15 14:30" (assumes UTC, 0 seconds)
/// - "2024-01-15" (assumes midnight UTC)
pub fn validate_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
    let date_str = date_str.trim();

    // Try full format with timezone: "2024-01-15 14:30:00 +0000"
    if let Ok(dt) = DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S %z") {
        return Ok(dt);
    }

    // Try format with timezone offset like "+0530" or "-0800"
    if let Ok(dt) = DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S%z") {
        return Ok(dt);
    }

    // Try without timezone (assume UTC)
    if let Ok(naive) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        let utc = FixedOffset::east_opt(0).unwrap();
        return Ok(utc.from_local_datetime(&naive).unwrap());
    }

    // Try without seconds
    if let Ok(naive) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M") {
        let utc = FixedOffset::east_opt(0).unwrap();
        return Ok(utc.from_local_datetime(&naive).unwrap());
    }

    // Try date only (midnight UTC)
    if let Ok(naive) =
        NaiveDateTime::parse_from_str(&format!("{} 00:00:00", date_str), "%Y-%m-%d %H:%M:%S")
    {
        let utc = FixedOffset::east_opt(0).unwrap();
        return Ok(utc.from_local_datetime(&naive).unwrap());
    }

    Err(HistError::InvalidDate(date_str.to_string()))
}

/// Format a date for editing (reversible format)
#[allow(dead_code)]
pub fn format_date_for_edit(dt: &DateTime<FixedOffset>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_valid_emails() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name@example.co.uk").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());
    }

    #[test]
    fn test_invalid_emails() {
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@example").is_err());
        assert!(validate_email("user @example.com").is_err());
    }

    #[test]
    fn test_valid_dates() {
        assert!(validate_date("2024-01-15 14:30:00 +0000").is_ok());
        assert!(validate_date("2024-01-15 14:30:00").is_ok());
        assert!(validate_date("2024-01-15 14:30").is_ok());
        assert!(validate_date("2024-01-15").is_ok());
    }

    #[test]
    fn test_invalid_dates() {
        assert!(validate_date("invalid").is_err());
        assert!(validate_date("15-01-2024").is_err());
        assert!(validate_date("2024/01/15").is_err());
    }

    #[test]
    fn test_date_roundtrip() {
        let original = "2024-01-15 14:30:00 +0530";
        let parsed = validate_date(original).unwrap();
        let formatted = format_date_for_edit(&parsed);
        let reparsed = validate_date(&formatted).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_email_with_plus_sign() {
        assert!(validate_email("user+tag@example.com").is_ok());
        assert!(validate_email("test+123@test.co.uk").is_ok());
    }

    #[test]
    fn test_email_with_dots() {
        assert!(validate_email("first.last@example.com").is_ok());
        assert!(validate_email("a.b.c@test.org").is_ok());
    }

    #[test]
    fn test_email_subdomain() {
        assert!(validate_email("user@mail.example.com").is_ok());
        assert!(validate_email("test@subdomain.mail.example.com").is_ok());
    }

    #[test]
    fn test_email_no_domain() {
        assert!(validate_email("user@nodomain").is_err());
    }

    #[test]
    fn test_email_multiple_at_signs() {
        assert!(validate_email("user@@example.com").is_err());
        assert!(validate_email("user@test@example.com").is_err());
    }

    #[test]
    fn test_email_dot_at_start_or_end_of_domain() {
        assert!(validate_email("user@.example.com").is_err());
        assert!(validate_email("user@example.com.").is_err());
        assert!(validate_email("user@.example.com.").is_err());
    }

    #[test]
    fn test_email_empty_parts() {
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("@").is_err());
    }

    #[test]
    fn test_date_with_timezone_variations() {
        // Test different timezone formats
        assert!(validate_date("2024-01-15 14:30:00 +0000").is_ok());
        assert!(validate_date("2024-01-15 14:30:00 -0800").is_ok());
        assert!(validate_date("2024-01-15 14:30:00 +0530").is_ok());
        assert!(validate_date("2024-01-15 14:30:00+0000").is_ok());
        assert!(validate_date("2024-01-15 14:30:00-0800").is_ok());
    }

    #[test]
    fn test_date_utc_default() {
        // Test that dates without timezone default to UTC
        let dt = validate_date("2024-01-15 14:30:00").unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_date_midnight_default() {
        // Test that date-only input defaults to midnight
        let dt = validate_date("2024-01-15").unwrap();
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_date_with_seconds_zero() {
        // Test date without seconds (should default to 0 seconds)
        let dt = validate_date("2024-01-15 14:30").unwrap();
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_date_edge_cases() {
        // Test leap year
        assert!(validate_date("2024-02-29 12:00:00").is_ok());
        // Test non-leap year (should fail)
        assert!(validate_date("2023-02-29 12:00:00").is_err());
        // Test valid dates
        assert!(validate_date("2024-12-31 23:59:59").is_ok());
        assert!(validate_date("2024-01-01 00:00:00").is_ok());
    }

    #[test]
    fn test_date_invalid_formats() {
        // Wrong separators
        assert!(validate_date("2024/01/15 14:30:00").is_err());
        assert!(validate_date("2024.01.15 14:30:00").is_err());
        // Wrong order
        assert!(validate_date("15-01-2024 14:30:00").is_err());
        assert!(validate_date("01-15-2024 14:30:00").is_err());
        // Missing parts
        assert!(validate_date("2024-01 14:30:00").is_err());
        assert!(validate_date("2024 14:30:00").is_err());
    }

    #[test]
    fn test_date_invalid_values() {
        // Invalid month
        assert!(validate_date("2024-13-15 14:30:00").is_err());
        assert!(validate_date("2024-00-15 14:30:00").is_err());
        // Invalid day
        assert!(validate_date("2024-01-32 14:30:00").is_err());
        assert!(validate_date("2024-01-00 14:30:00").is_err());
        // Invalid hour
        assert!(validate_date("2024-01-15 24:30:00").is_err());
        // Invalid minute
        assert!(validate_date("2024-01-15 14:60:00").is_err());
        // Note: 60 seconds is valid for leap seconds, so we test 61 instead
        assert!(validate_date("2024-01-15 14:30:61").is_err());
    }

    #[test]
    fn test_date_whitespace_handling() {
        // Test that leading/trailing whitespace is handled
        assert!(validate_date("  2024-01-15 14:30:00  ").is_ok());
        assert!(validate_date("\t2024-01-15 14:30:00\t").is_ok());
    }

    #[test]
    fn test_format_date_for_edit() {
        use chrono::FixedOffset;

        let offset = FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap();
        let dt = offset.with_ymd_and_hms(2024, 1, 15, 14, 30, 45).unwrap();

        let formatted = format_date_for_edit(&dt);
        assert_eq!(formatted, "2024-01-15 14:30:45 +0530");
    }

    #[test]
    fn test_format_date_for_edit_negative_offset() {
        use chrono::FixedOffset;

        let offset = FixedOffset::west_opt(8 * 3600).unwrap();
        let dt = offset.with_ymd_and_hms(2024, 1, 15, 14, 30, 45).unwrap();

        let formatted = format_date_for_edit(&dt);
        assert_eq!(formatted, "2024-01-15 14:30:45 -0800");
    }

    #[test]
    fn test_date_timezone_preservation() {
        // Test that timezone is preserved through parse
        let original = "2024-01-15 14:30:00 +0530";
        let dt = validate_date(original).unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 5 * 3600 + 30 * 60);

        let original_negative = "2024-01-15 14:30:00 -0800";
        let dt_negative = validate_date(original_negative).unwrap();
        assert_eq!(dt_negative.offset().local_minus_utc(), -8 * 3600);
    }
}
