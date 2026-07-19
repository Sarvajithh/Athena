//! `athena-reasoning` — AI orchestration: context shaping, prompt
//! construction, provider dispatch, grounding validation, confidence
//! labeling, and local-model/template fallback (Master Spec §4.5,
//! 06_AI_ENGINE.md).
//!
//! This crate implements the five-stage pipeline §3 specifies, applied
//! identically to every capability in §4:
//!
//! ```text
//! Stage 1 (Retrieval)        -> context.rs   (shapes an EvidencePayload
//!                                              from an athena-domain
//!                                              verdict; no SQL here)
//! Stage 2 (Deterministic     -> NOT this crate. Every verdict this
//!          Scoring)             crate consumes was already computed by
//!                                athena-domain (`priority::resolve_priority`,
//!                                `planner::replan`). This crate has no
//!                                scoring logic of its own anywhere in
//!                                it — "the AI layer must consume the
//!                                Decision Engine instead of replacing
//!                                it" is enforced by dependency graph,
//!                                not just convention.
//! Stage 3 (Prompt             -> prompt.rs   (the only place a
//!          Construction)                      PromptRequest is built)
//! Stage 4 (Synthesis)        -> provider.rs, providers/  (the
//!                                LlmProvider trait boundary + its cloud
//!                                and local implementations)
//! Stage 5 (Grounding Check)  -> pipeline.rs  (Synthesizer: dispatches
//!                                Stage 4, verifies citations, retries
//!                                once, falls through to the zero-LLM
//!                                template)
//! Output                      -> output.rs   (the typed Recommendation
//!                                              every capability returns)
//! ```
//!
//! `capabilities/` is the crate's public surface: one function per named
//! capability (`daily_briefing`, `weekly_planning`, `weakness_analysis`,
//! `reflection`), each a thin wrapper around the same pipeline pointed
//! at a different `context.rs` builder — per §1, "naming them separately
//! is a product-surface convenience; architecturally there is one
//! pipeline."
//!
//! **Prompt centralization:** `prompt.rs` is the *only* place in the
//! entire application that assembles a Stage 4 prompt. Nothing in
//! `src/` (the React frontend) constructs prompt text, imports a prompt
//! template, or has any notion of what a `PromptRequest` looks like —
//! the IPC boundary (`athena-app::commands`) only ever crosses a typed
//! `Recommendation` (`output.rs`), never a raw prompt or a raw LLM
//! response.
//!
//! **Offline-first (§10):** `pipeline::Synthesizer::synthesize` cannot
//! fail — its return type is `Recommendation`, not `Result`. With zero
//! providers configured, an unreachable cloud provider, or an
//! unreachable local model, the worst case is
//! `Recommendation::from_template`: fully grounded, just less fluent
//! (§10.2). Every capability in this crate is therefore safe to call
//! unconditionally, with no "is AI available" branch required at any
//! call site.

pub mod capabilities;
pub mod context;
pub mod error;
pub mod output;
pub mod pipeline;
pub mod prompt;
pub mod provider;
pub mod providers;

pub use context::{EvidenceItem, EvidencePayload, WeaknessSignal};
pub use error::ReasoningError;
pub use output::Recommendation;
pub use pipeline::Synthesizer;
pub use provider::{LlmProvider, PromptRequest};

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end smoke test across the whole pipeline with zero
    /// providers configured — the exact "LLM unavailable" state the
    /// product must remain fully usable in (Master Spec, 06_AI_ENGINE.md
    /// §10). Exercises Stage 1 (context.rs) through Output (output.rs)
    /// without touching the network.
    #[test]
    fn daily_briefing_end_to_end_with_no_llm_configured_is_fully_grounded() {
        use athena_domain::priority::{resolve_priority, DeadlineCandidate};

        let candidates = vec![DeadlineCandidate {
            id: 42,
            title: "Company X application".into(),
            due_at: "2026-07-20T00:00:00".into(),
            leverage_class: "high".into(),
        }];
        let verdict = resolve_priority(&candidates);

        let synthesizer = Synthesizer::new(vec![]);
        let rec = capabilities::daily_briefing::build_daily_briefing(
            &synthesizer,
            &verdict,
            "as of 2026-07-17T09:00:00Z",
        );

        assert_eq!(rec.source, "template");
        assert_eq!(rec.grounded_in, vec![42]);
        assert_eq!(rec.confidence, "inferred");
        assert!(!rec.reasoning.is_empty());
    }
}
