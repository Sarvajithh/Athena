//! Calendar Import connector (07_INTEGRATIONS.md §1.4). Local file
//! parse only, one-time per semester, through Semester Setup — never a
//! standing Google OAuth sync (§1.4's explicit reasoning). This module
//! is the "existing ICS parser" §1.4/§5 reference; see the V4
//! migration's doc comment for why it lives here rather than in
//! `athena-domain` (zero-I/O invariant).

use crate::error::IngestionError;

/// One `VEVENT` parsed out of an `.ics` file, shaped to map directly
/// onto a `deadlines` row (`title`, `due_at`, `notes`) — the caller in
/// `athena-app` still decides `category`/`leverage_class`/`semester_id`,
/// since none of those are present in a calendar event.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedIcsEvent {
    pub uid: Option<String>,
    pub summary: String,
    /// Raw `DTSTART` value, kept in whatever form the file has it
    /// (`YYYYMMDD` or `YYYYMMDDTHHMMSSZ`) — normalized to an ISO date
    /// by the caller, which already owns date/time-zone handling
    /// (`commands::planner`'s doc comment: "date/time stays a frontend
    /// concern in this schema").
    pub dtstart: Option<String>,
    pub description: Option<String>,
}

/// Parses every `VEVENT` block out of raw `.ics` file content.
/// Hand-rolled rather than a third-party ICS crate: the RFC 5545 subset
/// actually needed here (`SUMMARY`, `DTSTART`, `DESCRIPTION`, `UID`,
/// line folding) is small, and a hand-rolled parser has no transitive
/// dependency surface to audit for a file format Athena only ever
/// reads, never writes (§1.4: import-only, one direction).
pub fn parse_ics(content: &str) -> Result<Vec<ParsedIcsEvent>, IngestionError> {
    if !content.contains("BEGIN:VCALENDAR") {
        return Err(IngestionError::Parse(
            "not a valid .ics file: missing BEGIN:VCALENDAR".into(),
        ));
    }

    let unfolded = unfold_lines(content);
    let mut events = Vec::new();
    let mut in_event = false;
    let mut uid = None;
    let mut summary = None;
    let mut dtstart = None;
    let mut description = None;

    for line in unfolded.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VEVENT" {
            in_event = true;
            uid = None;
            summary = None;
            dtstart = None;
            description = None;
            continue;
        }
        if line == "END:VEVENT" {
            if let Some(summary) = summary.take() {
                events.push(ParsedIcsEvent {
                    uid: uid.take(),
                    summary,
                    dtstart: dtstart.take(),
                    description: description.take(),
                });
            }
            in_event = false;
            continue;
        }
        if !in_event {
            continue;
        }

        let Some((key, value)) = split_property(line) else {
            continue;
        };
        match key.as_str() {
            "UID" => uid = Some(value.to_string()),
            "SUMMARY" => summary = Some(unescape_ics_text(value)),
            "DESCRIPTION" => description = Some(unescape_ics_text(value)),
            k if k.starts_with("DTSTART") => dtstart = Some(value.to_string()),
            _ => {}
        }
    }

    if events.is_empty() {
        return Err(IngestionError::Parse(
            "no VEVENT blocks with a SUMMARY found in this .ics file".into(),
        ));
    }

    Ok(events)
}

/// RFC 5545 line folding: a line starting with a single space or tab is
/// a continuation of the previous line, joined with no separator.
fn unfold_lines(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for raw_line in content.split('\n') {
        if (raw_line.starts_with(' ') || raw_line.starts_with('\t')) && !result.is_empty() {
            result.push_str(raw_line[1..].trim_end_matches('\r'));
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(raw_line.trim_end_matches('\r'));
        }
    }
    result
}

/// Splits a `KEY;PARAM=x:value` or `KEY:value` line into `(KEY, value)`,
/// dropping any `;PARAM=...` segment — this parser only needs the bare
/// property name (e.g. `DTSTART` regardless of a `;TZID=...` param) and
/// the value.
fn split_property(line: &str) -> Option<(String, &str)> {
    let colon = line.find(':')?;
    let (key_part, value) = line.split_at(colon);
    let value = &value[1..];
    let key = key_part.split(';').next().unwrap_or(key_part).to_uppercase();
    Some((key, value))
}

fn unescape_ics_text(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:abc-123\r\n\
DTSTART;TZID=Asia/Kolkata:20260901T090000\r\n\
SUMMARY:CS3231 Midterm\r\n\
DESCRIPTION:Bring calculator\\, ID card\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:def-456\r\n\
DTSTART:20260915T140000Z\r\n\
SUMMARY:Internship application deadline\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn parses_two_events_with_expected_fields() {
        let events = parse_ics(SAMPLE).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].summary, "CS3231 Midterm");
        assert_eq!(events[0].dtstart.as_deref(), Some("20260901T090000"));
        assert_eq!(events[0].description.as_deref(), Some("Bring calculator, ID card"));
        assert_eq!(events[1].summary, "Internship application deadline");
    }

    #[test]
    fn rejects_non_ics_content() {
        let err = parse_ics("not an ics file").unwrap_err();
        assert!(matches!(err, IngestionError::Parse(_)));
    }
}
