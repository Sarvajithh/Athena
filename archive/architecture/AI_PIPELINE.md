# AI_PIPELINE.md — Project Athena

## 1. The Core Constraint This Pipeline Exists to Satisfy

`NON_NEGOTIABLES.md` §5: *"Athena is never allowed to guess at your state
and present the guess as fact."* And §10: *"A wrong confident
recommendation is more dangerous than an honest 'I don't have enough
information.'"*

An LLM, used naively, violates both by default — it will happily produce
fluent, confident-sounding text about a CGPA trend it wasn't actually
given, because that's what fluent text generation does when under-
specified. The entire design of this pipeline is about removing the LLM's
ability to be the source of any *fact*, while keeping it as the thing that
turns already-verified facts into a well-reasoned, readable answer. This
is restated from `ARCHITECTURE.md` §3 because it is the single most
important sentence in this document: **the LLM is a writer, not a
decider.**

## 2. Pipeline Stages

```
[1] Trigger            [2] Retrieval          [3] Deterministic Scoring
(event or user           (athena-data,           (athena-domain — priority,
 query)          ─────▶   grounded facts    ────▶  bottleneck, drift,
                          only, with source        divergence — pure Rust,
                          + freshness stamps)       no LLM involved)
                                                          │
                                                          ▼
[6] Output            [5] Grounding Check      [4] Synthesis
(Recommendation         (reject any claim in     (LLM call — turns Stage 3's
 row, confidence  ◀────  the LLM output not  ◀────  verdicts into a ranked,
 + grounding                traceable to a          justified sentence —
 attached)                Stage 2 fact)             cannot introduce new facts)
```

### Stage 1 — Trigger
Either a user-initiated query (opening the "Now" screen) or a system event
(`DriftDetected`, `SemesterRolledOver`, a scheduled deep-work allocation at
19:45). See `EVENT_SYSTEM.md` §5.

### Stage 2 — Retrieval
`athena-reasoning::retrieval` pulls exactly the rows relevant to the
trigger — current deadlines, latest grade/Codeforces/project snapshots,
current bottleneck, current drift signals — each tagged with its
`data_sources.last_synced_at`. Nothing is summarized or interpreted yet.
If a required input is missing or stale beyond `data_sources
.staleness_threshold_hours`, that absence is itself passed forward as a
retrieval fact ("no grade snapshot in 34 days"), not silently skipped.

### Stage 3 — Deterministic Scoring
This is the stage that actually decides. `athena-domain`'s priority
resolution, bottleneck detection, drift scoring, and divergence check run
against the retrieved facts and produce a structured **verdict**: a
ranked answer (or a small comparably-ranked set, per
`CORE_PRINCIPLES.md` #9), a confidence class, and the specific evidence
rows that justify it. No natural language yet — this is typed Rust data.

Why this stage exists at all instead of asking the LLM to "reason over
the data": determinism and testability. The priority resolution algorithm
can be unit-tested with fixed inputs and an exact expected output; an LLM
asked to do the same reasoning cannot be pinned down that way, and this is
the one piece of the system that must never silently drift in behavior
across a provider/model upgrade five years from now.

### Stage 4 — Synthesis
Only now does the LLM get involved. It receives the Stage 3 verdict
(structured) and is asked to do exactly one job: **express this verdict
as a single, direct, well-reasoned sentence or short paragraph, using only
the facts provided.** The prompt is deliberately narrow — it is not asked
"what should the user do," because that question was already answered in
Stage 3. It's asked "explain this answer clearly."

The synthesis call is explicitly instructed, and structurally constrained
via a JSON schema response, to:
- Never soften a negative verdict (`NON_NEGOTIABLES.md` §1) — the schema
  separates `verdict` (fixed, from Stage 3) from `tone`, and tone is not
  permitted to alter verdict content, only phrasing.
- Distinguish, in its own output, which parts are `confirmed` (directly
  from a retrieved row) vs `inferred` (a reasonable read of a trend that
  Stage 3 flagged as inference) — mirroring `CORE_PRINCIPLES.md` #10.
- Cite the specific evidence it's referencing using stable IDs supplied in
  the prompt (e.g. `[grade_snapshot:4821]`), not free-text description of
  data it wasn't given verbatim.

### Stage 5 — Grounding Check
Before anything reaches the user, `athena-reasoning::grounding` parses the
LLM's cited evidence IDs and verifies every one resolves to a row that was
actually present in the Stage 2 retrieval payload. **Any claim citing an ID
not present, or containing an unsourced factual assertion, causes the
entire synthesis to be rejected and retried once with a stricter prompt;
on a second failure, the pipeline falls back to a template-based rendering
of the Stage 3 verdict with no prose embellishment**, rather than shipping
an ungrounded sentence. This is the concrete enforcement of
`NON_NEGOTIABLES.md` §5 — grounding is checked by code, not trusted from
the model.

### Stage 6 — Output
A `recommendations` row is written (see `DATABASE_SCHEMA.md` §2) with
`verdict`, `reasoning`, `confidence`, `grounded_in`, and
`data_freshness_note` all populated — `confidence` and freshness are
non-nullable, so the UI can never render a recommendation without a
visible confidence signal (`NON_NEGOTIABLES.md` §10).

## 3. Confidence Model

Three explicit classes, chosen to make `CORE_PRINCIPLES.md` #10's
distinction ("I'm confident, here's why" vs. "I'm inferring this, treat it
as a hypothesis") a first-class, renderable state rather than a tone:

| Class | Meaning | UI Treatment |
|---|---|---|
| `confirmed` | Verdict follows directly from retrieved, fresh data | Normal presentation |
| `inferred` | Verdict follows from a trend/pattern read, not a single hard fact (e.g. "likely drifting" from a 4-day slope) | Explicitly labeled "hypothesis" in the UI, per NON_NEG §10 |
| `insufficient_data` | Retrieval didn't have enough to produce Stage 3 output at all | Rendered as an honest gap, never papered over with a generic answer |

`insufficient_data` is a **first-class, expected state**, not an error —
this matters most at the start of a new semester (`ARCHITECTURE.md` §6,
cold-start correctness), where Athena should say exactly that rather than
extrapolate from last semester (which would itself violate
`NON_NEGOTIABLES.md` §7).

## 4. Model Choice: Hybrid Cloud + Local

**Primary path:** a cloud LLM API call (Claude, via the Anthropic API) for
Stage 4 synthesis. Chosen because synthesis quality — producing a genuinely
well-reasoned, non-generic sentence — currently benefits meaningfully from
frontier model capability, and Stage 4 by design cannot introduce facts,
so the risk surface of using a cloud model is bounded to phrasing quality,
not correctness.

**Fallback path:** a local model (e.g. via Ollama, a small instruction-
tuned model) used when offline or when the user opts out of cloud calls
for a session. This exists for two reasons: (1) `ARCHITECTURE.md` §6's
offline-first requirement — priority resolution must still work with no
network, even if the prose is more template-like; (2) 5-year survivability
— a system with a hard dependency on one vendor's API staying available
and unchanged for 5 years is fragile. The local path doesn't need to match
cloud quality; it needs to guarantee the pipeline degrades to *something
grounded and honest* rather than failing entirely.

**What's sent off-device:** only the Stage 2 retrieval payload relevant to
the specific synthesis call — never the full database, never raw personal
identifiers beyond what's needed (e.g. course names and scores, not
student ID numbers). This is the practical implementation of minimizing
what leaves local storage, in service of `NON_NEGOTIABLES.md` §8's
ownership principle even though the LLM call itself is an explicit,
narrow exception to "fully local."

## 5. The Grounding Contract, Concretely

Every Stage 4 prompt includes a manifest of exactly which facts (with
stable IDs) the model is allowed to reference, and the system prompt
states plainly that any claim not traceable to that manifest will be
programmatically discarded. This is not a soft instruction the model can
ignore without consequence — Stage 5 actually enforces it. This is the
mechanism that lets `NON_NEGOTIABLES.md` §5 be a guarantee rather than a
hope: **the worst-case failure mode of this pipeline is a template-
flattened, fact-only answer — never a fluent hallucination.**

## 6. Interaction with the Decision Challenge Layer

When Stage 1's trigger is a `SubmitDecision` command (see
`EVENT_SYSTEM.md` §4), the pipeline runs identically, except Stage 3's
verdict comes from evaluating the decision *hypothetically* against
current bottleneck/drift state, and Stage 6's output is a `challenge`-kind
recommendation attached to that specific command rather than a standing
`priority_now` recommendation. The grounding and confidence discipline is
identical — a challenge is held to exactly the same "must be traceable to
real data" bar as any other recommendation, per `NON_NEGOTIABLES.md` §5
applying without exception.

## 7. What This Pipeline Deliberately Does Not Do

- It does not let the LLM call any tool that mutates state. Synthesis is
  read-only by construction — all writes happen through Commands
  (`EVENT_SYSTEM.md`), never as a side effect of a model response.
- It does not maintain open-ended conversational memory as its primary
  mode. A follow-up chat surface may exist for the user to ask "why?"
  about a specific recommendation (re-running Stage 4 with the same Stage
  2/3 payload plus the question), but the system's primary value doesn't
  depend on the user initiating conversation (`VISION.md`: "Athena
  pushes; you don't have to pull").
- It does not fine-tune or retrain a model on the user's data. Given
  single-user scale, the ROI is negative and it would create exactly the
  kind of opaque, hard-to-audit behavior this whole pipeline is designed
  to avoid.
