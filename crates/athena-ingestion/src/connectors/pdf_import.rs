//! Resume / Transcript / PDF Import connector (07_INTEGRATIONS.md
//! §1.5). Local-only file parse — zero network call — of a user-
//! supplied PDF. Extraction always ends in a confirmation step against
//! a typed schema field; this module never free-text-dumps parsed PDF
//! content into the database (§1.5) — it only proposes
//! `CandidateAchievement`s for the caller to show the user before any
//! `deadlines` row (see the V4 migration's doc comment on why
//! `deadlines` rather than an unbuilt `research_activities` table) is
//! ever written.

use crate::error::IngestionError;

/// One fact this connector believes it found in the document, offered
/// for confirmation — never committed automatically (§1.5).
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateAchievement {
    /// One of `"project"`, `"publication"`, `"certification"` — the
    /// three kinds §1.5 names. The caller maps this to a `deadlines`
    /// row's `category` (`career` for project/certification, `research`
    /// for publication) once the user confirms.
    pub kind: &'static str,
    pub title: String,
    /// The source line/sentence this was lifted from, shown to the user
    /// alongside the proposed title so confirming is an informed choice,
    /// not a blind "looks right" click.
    pub source_excerpt: String,
}

/// Extracts raw text from PDF bytes. Local-only — no network call, no
/// temp file left behind beyond what the OS's own memory-mapped read
/// needs (§1.5: "zero outbound dependency, zero fragility risk").
pub fn extract_text(pdf_bytes: &[u8]) -> Result<String, IngestionError> {
    pdf_extract::extract_text_from_mem(pdf_bytes)
        .map_err(|e| IngestionError::Parse(format!("could not read PDF: {e}")))
}

/// Heuristic candidate-fact extraction over already-extracted text.
/// Deliberately conservative: it looks for lines carrying an explicit
/// label (`Project:`, `Publication:`, `Certification:` and a few close
/// synonyms) rather than attempting general-purpose resume NLP —
/// producing zero candidates on an unrecognized format is the correct,
/// honest failure mode here (nothing gets silently invented), not a
/// bug to work around with looser heuristics.
pub fn extract_candidate_achievements(text: &str) -> Vec<CandidateAchievement> {
    const LABELS: &[(&str, &[&str])] = &[
        ("project", &["project:", "project -", "project –"]),
        ("publication", &["publication:", "paper:", "published:"]),
        (
            "certification",
            &["certification:", "certificate:", "certified:"],
        ),
    ];

    let mut candidates = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lowered = line.to_lowercase();
        for (kind, prefixes) in LABELS {
            for prefix in *prefixes {
                if let Some(pos) = lowered.find(prefix) {
                    let title = line[pos + prefix.len()..].trim().trim_matches(['-', '–', ':']).trim();
                    if !title.is_empty() {
                        candidates.push(CandidateAchievement {
                            kind,
                            title: title.to_string(),
                            source_excerpt: line.to_string(),
                        });
                    }
                    break;
                }
            }
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_labeled_lines_only() {
        let text = "\
John Doe
Software Engineer

Project: Athena — a self-hosted academic planner
Publication: \"Formal Verification of X\", ICSE 2025
Certification: AWS Solutions Architect
Some unrelated line about hobbies";
        let candidates = extract_candidate_achievements(text);
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].kind, "project");
        assert!(candidates[0].title.contains("Athena"));
        assert_eq!(candidates[1].kind, "publication");
        assert_eq!(candidates[2].kind, "certification");
    }

    #[test]
    fn produces_zero_candidates_on_unlabeled_text_rather_than_guessing() {
        let text = "A resume with no explicit labels of any kind, just prose.";
        assert!(extract_candidate_achievements(text).is_empty());
    }
}
