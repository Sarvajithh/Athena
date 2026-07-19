//! One module per capability named in 06_AI_ENGINE.md §4 that this pass
//! implements. Per §1, none of these is a separate AI system — each is
//! a thin, capability-specific wrapper that shapes an `EvidencePayload`
//! (`context.rs`) from a value `athena-domain` already computed, then
//! calls the identical `pipeline::Synthesizer` (Stages 3–5). Callers
//! (`athena-app`) never construct a `PromptRequest` or call an
//! `LlmProvider` directly — this module is the whole public surface of
//! the AI layer.

pub mod daily_briefing;
pub mod reflection;
pub mod weakness_analysis;
pub mod weekly_planning;
