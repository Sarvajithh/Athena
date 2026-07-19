//! CSV Import connector (07_INTEGRATIONS.md §1.6). Institute grade/
//! timetable exports, or any other structured export, parsed locally —
//! no live sync, no credentials, re-run manually every semester.
//!
//! Per §1.6's note carried forward from `ROADMAP_REVIEW.md`, the real
//! institute export format wasn't available to test against while
//! building this — the parser below is deliberately generic (header
//! row + typed column mapping the caller supplies) rather than hard-
//! coded to one institute's column layout, so it isn't reworked from
//! scratch once that format is obtained; only the `CsvColumnMapping`
//! the caller passes in changes.

use std::collections::HashMap;

use crate::error::IngestionError;

/// One parsed data row, as a header-name -> cell-value map. The caller
/// (`athena-app`) resolves this against whichever typed entity the
/// import targets (`courses`, `deadlines`) — this connector has no
/// opinion on what a CSV's columns *mean*, only on turning rows of text
/// into addressable cells (§1.6: "any other structured export").
pub type CsvRow = HashMap<String, String>;

/// Parses `content` as RFC 4180-ish CSV: comma-separated, double-quote
/// escaping (`""` inside a quoted field is a literal `"`), first row is
/// the header. Hand-rolled rather than a third-party CSV crate: this
/// project's `NewCourse`/`NewDeadline` shapes are simple enough (no
/// embedded newlines expected in institute exports) that a small parser
/// covers the real format once obtained, without a dependency whose
/// full RFC 4180 edge-case surface this import path doesn't need.
pub fn parse_csv(content: &str) -> Result<Vec<CsvRow>, IngestionError> {
    let mut lines = content.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| IngestionError::Parse("CSV file is empty".into()))?;
    let headers = parse_csv_line(header_line);
    if headers.is_empty() {
        return Err(IngestionError::Parse("CSV file has no header row".into()));
    }

    let mut rows = Vec::new();
    for (line_number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cells = parse_csv_line(line);
        if cells.len() != headers.len() {
            return Err(IngestionError::Parse(format!(
                "row {} has {} cell(s), expected {} (matching the header row)",
                line_number + 2,
                cells.len(),
                headers.len()
            )));
        }
        let row: CsvRow = headers.iter().cloned().zip(cells).collect();
        rows.push(row);
    }

    if rows.is_empty() {
        return Err(IngestionError::Parse(
            "CSV file has a header row but no data rows".into(),
        ));
    }

    Ok(rows)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                cells.push(current.trim().to_string());
                current.clear();
            }
            other => current.push(other),
        }
    }
    cells.push(current.trim().to_string());
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_simple_grade_export() {
        let csv = "course_code,title,credits\nCS3231,Formal Methods,4\nCS4225,\"Big Data, Systems\",4\n";
        let rows = parse_csv(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["course_code"], "CS3231");
        assert_eq!(rows[1]["title"], "Big Data, Systems");
    }

    #[test]
    fn mismatched_column_count_is_a_parse_error() {
        let csv = "a,b,c\n1,2\n";
        let err = parse_csv(csv).unwrap_err();
        assert!(matches!(err, IngestionError::Parse(_)));
    }

    #[test]
    fn empty_file_is_a_parse_error() {
        let err = parse_csv("").unwrap_err();
        assert!(matches!(err, IngestionError::Parse(_)));
    }
}
