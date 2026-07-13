# MASTER_SPECIFICATION.md — Project Athena
### Single Source of Truth — supersedes VISION.md, NON_NEGOTIABLES.md, CORE_PRINCIPLES.md, PROJECT_SCOPE.md, USER_PROFILE.md, ARCHITECTURE.md, MODULES.md, EVENT_SYSTEM.md, DATABASE_SCHEMA.md, API_INTEGRATIONS.md, FOLDER_STRUCTURE.md, AI_PIPELINE.md, all `docs/ui/*.md`, and all `docs/ai/*.md`.

## 0. How to Read This Document

Four teams produced independent design documents for the same product.
They agree on almost everything philosophical and disagree on a
surprising amount architecturally. This document is not a summary of
those four — it is a **judgment**. Where two documents agreed, that
agreement is now law. Where they conflicted, Section 1 shows the conflict
and states which side won, and why. Every section after that is written
as if the conflict never happened — clean, authoritative, buildable.

If a future change conflicts with this document, the burden of proof is
on the change. This document itself should be revised deliberately, in
writing, not silently overridden by a single PR.

---

## 1. Critical Review — Contradictions, Duplication, Risk, and Simplification

This section exists because the brief asked for critical review, not a
summary. Nothing below is soft-pedaled.

### 1.1 The Single Biggest Problem: Two Incompatible AI Architectures

The **Software Architecture team** (`AI_PIPELINE.md`) and the **AI Design
team** (`docs/ai/*.md`) each designed a complete, self-consistent AI
system — and they are not the same system.

| | Architecture team's design | AI Design team's design |
|---|---|---|
| What decides | Deterministic Rust (`athena-domain`) — pure functions, unit-testable, exact expected output | An LLM-driven "Engine" layer inferring habits, weaknesses, and blind spots from a "Memory System" with confidence scores |
| What the LLM is for | Turning an already-computed verdict into a sentence ("writer, not decider") | Reasoning, remembering, inferring patterns, and holding a "credibility ledger" on the user's judgment |
| Memory | Typed SQLite tables (snapshots, decisions, drift_signals, bottlenecks) | A bespoke four-part memory system (episodic / semantic / procedural-habit / decision), with LLM-driven "distillation," "decay," and "tension flags" |
| Interaction model | Push-only; no persistent conversational memory as the primary mode | Daily "morning briefing" and "evening debrief," a 10–15 minute structured weekly conversational review |
| Grounding guarantee | Every claim traces to a retrieved row; ungrounded claims are programmatically rejected (Stage 5) | "Blind-spot weaknesses" are explicitly detected by the LLM noticing a *pattern the user hasn't self-reported* — i.e., inference presented as insight, with no schema-level grounding check at all |

This is not a difference in emphasis. It is two different products. The
AI Design docs describe something closer to a therapist/executive-coach
chatbot with persistent memory (their own worked examples — "you're a
founder," "the vendor missed their date," "a client call" — are generic
startup-founder scenarios, not this user's actual life, which is itself
evidence the AI Design docs were not written against `USER_PROFILE.md` at
all, but adapted from a template).

**Ruling: the Architecture team's design wins, in full, without
compromise.**

Reasons:
1. `NON_NEGOTIABLES.md` §5 ("never allowed to guess at your state and
   present the guess as fact") and §10 ("a wrong confident recommendation
   is more dangerous than an honest 'I don't have enough information'")
   are not satisfiable by a system whose core mechanism is an LLM
   inferring psychological patterns ("habit: works late, may affect
   decision quality") from indirect behavioral logs and assigning them a
   confidence score. That is exactly the guess-presented-as-fact pattern
   the non-negotiable forbids — just with a probability number attached
   to make it look rigorous.
2. The Architecture team's Stage 5 grounding check is an actual,
   code-enforced guarantee. The AI Design team's "tension flags" and
   "blind spot" detection have no equivalent enforcement mechanism
   anywhere in their own documents — they rely entirely on the LLM's
   discipline, which is precisely what `ARCHITECTURE.md` §3 already
   identified as the failure mode of "just CRUD with a chatbot bolted
   on."
3. A bespoke four-type memory system with decay, reinforcement, and
   LLM-driven distillation is a large, ongoing engineering and cost
   liability for a single-developer, 5-year project — and it duplicates
   data that already exists, better-structured, in `DATABASE_SCHEMA.md`
   (see 1.5 below).
4. The AI Design docs' interaction model (daily conversational debriefs,
   a 10–15 minute weekly "session") directly contradicts `VISION.md`
   ("Athena pushes; you don't have to pull") and `ARCHITECTURE.md` §8
   ("no generic chatbot surface as the primary interface"). It also
   reintroduces exactly the decision-fatigue and typing friction the
   whole project exists to remove — CGPA 7.38 → 8.8 does not improve
   because the user spent 15 minutes a week talking to a chatbot about
   his feelings.

**What survives from the AI Design docs, re-grounded:**
- The **cadence model** (daily / weekly / semester horizons) is genuinely
  good and is kept — but re-implemented as *when the existing
  deterministic scans run and what they surface*, not as conversational
  check-ins requiring the user to answer questions. See §6.4.
- The **Signal Threshold** (recurrence, stakes, reversibility,
  contradiction — surface only if 2 of 4 are met) is kept as the concrete
  algorithm behind `drift_signals.severity` and the Priority Resolution
  closeness threshold — implemented in `athena-domain`, deterministically,
  not as an LLM judgment call. See §6.5.
- The **persona/tone guidance** (direct, no sycophancy, says "I disagree"
  without hedging, earns the right to challenge by being right about
  small things) is kept as the literal system-prompt guidance for Stage 4
  synthesis. See §6.6.
- The **"say it once, then respect the decision"** rule was already
  independently specified by the Architecture team (`decisions
  .final_outcome`, "not re-challenged") — both teams converged on this
  correctly, so it stands as doubly-confirmed.
- The **Opportunity Engine** concept is kept, but reframed as a
  deterministic query over the `opportunities` table plus
  `trajectory_metrics`, not an LLM "scanning episodic memory for passing
  mentions."

**What is explicitly rejected:** the four-type Memory System in its
entirety (`MEMORY_SYSTEM.md`), LLM-inferred habit/weakness/blind-spot
detection, the "credibility ledger" as an LLM-computed subjective score,
conversational daily/weekly check-ins as a required interaction pattern,
and any feature that asks the LLM to notice something about the user that
isn't traceable to a row in the database. See §11 for the full rejected
list.

### 1.2 UI Philosophy vs. Everything Else: "Never Make the User Feel Bad"

`docs/ui/ANALYTICS.md` states outright: *"If a metric could make the user
feel bad about themselves, it does not appear here."* `docs/ui/CAREER_VIEW.md`
states Career View should show *"no deadlines... with urgency styling"*
because it should be *"the calmest screen in the app."* `UI_GUIDELINES.md`
bans red, exclamation marks, and "urgency language" categorically.

This directly contradicts two Non-Negotiables the UI team had direct
access to:
- §1: *"Athena's job is to optimize Future You, not to make Present You
  feel good in the moment... Comfort-preserving silence is a failure
  mode, not politeness."*
- §6: *"If a subject, skill, or habit is a genuine liability... Athena
  must name it plainly and keep naming it until it's resolved — not
  soften language over time... Diplomacy is fine; obscuring the truth is
  not."*

An analytics screen that structurally cannot show an ugly truth, and a
Career View that refuses to show real urgency on a real internship
application deadline, are both in direct violation.

**Ruling: the non-negotiables win. "Calm" is a style constraint, not a
content constraint.**

The UI team's underlying instinct — don't manufacture *artificial*
urgency, don't use red for its own sake, don't gamify — is good and is
kept. What's rejected is the absolute rule that *no* real signal may ever
look serious. The resolution: severity (`watch` / `flag` / `urgent` from
`drift_signals`, and real `apply_by` proximity on `opportunities`) is
always visually distinguishable, using the calm visual language's own
vocabulary (the UI docs' own "single soft amber dot," used correctly
elsewhere in the same documents for exactly this purpose) rather than
alarm-red screaming banners. Muted ≠ hidden. Every screen, including
Career View and Analytics, must be able to render `urgent` distinctly
from `watch` — the earlier "never" language is struck.

### 1.3 UI Philosophy vs. Architecture: "No Modals" vs. the Blocking Challenge Layer

`UI_GUIDELINES.md` §4: *"No modal pop-ups for planning decisions. Modals
interrupt; Athena never interrupts."*

`EVENT_SYSTEM.md` §4 and `MODULES.md` §5 (the Decision Challenge Layer,
which directly implements `NON_NEGOTIABLES.md` §4 and `CORE_PRINCIPLES.md`
#3): *"The UI shows a single, blocking `ChallengeDialog` — once... The
command blocks pending user confirmation."*

These cannot both be true. A system that "never interrupts" cannot
structurally challenge a decision *before* it commits — which is the
entire mechanism that makes §4 ("no decision made silently") real instead
of aspirational.

**Ruling: the Challenge Layer's blocking interaction wins, as a narrow,
named exception — not a general reopening of "modals are fine."**

The `ChallengeDialog` and the Deep Work Guard's override-confirmation
prompt are the **only two** interruptive, blocking UI moments in the
entire app. Everything else in `UI_GUIDELINES.md` §4 (no modals for
planning, inline dismissible prompts, undo-everywhere) stands as written.
This is a feature, not a compromise: because interruption is used
nowhere else, these two moments retain their weight instead of being
diluted into the general noise a "no modals, ever" system was trying to
avoid in the first place.

### 1.4 UI Philosophy vs. Vision/Scope: Seven Screens, Not Four

`ARCHITECTURE.md` §7 and `MODULES.md` §8, both explicitly citing
`CORE_PRINCIPLES.md` #11 ("minimal surface, maximum signal"), specify
**four** screens: Now, Trajectory, Semester Setup, Decision Log.

The UI team independently designed **seven**: Home Screen, Dashboard,
Daily Planner, Weekly Planner, Semester View, Career View, Analytics —
each with its own navigation, its own empty states, its own inspiration
notes. None of the seven cite `CORE_PRINCIPLES.md` #11 or any other
foundational document at all — the UI docs are the only design documents
in the whole set that never reference the five foundational files by
name.

This is real duplication, not just a naming mismatch: "Home Screen" and
"Now" both claim to be the first thing the user sees; "Dashboard,"
"Weekly Planner," and "Semester View" together re-implement most of what
`Trajectory` already covers, plus a general-purpose calendar the Vision
document explicitly rules out (*"Not a passive log of events
[Calendar]"*). "Career View" duplicates part of `Trajectory`. Several
screens (Weekly Planner's "Task / Personal" quick-add, Home Screen's
"Quick Capture") quietly reintroduce a general task manager, which
`PROJECT_SCOPE.md` explicitly excludes and `DATABASE_SCHEMA.md` §4
explicitly has no table for.

**Ruling: four screens, per the Architecture team, stands. The UI team's
visual language is retained and remapped onto those four screens; the
extra three screens' functions are folded in or cut.**

See §5 for the reconciled screen list and what happened to each of the
seven original screens' functions.

### 1.5 Duplication: A Second, Shadow Data Model

Beyond the AI Design docs' memory system (1.1), there's a narrower
duplication worth calling out on its own: `DATABASE_SCHEMA.md` already
has `grade_snapshots`, `dsa_practice_log`, `codeforces_snapshots`,
`deep_work_sessions`, `bottlenecks`, `drift_signals`, `decisions`, and
`recommendations` — this **is** Athena's memory. It is typed, queryable,
migration-safe, and already satisfies every non-negotiable. The AI Design
docs' Memory System re-invents a parallel, untyped, LLM-native version of
the same concept (episodic memory ≈ `event_log` + raw entries; semantic
memory ≈ `user_profile`/`user_profile_history`; habit memory ≈ what
`drift_signals` and repeated `bottlenecks` already do; decision memory ≈
literally the `decisions` table, renamed). Building both is not
belt-and-suspenders, it's two belts. Rejected in favor of the one that
already exists and is enforceable.

### 1.6 Unrealistic Assumption: Windows-Only

`ARCHITECTURE.md` §2 calls Athena *"a Windows desktop app,"*
`API_INTEGRATIONS.md` §6 is titled *"Windows OS Integration,"* and
`FOLDER_STRUCTURE.md` names `tray.rs` as *"Windows tray/notification
integration."* Nothing in `USER_PROFILE.md` or any other document states
the user's operating system, and IIT-Hyderabad AI/CS students are at
least as likely to run Linux or macOS day-to-day as Windows. Tauri is
cross-platform at near-zero marginal cost; hardcoding Windows-specific
language into three separate documents is an assumption nobody actually
made a case for — it just leaked in from Tauri's most common tutorial
target.

**Ruling: generalize to cross-platform (Windows/macOS/Linux) via Tauri's
native notification and tray APIs on each platform.** If the user is in
fact Windows-only, this costs nothing to narrow later; assuming it now
and being wrong costs a rewrite of the notification/tray layer.

### 1.7 Unnecessary Complexity Flagged and Cut

- **LLM-driven "distillation" of episodic → semantic memory on a
  schedule** (`MEMORY_SYSTEM.md` §3): cut. Replaced by ordinary SQL
  queries and the scheduled `DriftScan` job that already exists.
- **"Tension flags" resolved by the system asking the user to adjudicate
  contradictions in its own inferred model of them** (`MEMORY_SYSTEM.md`
  §5): cut. If two data points conflict, the system shows both, dated,
  and lets the Priority Resolution / Decision Log surface it factually —
  no separate conversational reconciliation flow.
- **Per-domain "credibility ledger" as an LLM-graded score of the user's
  judgment** (`DECISION_ENGINE.md` §3): cut from v1. A deterministic
  version (override-rate per `decision_type`, computed by SQL over the
  `decisions` table) is listed as a Future Feature (§10) if it proves
  useful later — but it is not core, and it is never LLM-graded.
- **Two independent "confidence"/"threshold" systems** (Architecture's
  `confirmed`/`inferred`/`insufficient_data` + `drift_signals.severity`,
  and AI Design's Signal Threshold): merged into one — see §6.5. Running
  both as separately-maintained systems would be duplicated logic with no
  benefit.
- **Seven screens' worth of independently-designed empty states,
  navigation transitions, and "inspiration notes"**: cut down to four
  screens' worth (§5). Every removed screen's one or two genuinely useful
  ideas (the "+N more" collapse rule, the muted-severity dot language,
  the swipe-between-density-levels pattern) are kept as *interaction
  patterns available to any screen*, not screens of their own.

### 1.8 What the Four Teams Got Right, Independently

Worth stating plainly, since a critical review can read as all-negative:
the **five foundational documents, the Software Architecture documents,
and the visual/interaction language of the UI documents are excellent**
and required almost no correction. The event/command distinction, the
grounding pipeline, the schema's snapshot-over-overwrite discipline, and
the UI's restraint-over-decoration instinct are all kept close to
verbatim. The AI Design docs' cadence and signal-threshold *ideas* are
good even though their *implementation* (a bespoke LLM memory system) is
rejected. The corrections in this section are concentrated almost
entirely in two places: the AI Design docs' implementation approach, and
the UI docs' failure to check their output against the non-negotiables
and the architecture's screen count before designing seven screens' worth
of detail.

---

## 2. Vision

Athena is a single-user Personal Operating System whose job is to sit
between the user and every decision that determines whether he becomes
the person who gets into Imperial/Oxford for an MSc in Mathematics and
lands a high-paying ML/Quant career — or the person who almost did.

It is infrastructure, not an app to check. A calendar tells you what's
happening; a to-do list tells you what's pending; **Athena tells you what
matters right now, and why**, continuously, without being asked.

**The core bet:** the user's bottleneck is allocation, not laziness —
hours are spent, but not always on the highest-leverage thing available.
Every feature must pass one test: *does this reduce the number of
decisions he has to make about WHAT to work on, and does it bias those
decisions toward Future-Him over Present-Him?*

**End state (3–5 years):** CGPA ≥ 8.8 with drift caught a semester early;
a DSA/Codeforces/project/research portfolio that makes him the strongest
candidate in the pool, not a hopeful applicant; an Imperial/Oxford MSc
application that's a formality because it was engineered for three years,
not assembled in the final two months; a person who is never confused, on
any given evening, about the best use of the next hour.

**Not:** a passive event log, a flat unranked to-do list, a scheduling
optimizer that treats all tasks as equally worthy, or a chatbot that has
to be interrogated to produce value. Athena pushes; the user doesn't have
to pull.

Success is measured in **trajectory** — CGPA slope, skill-acquisition
rate, opportunity capture rate — never in tasks completed.

---

## 3. Core Principles

### 3.1 Non-Negotiables (hard constraints — never traded off)

1. **Trajectory over comfort.** Athena optimizes Future You, not Present
   You's mood. Willing to say things the user doesn't want to hear.
2. **Never a passive reminder.** Every surfaced item carries a
   recommendation, a reason, or a trade-off — never a bare notification.
3. **The 8 PM–midnight deep-work block is sacred.** Nothing low-leverage
   is scheduled or passively allowed into it without explicit override.
4. **No decision made silently on the user's behalf.** Irreversible or
   high-stakes actions require confirmation; Athena recommends and
   challenges, it does not act unilaterally.
5. **Grounded in reality, never guessed.** Every recommendation traces to
   real data. Missing/stale data is stated explicitly, never
   interpolated.
6. **Weaknesses tracked honestly, never softened.** A genuine liability
   is named plainly and kept named until resolved by evidence.
7. **The system adapts to the semester, not the reverse.** No fixed
   weekly template is ever assumed to still be valid.
8. **Privacy and sole ownership.** Single-tenant, single-user, forever.
   Data lives on disk under the user's control.
9. **No metric gaming.** Proxy metrics (tasks completed, hours logged)
   are never optimized at the expense of trajectory metrics (CGPA,
   rating, portfolio strength); divergence between the two is flagged.
10. **Fail loud, not silent.** Low confidence or missing data is stated
    explicitly rather than papered over with false confidence.

### 3.2 Principles (judgment calls, weighed against each other)

1. Reduce the decision, don't just surface the data — arrive at a ranked,
   justified answer, don't hand raw options back.
2. Every recommendation carries its "why," proactively, in one sentence.
3. Challenge, don't just comply — push back with substance once, then
   respect the final call.
4. Protect deep work like capital — allocate the sacred window
   deliberately to the single highest-expected-return activity.
5. Treat semester context as a first-class, frequently-refreshed input.
6. Trajectory over task-completion, always.
7. Early signal beats late correction — catch drift as a trend, not a
   post-mortem.
8. Bottleneck-first thinking — always be able to name the single biggest
   current constraint.
9. Present options only when genuinely, closely ambiguous; otherwise
   decide and say so.
10. Be honest about confidence and limits — distinguish "confirmed" from
    "inferred, treat as hypothesis."
11. Minimal surface, maximum signal — fewer, denser, higher-signal
    touchpoints over a sprawling dashboard.
12. Build for the person the user is becoming — favor durable capital
    over short-term convenience.

---

## 4. Software Architecture

### 4.1 Style

**Athena is a modular monolith running inside a Tauri desktop shell**,
with an in-process event bus connecting independently-testable Rust
modules, backed by a single local SQLite database as the sole source of
truth. Deliberately not microservices (wrong shape for one user, one
machine) and not a cloud-backend thin client (violates §3.1's ownership
non-negotiable).

### 4.2 Stack

| Layer | Choice | Why |
|---|---|---|
| Shell | Tauri (cross-platform — Windows, macOS, Linux) | Small native binary, typed Rust↔JS IPC boundary, direct OS notification/tray integration on each platform |
| Frontend | React + TypeScript | Presentation and light client state only — never domain logic |
| Domain logic | Rust, in `src-tauri` | Bottleneck detection, drift scoring, deep-work guard, divergence check — must be correct and stable, so it's pure, typed, and heavily tested |
| Database | SQLite, single file | Zero-ops, trivially backed up, matches sole-ownership requirement, more than sufficient for one user's history |
| AI inference | Hybrid: cloud LLM (primary, synthesis only) + local model (fallback) | Reasoning quality favors frontier models for phrasing; the deterministic scoring behind it never leaves Rust |
| Internal messaging | In-process event bus (`tokio::sync::broadcast` + typed command dispatcher) | One process, one user — an external MQ is pure overhead |

### 4.3 Why This Cannot Be "CRUD With a Chatbot Bolted On"

This is the load-bearing architectural decision of the whole project (see
§1.1 for why the AI Design docs' alternative was rejected). **Deterministic
Rust scoring produces facts and verdicts → the event/command system
enforces where those facts get to intervene (blocking writes, generating
recommendations, escalating drift) → the LLM's only job is to turn
already-computed, already-grounded signals into a well-reasoned sentence.**
The LLM is a writer, not a decider. This single decision is what makes
almost every non-negotiable enforceable in code rather than aspirational
in a prompt.

### 4.4 Layering

```
Presentation (React + TS)
  Now · Trajectory · Semester Setup · Decision Log
  — talks to Rust ONLY via typed Tauri commands
        │  typed IPC
Application Layer (Rust) — command/query handlers, Tauri bindings
        │
Domain Layer (Rust, pure — zero I/O)
  Priority Resolution · Bottleneck Detection · Drift Scoring ·
  Deep Work Guard · Divergence Check
        │
Reasoning/AI Layer (Rust orchestrator + LLM client)
  Retrieval → grounding → synthesis → confidence labeling
        │
Event Bus (in-process) — Commands (interceptable) · Events (async)
        │
Data Layer (Rust repositories) → SQLite
  Ingestion connectors: Codeforces, ICS import, CSV import
```

The domain layer depends on nothing outside the Rust standard library and
domain value types — it can be unit-tested with no infrastructure and can
outlive a full rewrite of the UI, persistence, or LLM vendor. This is the
single highest-value decision for 5-year survivability.

### 4.5 Module Map

- **`athena-domain`** — pure reasoning rules. Sub-modules: `priority/`
  (the load-bearing algorithm, >90% branch coverage bar), `bottleneck/`,
  `drift/`, `deep_work/` (hard guard + allocator), `divergence/`
  (proxy-vs-trajectory metric check). Depends on nothing internal.
- **`athena-data`** — one repository per aggregate; the only crate
  allowed to write SQL; owns migrations.
- **`athena-events`** — the Command/Event bus and interceptor registry,
  including the Decision Challenge Layer (see §4.6).
- **`athena-reasoning`** — AI orchestration: retrieval, prompt
  construction, grounding validation, confidence labeling, local-model
  fallback.
- **`athena-ingestion`** — external connectors (Codeforces, ICS, CSV).
  Isolated because ingestion is the most likely thing to break over 5
  years.
- **Scheduler** (inside `athena-app`, not its own crate) — a dumb timer
  that fires events (`DriftScan`, staleness checks, deep-work close-out);
  all actual logic stays in `athena-domain`, testable without a clock.

Dependency rule, enforced by the Cargo workspace, not convention:
```
athena-app
   ├── athena-events ──┬── athena-domain
   │                    └── athena-data
   ├── athena-reasoning ── athena-domain, athena-data
   └── athena-ingestion ── athena-data

athena-domain depends on NOTHING internal.
```

### 4.6 Commands vs. Events, and the Decision Challenge Layer

Two message kinds:

| | Command | Event |
|---|---|---|
| Meaning | "Please do this" | "This already happened" |
| Timing | Synchronous, interceptable before commit | Async, fire-and-forget after commit |
| Blockable | Yes | No |
| Examples | `CommitScheduleItem`, `SubmitDecision` | `DriftDetected`, `SemesterRolledOver` |

Registered interceptors, fixed order: **Deep Work Guard → Decision
Challenge Layer → Divergence Check**. Interceptors never return a bare
boolean — only `Clear` or `RequiresConfirmation` with a mandatory
`reasoning` string.

The **Decision Challenge Layer**, concretely: user submits a decision →
the interceptor evaluates it hypothetically against current
drift/bottleneck state → if it trips a rule, a single blocking
`ChallengeDialog` (see §1.3 for why this is the one deliberate
interruption) shows a plain-language, domain-grounded challenge → the
user confirms, revises, or cancels → recorded in `decisions.final_outcome`
→ **never re-challenged**.

Every event, whether subscribed to or not, is persisted to `event_log` —
the system's behavior must be reconstructable years later.

`DriftScan` runs on a daily timer, not an event trigger, because drift is
a trend property that can't be detected from any single event.

**Failure semantics:** commands fail closed (a blocked interceptor means
nothing commits); events fail open per-subscriber (one subscriber's
failure — e.g. an LLM timeout — never blocks another, like event-log
persistence, or blocks the user from logging a grade).

### 4.7 Non-Functional Requirements

- 5-year maintainability by a single developer.
- Offline-first: priority resolution, logging, and trajectory viewing all
  work with zero network access. Only LLM synthesis and Codeforces sync
  need connectivity, and both degrade gracefully.
- Data durability: single SQLite file + rolling local timestamped
  backups. No sole reliance on cloud backup.
- Auditable reasoning: every recommendation must be explainable after the
  fact.
- Cold-start correctness: at the start of a semester with mostly empty
  data, the system says "insufficient data," never guesses.

### 4.8 Presentation Surface

Four screens, no more (§5 has full detail): **Now**, **Trajectory**,
**Semester Setup**, **Decision Log**. No settings sprawl, no
notification-preference matrix, no dashboard builder.

---

## 5. UI Philosophy

### 5.1 Governing Rule

*"If I glance at this for one second while overwhelmed, do I know what to
do next?"* — kept verbatim from the UI team's own test, because it's the
right test.

The UI team's **visual and interaction language is adopted almost
entirely as designed**: neutral low-saturation palette with one accent
color reserved for "the one thing that matters," muted category colors,
one typeface family at two weights, numbers as the largest thing on any
screen, generous spacing, a hard "max 5 visible items before collapse"
rule, slow physical motion (never bouncy/urgent-reading), undo on
reversible actions, no streaks/badges/leaderboards, batched notifications
rather than a constant drip, and the density toggle (Calm / Detail) per
screen. All of this stands.

Two things are corrected, per §1.2 and §1.3: **severity is always
visually distinguishable** (a real `urgent` bottleneck or a real 48-hour
internship deadline is never rendered identically to "everything's
fine"), and **the Challenge Dialog and Deep Work Guard override are the
two named exceptions to "no modals."**

### 5.2 The Four Screens (Reconciled)

**Now** (was: "Home Screen," folds in Dashboard's "Next Deadline" and
"Quick Wins" cards)
The default screen. Answers one question: *what's the one thing right
now?* The dominant element is **not** the next chronological calendar
item — it is the Priority Resolution engine's ranked answer: verdict,
one-sentence reasoning, confidence badge. Below it: the current
bottleneck (if any), any active drift banner, and — only if the Decision
Challenge Layer has fired — the blocking `ChallengeDialog`. A soft
Load/Focus state (Light/Steady/Full) may still appear as a secondary,
non-blocking visual, but it never substitutes for the ranked
recommendation as the primary element. "Quick Wins" as a
task-clearing prompt is cut (§1.7, §1.4 — it re-introduces proxy-metric
gaming); the deep-work allocation for tonight's window is shown here
instead, since that's the actual highest-leverage prompt for the evening.

**Trajectory** (folds in Dashboard's course/career rings, Analytics,
Semester View's density levels, and Career View)
CGPA trend, DSA/Codeforces trend, project/research status, all as time
series against target lines, at three zoom levels (week / month /
semester — reusing the UI docs' pinch-to-zoom and swimlane visual
language, which is genuinely good). Career/internship threads live here
as one section, not a separate screen — with real `apply_by` urgency
rendered honestly (§1.2), not suppressed for calmness. The "gentle
mirror, never anything that could make the user feel bad" framing from
`ANALYTICS.md` is replaced with: patterns are shown factually, phrased
without editorializing or shame, but never hidden. Muted color language
(the amber dot, the "still here" tag for missed items) is kept as the
*style* of honesty, not a replacement for it.

**Semester Setup** (unchanged from Architecture's spec; absorbs Semester
View's "Big Picture" phase strip as a visual once setup is complete)
The re-derivation wizard run at the start of each term: courses,
deadlines (CSV/ICS import), timetable, deep-work window confirmation.
Explicitly manual-but-structured, run every semester — never a stale
live-sync silently carrying over last semester's structure.

**Decision Log** (unchanged; the UI docs never designed this screen at
all — an omission, now filled using the same visual language as the
other three)
The historical record of decisions, challenges issued, and how they
resolved. Uses the same card/timeline visual language as Trajectory.

### 5.3 What Was Cut, and Where Its Good Ideas Went

- **Daily Planner** and **Weekly Planner** as standalone screens are cut
  (§1.4 — they reintroduce a general calendar/task manager, which
  `VISION.md` and `PROJECT_SCOPE.md` explicitly rule out). Their genuinely
  useful patterns — the pinned "current item" card, the max-5-rows
  collapse rule, drag-to-reschedule *for entities that already exist in
  the schema* (a deadline, a deep-work allocation) — are folded into the
  **Now** screen's expanded/Detail view.
- **Dashboard** as a standalone 4–6-card screen is cut; its individual
  cards are redistributed into Now (next deadline, quick wins → replaced)
  and Trajectory (course health, career thread).
- **Quick Capture** (freeform, uncategorized task jotting, "auto-sorted
  later") is cut as designed. If a quick-add affordance exists at all, it
  must create one of the schema's existing typed entities (a deadline, a
  project milestone, a DSA log entry) directly — no ungrounded "AI will
  sort it out later" step, because nothing in the AI pipeline is
  specified to do that safely.
- **Mood/Energy tap-scale logging** ("Energy Check," "Load Indicator" as
  a feelings gauge) is cut from core scope — see §11. It has no backing
  table, no consumer in the domain layer, and no basis in
  `USER_PROFILE.md`. It's a plausible future feature (§10), not a v1
  screen element.

---

## 6. AI Philosophy

### 6.1 The One Sentence That Governs Everything

**The LLM is a writer, not a decider.** Every fact, verdict, ranking, and
severity level is produced by deterministic Rust before any LLM is
called. The LLM's only job is to turn an already-decided, already-cited
verdict into a clear, well-reasoned sentence.

### 6.2 Pipeline

```
Trigger → Retrieval (grounded facts + freshness stamps, athena-data)
       → Deterministic Scoring (priority / bottleneck / drift /
         divergence, pure Rust, produces a typed verdict + confidence +
         evidence rows — no LLM)
       → Synthesis (LLM turns the verdict into prose, using only the
         supplied facts, citing stable IDs)
       → Grounding Check (every cited ID is verified against the
         retrieval payload; any unverified claim → reject and retry once
         with a stricter prompt; second failure → template-flattened,
         prose-free output)
       → Output (a `recommendations` row: verdict, reasoning, confidence,
         grounded_in, data_freshness_note — confidence is never nullable)
```

The worst-case failure mode of this pipeline is a template-flattened,
fact-only answer — never a fluent hallucination.

### 6.3 Confidence Model

Three classes: `confirmed` (follows directly from fresh retrieved data),
`inferred` (follows from a trend/pattern read — explicitly labeled a
hypothesis in the UI), `insufficient_data` (a first-class, expected state
at the start of a semester — never papered over with a generic answer).

### 6.4 Cadence (from the AI Design docs, re-grounded)

The AI Design docs' daily/weekly/semester rhythm is correct and kept —
its *conversational* implementation is not. Re-implemented as:

- **Daily** — a lightweight, non-conversational check: does the Now
  screen's recommendation still hold given anything ingested since
  yesterday (a Codeforces sync, a grade snapshot)? No required user
  interaction; this is the `DriftScan`-adjacent daily pass, not a
  "morning briefing" chat.
- **Weekly** — the `DriftScan` job's output accumulates into a Trajectory
  view update and, if a pattern crosses the recurrence threshold, a
  surfaced `drift_signal`. Reviewed by the user opening **Trajectory**
  whenever he chooses — not a scheduled 10–15 minute mandatory exchange.
- **Semester** — the **Semester Setup** wizard, run at natural term
  boundaries, is where goals are explicitly re-affirmed or revised
  against the closing semester's actual `user_profile_history`,
  `bottlenecks`, and `drift_signals` — the "re-founding" idea from
  `SEMESTER_AGENT.md` survives, executed as a structured wizard step
  (compare stated goals vs. `grade_snapshots`/`codeforces_snapshots`
  trend, confirm/revise), not a facilitated conversation.

No cadence requires the user to type an answer to a daily or weekly
question. Optional structured logging (e.g., "what did you actually work
on tonight" as a one-tap selection from open deadlines/bottlenecks, not a
text box) feeds `deep_work_sessions.actual_activity` when the user
chooses to provide it; it is never required for the pipeline to function.

### 6.5 Signal Threshold (merged into the confidence/severity model)

A candidate observation graduates from "logged silently" to "surfaced"
only if at least two of the following hold, computed deterministically in
`athena-domain`, not judged by the LLM:

- **Recurrence** — this pattern has appeared 2–3+ times across the
  relevant window (mirrors `CORE_PRINCIPLES.md` #7's "sustained
  deviation").
- **Stakes** — the evidenced cost (grade impact, deadline proximity,
  portfolio relevance) crosses a defined threshold.
- **Reversibility** — the window to act is closing (an `apply_by` date, a
  deadline).
- **Contradiction** — it conflicts with a decision or goal the user
  explicitly committed to (`decisions`, `user_profile_history`).

This is the same mechanism that already governs `drift_signals.severity`
and the Priority Resolution "single answer vs. closeness-threshold list"
behavior — the AI Design docs' Signal Threshold and the Architecture
team's severity/confidence system were solving the same problem twice;
this is the one implementation.

### 6.6 Persona (kept from the AI Design docs, applied to the Stage 4 prompt)

Direct, economical, respectful of the user's time. No performed
enthusiasm, no hedging a disagreement into mush. Athena is allowed to say
"I think this is a mistake, here's why" — once, with evidence — and then
respects the decision unless the same failure recurs. It never moralizes,
never nags, and — per Non-Negotiable §1 — never softens a negative
verdict for comfort. This is a **tone constraint on the synthesis prompt**,
not a separate reasoning system: it governs *how* Stage 4 phrases a
Stage 3 verdict, nothing more.

### 6.7 Model Choice

Hybrid: a cloud LLM (Claude, via the Anthropic API) is the primary path
for Stage 4 synthesis quality; a local model (via Ollama or equivalent)
is a first-class, not an afterthought, fallback for offline use and
5-year vendor-independence. Only the narrow Stage-2 retrieval payload for
a given synthesis call ever leaves the device — never a database dump,
never raw identifiers beyond what phrasing requires.

### 6.8 What This Pipeline Deliberately Does Not Do

- Does not let the LLM call any state-mutating tool. Synthesis is
  read-only; all writes go through Commands.
- Does not maintain open-ended conversational memory as the primary mode.
  A follow-up "why?" chat surface may exist, re-running Stage 4 with the
  same Stage 2/3 payload plus the question — but the product's value
  never depends on the user initiating conversation.
- Does not fine-tune or retrain on the user's data — negative ROI at
  single-user scale, and it would create exactly the opaque,
  hard-to-audit behavior this whole pipeline exists to avoid.
- Does not infer psychological state, diagnose patterns the user hasn't
  evidenced in structured data, or maintain a subjective "credibility"
  judgment of the user's character. If a pattern is real, it shows up in
  `drift_signals` or `bottlenecks` with evidence rows — never as a vibe.

---

## 7. Database

Single SQLite file. No multi-tenancy — enforced by the *absence* of a
user dimension, not a `WHERE user_id = 1` convention.

### 7.1 Design Rules

1. **Snapshots over overwrites** — anywhere a value changes over time,
   store a time series. Trend detection is impossible otherwise.
2. **Semester as a thread, not an assumption** — `semester_id` is a
   foreign key on nearly every table; no table encodes a fixed weekly
   structure as global truth.
3. **Every recommendation is accountable** — `recommendations` and
   `decisions` exist so the "why" behind any past nudge is reconstructable.

### 7.2 Core Tables (summary — full column definitions carry over verbatim from the Architecture team's `DATABASE_SCHEMA.md`, which is adopted as-is)

`semesters` · `courses` · `grade_snapshots` · `deadlines` ·
`dsa_practice_log` · `codeforces_snapshots` · `projects` /
`project_status_snapshots` · `research_activities` · `deep_work_sessions`
· `bottlenecks` (no `resolved_by_inactivity` state, ever) · `drift_signals`
· `opportunities` · `decisions` · `recommendations` · `data_sources` ·
`user_profile` / `user_profile_history` · `event_log` (append-only audit
trail).

### 7.3 Explicitly Rejected Tables

- No `tasks` table for arbitrary to-dos.
- No `streaks` / `badges` / `points` tables.
- No `weekly_template` table.
- No `shared_with` / `collaborators` columns anywhere.
- **No memory-system tables** (`episodic_memory`, `semantic_memory`,
  `habit_memory`, `tension_flags`, `credibility_ledger`) — per §1.1 and
  §1.5, the existing snapshot/decision/drift tables already are Athena's
  memory; a second, LLM-native shadow memory model is not built.
- No `mood_log` / `energy_log` table in v1 — per §5.3 and §10.

### 7.4 Proxy vs. Trajectory Metrics

Kept exactly as specified: proxy metrics (`deadlines.status = 'done'`,
`dsa_practice_log.problems_attempted`) and trajectory metrics
(`grade_snapshots.*`, `codeforces_snapshots.rating`,
`project_status_snapshots.portfolio_strength_score`) are never conflated
in a query without deliberate joining; `DivergenceCheck` is the only code
path allowed to compare them.

### 7.5 Migration Philosophy

Additive-only. A column is deprecated, never dropped or repurposed
in-place across a semester boundary — a destructive migration mid-history
would corrupt the long-horizon trend data the system exists to preserve.

---

## 8. Integrations

Governing rule: **does this make the system's facts more grounded, or
does it just add convenience at the cost of fragility/privacy?**
Convenience alone never justifies a new outbound dependency.

| Integration | Shape | Why |
|---|---|---|
| **Codeforces public API** | Read-only polling (`user.rating`, `user.status`) | Stable, documented, public, maps to a trajectory metric. On failure, data is flagged stale, never silently treated as current. |
| **LLM provider (Anthropic Claude, primary)** | Narrow per-call payloads, JSON-schema-constrained output | Synthesis only (§6); risk is bounded because it cannot introduce facts. |
| **Local LLM (Ollama or equivalent), fallback** | Localhost-only, no egress | Removes hard dependency on one vendor for 5-year survivability. |
| **Institute timetable/grades — no live integration** | CSV/ICS import through Semester Setup | No public API exists; a scraper against a private, unversioned system is the highest-maintenance, most brittle thing a 5-year single-developer project could build, and would require storing institute credentials — a security liability disproportionate to the value. Deliberately manual, re-run every semester (matches §3.1 non-negotiable §7). |
| **Calendar (.ics) import** | Local file parse, one-time per semester | Not a live sync — no OAuth, no standing dependency. |
| **OS notifications/tray** | Native APIs, cross-platform via Tauri (§1.6) | Every notification is a delivery channel for a typed `Recommendation`/`Alert` object — never a raw string constructed ad hoc. |

**Explicitly never built:** cloud backup/sync service integration
(Dropbox, OneDrive, etc. as a first-class feature — the user may put
`backups/` under their own sync tool entirely outside the app's
awareness); third-party task-manager sync (Notion, Todoist — nothing to
serve, since general task management is out of scope); social/sharing
integrations; analytics/telemetry SDKs of any kind.

### 8.1 Network Access Summary

| Destination | Direction | Can the app function without it? |
|---|---|---|
| Codeforces API | Outbound, read-only | Yes — degrades to stale-flagged data |
| Anthropic API | Outbound | Yes — falls back to local model, then template output |
| Local model server | Localhost only | Yes — this *is* the no-network fallback |
| Anything else | None | — |

---

## 9. Development Roadmap

Phased so that every phase ships something independently useful and
testable — no phase depends on a later phase's LLM or ingestion work to
be minimally valuable.

### Phase 0 — Foundation (infrastructure, no product value yet)
- Cargo workspace + crate boundaries (`athena-domain`, `athena-data`,
  `athena-events`, `athena-reasoning`, `athena-ingestion`, `athena-app`).
- SQLite schema + migration runner; all core tables from §7.2.
- Tauri shell boots, IPC chokepoint (`ipc/`) established, empty screens
  render.

### Phase 1 — Manual Core Loop (usable without any AI or integrations)
- `Semester Setup` wizard: manual course/deadline entry (CSV/ICS import
  can land later in this phase).
- `deadlines`, `grade_snapshots`, `dsa_practice_log` manual entry.
- **Priority Resolution** (deterministic, no LLM) — the single most
  important algorithm in the product — ships first, tested to its >90%
  branch-coverage bar.
- `Now` screen renders the Priority Resolution answer with a
  template-only (no-LLM) reasoning string.
- **Deep Work Guard** hard-block, enforced on `CommitScheduleItem`.

*Milestone 1: the user can run a full week on Athena with zero AI and
zero external integrations, and it already changes what he works on in
the 8 PM–midnight window.*

### Phase 2 — Grounded AI Synthesis
- `athena-reasoning`: retrieval, Stage 4 synthesis (cloud LLM), Stage 5
  grounding check, confidence labeling.
- `recommendations` table fully populated; Now screen shows
  LLM-synthesized reasoning instead of template text.
- Local-model fallback path wired in (even if initially lower priority
  than getting the cloud path solid).

*Milestone 2: recommendations read like a Chief of Staff wrote them, and
every claim is checkably grounded.*

### Phase 3 — Drift, Bottlenecks, and the Challenge Layer
- `bottleneck/`, `drift/`, `divergence/` domain modules.
- Scheduled `DriftScan` job.
- Decision Challenge Layer interceptor + blocking `ChallengeDialog`.
- `Decision Log` screen.

*Milestone 3: Athena can now say "I think this is a mistake" before a bad
decision commits, and can catch a slipping subject a semester early.*

### Phase 4 — External Grounding
- Codeforces sync connector.
- `Trajectory` screen fully built out (multi-metric time series, three
  zoom levels).
- Data-source staleness handling end-to-end.

*Milestone 4: trajectory claims are backed by live competitive-programming
data, not just self-reported logs.*

### Phase 5 — Opportunity Surfacing and Hardening
- `opportunities` table + surfacing logic (deterministic query, per §1.1's
  correction of the Opportunity Engine concept).
- Rolling local backups.
- Cross-platform notification/tray polish (Windows/macOS/Linux — §1.6).
- Full offline-first audit: confirm every core function works with zero
  network access.

*Milestone 5: the product is feature-complete against this specification
and durable enough to trust with multiple semesters of history.*

Each phase is independently shippable and independently useful — Phase 1
alone is already meaningfully better than a to-do list, because it's the
only phase that requires the Priority Resolution algorithm, the single
load-bearing piece of the whole system, to exist and be correct.

---

## 10. Future Features (deferred, not rejected — revisit once the core loop is proven)

- **Deterministic credibility ledger** — an override-rate-per-decision-type
  computed by SQL over `decisions.final_outcome`, surfaced as a light
  calibration signal for how hard the Challenge Layer pushes in a given
  decision category. Only if it can be built as a transparent, inspectable
  computation — never an LLM-graded judgment (§1.7).
- **Mood/energy logging** — a single-tap, no-text state log, *if* it can
  be wired to a real domain consequence (e.g., correlating logged state
  with `deep_work_sessions.protected` rate) rather than existing as
  decoration. Needs its own schema design and its own justification
  against a non-negotiable before it's built, not before.
- **Follow-up chat surface** — a narrow "why?" conversational mode that
  re-runs Stage 4 with the existing Stage 2/3 payload plus the user's
  question. Secondary interaction mode only, never load-bearing.
- **Cross-device sync** (still self-hosted/user-owned, e.g. the user's own
  sync tool over the SQLite file or an end-to-end-encrypted personal
  relay) — only if the single-machine constraint becomes a real friction
  point, and only without introducing Athena-operated cloud
  infrastructure, which would violate §3.1 non-negotiable §8.
- **Institute portal integration**, if the institute ever ships a public,
  documented API — revisit §8's rejection at that point, not before.

---

## 11. Explicitly Rejected Features

- **The four-type LLM Memory System** (episodic / semantic / procedural /
  decision, with decay, reinforcement, and "distillation") — §1.1, §1.5.
- **LLM-inferred habit, weakness, and "blind spot" detection** — an LLM
  concluding something about the user's psychology that the user hasn't
  evidenced in structured data is a guess presented as fact, which §3.1
  non-negotiable §5 forbids outright — §1.1.
- **LLM-graded "credibility ledger"** as a subjective judgment of the
  user's decision-making track record — §1.1, §1.7. (A deterministic,
  SQL-computed version is a Future Feature, not this.)
- **Mandatory conversational daily/weekly check-ins** ("morning
  briefing," "evening debrief," 10–15 minute weekly review) — contradicts
  "Athena pushes, you don't have to pull" and reintroduces the exact
  decision fatigue the product exists to remove — §1.1.
- **Seven-screen information architecture** (Home Screen, Dashboard,
  Daily Planner, Weekly Planner, Semester View, Career View, Analytics as
  separate screens) — collapses to four; §1.4, §5.
- **General task manager / freeform "Quick Capture"** — explicitly out of
  scope since the original `PROJECT_SCOPE.md`; reintroducing it as an
  "auto-sorted" capture box does not change that — §1.4, §5.3, §7.3.
- **"Never show anything that could make the user feel bad"** as an
  absolute UI rule — directly contradicts non-negotiables §1 and §6;
  restraint in *style* is kept, suppression of real signal is not — §1.2.
- **"No modals, ever"** as an absolute rule — narrowed to a real exception
  for the Challenge Dialog and Deep Work Guard override, which structurally
  require a blocking moment to exist at all — §1.3.
- **Gamification of any kind** (streaks, badges, points, leaderboards) —
  a direct proxy-metric violation of non-negotiable §9.
- **Cloud backup/sync as a first-class Athena feature, third-party task
  manager sync, social/sharing integrations, analytics/telemetry SDKs** —
  §8.
- **Live scraping of the institute's student portal** — brittle,
  credential-risky, disproportionate to the value — §8.
- **Windows-only assumption** baked into architecture/integration docs —
  not a "feature" per se, but explicitly struck as an unjustified
  narrowing of the platform target — §1.6.

---

## 12. Engineering Guidelines

1. **The domain layer (`athena-domain`) has zero dependencies on Tauri,
   SQLite, the network, or the LLM.** Any PR that adds one is a
   review-blocking violation, not a style nit.
2. **The LLM never decides, only phrases.** Any change that lets a
   synthesis call introduce a new fact, ranking, or severity not already
   present in the Stage 2 retrieval payload is rejected at grounding-check
   design time, not caught later in testing.
3. **Every user-facing surfaced item is a typed `Recommendation` or
   `Alert` object with a mandatory `reasoning` field.** There is no code
   path that constructs a bare notification string.
4. **Migrations are additive-only.** No column is dropped or repurposed
   in place across a semester boundary.
5. **Test bar:** `athena-domain`, especially `priority/`, holds the
   highest coverage requirement in the codebase (>90% branch coverage) —
   it's the one piece of logic that must never silently drift across a
   refactor or model upgrade.
6. **Every architectural or product decision that isn't purely
   cosmetic must cite which section of this document (or a deliberate,
   written revision to it) justifies it.** This is how the project avoids
   regenerating the exact conflict this review just resolved — a future
   contributor (including a future AI collaborator) designing a screen,
   an engine, or a table without checking it against this document first.
7. **When in doubt, cut, don't add.** Every screen, table, and engine in
   this document earned its place by surviving a "does this reduce a
   decision, or just look impressive" test. New proposals are held to the
   same bar, not a lower one because the product already "has enough
   stuff now."
8. **Offline-first is a testable property, not an aspiration.** CI should
   include a mode that runs the core loop (logging, viewing trajectory,
   priority resolution) with network access disabled entirely.
9. **No feature ships with an implicit new table.** If a UI or AI design
   proposal requires data that doesn't exist in §7's schema, the schema
   change is the first deliverable, reviewed on its own, before any UI or
   prompt work references it.
