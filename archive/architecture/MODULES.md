# MODULES.md — Project Athena

## 0. How to Read This Document

Each module below corresponds to a crate in `src-tauri/crates/` (see
`FOLDER_STRUCTURE.md` §2) or a top-level directory in `src/`. For each, this
document states: what it owns, what it explicitly does not own, what it
depends on, and which line in the foundational docs justifies its
existence as a *separate* module rather than being folded into another one.

The separation test used throughout: **a module boundary exists here only
if two different foundational-doc concerns would otherwise be tangled
together.** Modules are not split for their own sake — a smaller monolith
that's easy to hold in one head is preferable to over-fragmentation for a
single-developer 5-year project (see `ARCHITECTURE.md` §1).

## 1. `athena-domain` — Pure Reasoning Rules

**Owns:** the actual logic of what counts as a bottleneck, what counts as
drift, what the deep-work guard allows, and the proxy/trajectory
divergence check. Zero I/O.

**Sub-modules:**

- **Priority Resolution** (`priority/`) — implements `CORE_PRINCIPLES.md`
  #1 and #9. Takes a snapshot of current state (deadlines, bottleneck,
  time-of-day, deep-work window) and produces either a single ranked
  answer, or — only when two candidates are within a defined closeness
  threshold — a short ranked list. This is the load-bearing algorithm of
  the entire product; it is the only module with a mandated >90% branch
  test coverage bar, because a wrong answer here undermines the entire
  premise of the system (VISION.md: "closes the allocation gap").
- **Bottleneck Detection** (`bottleneck/`) — implements `CORE_PRINCIPLES.md`
  #8 and `NON_NEGOTIABLES.md` §6. Scans trajectory metrics for the single
  biggest current constraint and refuses to let it be silently dropped
  (no `resolved_by_inactivity` state, per `DATABASE_SCHEMA.md` §2).
- **Drift Scoring** (`drift/`) — implements `CORE_PRINCIPLES.md` #7.
  Trend-line detection over `grade_snapshots`, `dsa_practice_log`,
  `deep_work_sessions.protected`. Produces `drift_signals` rows with
  explicit evidence references — never a bare "something feels off."
- **Deep Work Guard** (`deep_work/`) — implements `NON_NEGOTIABLES.md` §3
  and `CORE_PRINCIPLES.md` #4. Two responsibilities: (a) a *hard guard*
  that intercepts any `CommitScheduleItem` command touching the 20:00–
  00:00 window with a `leverage_class = low` activity, and (b) an
  *allocator* that, within the protected window, picks the single
  highest-expected-return activity from open deadlines/bottlenecks/
  projects — never leaves the slot to "whatever feels easiest," per
  `CORE_PRINCIPLES.md` #4's explicit wording.
- **Divergence Check** (`divergence/`) — implements `NON_NEGOTIABLES.md`
  §9. The only code path allowed to compare proxy metrics against
  trajectory metrics (`DATABASE_SCHEMA.md` §5); raises a `drift_signal`
  when they move in opposite directions.

**Depends on:** nothing outside the Rust standard library and domain
value types. This is deliberate — see `ARCHITECTURE.md` §4.

**Why separate from `athena-reasoning`:** this module produces *facts and
verdicts*; `athena-reasoning` produces *prose explaining facts and
verdicts*. Keeping the LLM entirely out of `athena-domain` is what makes
`NON_NEGOTIABLES.md` §5 (grounded, never guessed) enforceable rather than
a matter of prompt discipline.

## 2. `athena-data` — Persistence

**Owns:** one repository per aggregate (`SemesterRepository`,
`DeadlineRepository`, `RecommendationRepository`, etc.), each exposing a
narrow, domain-shaped interface — never raw SQL leaking upward. Owns the
migration runner.

**Depends on:** SQLite via `sqlx`/`rusqlite`.

**Why separate:** isolates the *one* place in the codebase allowed to
write SQL, so a future change of storage engine (unlikely, but a 5-year
project should assume the unlikely eventually happens) touches one crate.

## 3. `athena-events` — Event Bus and Command Dispatch

**Owns:** the Command/Event distinction (see `EVENT_SYSTEM.md`), the
in-process pub/sub bus, and the registry of interceptors (including the
Challenge Layer, §5 below).

**Depends on:** `athena-domain` (to invoke rules during interception),
`athena-data` (to persist after a command clears interception).

**Why separate:** this is the mechanism that lets `NON_NEGOTIABLES.md` §4
("no decision made silently") be enforced structurally — every state
mutation is forced through a single chokepoint that can be intercepted,
rather than trusting every call site to remember to check.

## 4. `athena-reasoning` — AI Orchestration

**Owns:** retrieval of grounded facts, LLM prompt construction, response
validation (rejecting ungrounded claims), confidence labeling, and the
local-model fallback path. Full detail in `AI_PIPELINE.md`.

**Depends on:** `athena-domain` (consumes its verdicts as the *only*
source of facts to synthesize), `athena-data` (retrieval).

**Why separate from `athena-domain`:** the LLM call is the single least
deterministic, least testable part of the system, and the only part with
an external network dependency. Isolating it means a provider swap, a
prompt rewrof, or a move to local-only inference never touches the tested,
deterministic rules in `athena-domain`.

## 5. Decision Challenge Layer

Not a crate of its own — it is a specific **interceptor registered against
the event bus**, living in `athena-events` but invoking `athena-domain`
and `athena-reasoning`. Called out separately here because it's a named
concept in `CORE_PRINCIPLES.md` #3 and `PROJECT_SCOPE.md` §2.7, and its
behavior is distinct enough to warrant its own description:

1. User submits a `Decision` command (e.g. "move DSA practice to next
   week").
2. The interceptor runs the decision through `athena-domain`'s relevant
   rule (drift, bottleneck, deep-work guard) *before* the command is
   allowed to commit.
3. If the rule flags a conflict, `athena-reasoning` synthesizes a single,
   plain-language challenge — stated once, with reasoning, per
   `CORE_PRINCIPLES.md` #3's explicit "says so plainly, once."
4. The command blocks pending user confirmation. On confirm (with or
   without revision), it commits and the outcome is recorded in
   `decisions.final_outcome`. The system does **not** re-challenge the
   same decision twice — that would violate the "don't nag after you've
   decided" clause.

## 6. `athena-ingestion` — External Connectors

**Owns:** the Codeforces sync connector, ICS calendar import, CSV import
for institute timetables/grades (no scraping — see `API_INTEGRATIONS.md`
for why). Each connector writes to `data_sources` and stamps
`last_synced_at`.

**Depends on:** `athena-data`. Network access is isolated here — no other
module talks to the network directly except `athena-reasoning`'s LLM
client.

**Why separate:** ingestion code is the most likely to break over 5 years
(APIs change, formats drift) and the most likely to need frequent,
isolated patching without touching anything else.

## 7. Scheduler / Background Jobs

Lives inside `athena-app` (the Tauri binary crate), not its own crate,
because it's thin — it only owns *timing*, not logic:

- Periodic `DriftScan` trigger (implements `CORE_PRINCIPLES.md` #7 — early
  signal, not user-triggered).
- Staleness check on `data_sources` (implements `NON_NEGOTIABLES.md` §10).
- End-of-day deep-work session close-out prompt.

Each job fires an **event** (never calls domain logic directly) — see
`EVENT_SYSTEM.md` §5 — so the scheduler stays a dumb timer, and all actual
behavior stays testable in `athena-domain` without a clock dependency.

## 8. Frontend Modules (`src/`)

**Screens** (`Now`, `Trajectory`, `SemesterSetup`, `DecisionLog`) are the
only stateful React modules. **Components** are presentation-only and
receive fully-formed domain objects (a `Recommendation`, a `Bottleneck`)
as props — they never compute a verdict, confidence label, or ranking
client-side. **`ipc/`** is the single chokepoint to Rust (see
`FOLDER_STRUCTURE.md` §3).

**Why so few screens:** `CORE_PRINCIPLES.md` #11 — every additional screen
is cognitive tax. A module is added to the frontend only if it renders a
genuinely new *kind* of grounded object, not a new way of slicing existing
ones.

## 9. Module Dependency Rule (Enforced, Not Just Documented)

```
athena-app
   ├── athena-events ──┬── athena-domain
   │                    └── athena-data
   ├── athena-reasoning ── athena-domain, athena-data
   └── athena-ingestion ── athena-data

athena-domain depends on NOTHING internal.
```

Any Cargo dependency edge not in this diagram is a review-blocking
architectural violation. This is the concrete mechanism referenced in
`ARCHITECTURE.md` §1's claim that module boundaries are "enforced by
Rust's crate system," not aspirational.
