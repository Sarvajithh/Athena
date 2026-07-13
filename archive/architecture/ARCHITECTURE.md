# ARCHITECTURE.md — Project Athena

## 0. What This Document Is

This is the top-level architectural decision record for Athena. It exists
so that a future maintainer — including a future version of the user —
can open this file and understand not just *what* was built, but *why*,
without re-deriving the reasoning from scratch. Every major decision below
is traceable to a line in `VISION.md`, `CORE_PRINCIPLES.md`,
`NON_NEGOTIABLES.md`, `PROJECT_SCOPE.md`, or `USER_PROFILE.md`. Where a
decision is purely technical (e.g. "which SQLite driver"), it's still
justified against the 5-year survivability requirement, not just current
convenience.

If a future change conflicts with something in this document, the burden
of proof is on the change, per `PROJECT_SCOPE.md`'s resolution rule:
conflicts resolve in favor of the five foundational documents unless those
documents are deliberately revised first.

## 1. Architectural Style

**Athena is a modular monolith running inside a Tauri desktop shell, with
an in-process event bus connecting independently-testable modules, backed
by a single local SQLite database as the sole source of truth.**

This is a deliberate rejection of two more "modern-looking" alternatives:

- **Microservices** — wrong shape for a single-user, single-machine,
  offline-capable system. Network boundaries between modules would add
  latency, deployment complexity, and failure modes (partial availability,
  service discovery, versioning skew) that buy nothing when there is
  exactly one user and one process. A modular monolith gives the same
  separation-of-concerns benefit (independent modules, testable in
  isolation, replaceable individually) without the operational tax.
- **A thin client talking to a cloud backend** — directly violates
  `NON_NEGOTIABLES.md` §8 (privacy and sole ownership) and the local-first
  constraint. Athena's data — CGPA, weaknesses, drift, deadlines — is the
  most sensitive personal performance data the user has. It lives on disk,
  under the user's control, full stop. Any network calls (LLM inference,
  Codeforces sync) are outbound, opt-in per call, and never a dependency
  for the app to function.

Why a monolith survives 5 years better than it sounds like it should:
the failure mode of long-lived single-developer software is usually not
"it doesn't scale," it's "nobody can hold the whole system in their head
anymore and every change becomes archaeology." A monolith with **hard
internal module boundaries enforced by Rust's crate/module system and an
event bus instead of direct cross-module function calls** gets you
microservice-grade decoupling with monolith-grade operational simplicity.
See `MODULES.md` and `EVENT_SYSTEM.md` for how that boundary is enforced.

## 2. Technology Stack and Why

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri** | Windows desktop app with a native, small-footprint binary (Rust core, no bundled Chromium runtime like Electron), direct OS integration (notifications, filesystem, autostart) via typed Rust commands instead of a Node.js backend. Given "beautiful UI" + "AI-first" + "5 years," we want a UI layer that can be restyled/rebuilt without touching the reasoning engine — Tauri's IPC boundary forces that separation structurally, not by convention. |
| Frontend | **React + TypeScript** | Given constraint. Used *only* for presentation and light client-side state — never for domain logic, scoring, or reasoning (see §4). This keeps the UI replaceable in isolation; a 5-year-old React app is often the first thing a maintainer wants to modernize, and it should be safe to do that without touching the parts of the system that encode the user's actual goals. |
| Core domain logic | **Rust**, inside `src-tauri` | Bottleneck detection, drift scoring, deep-work protection rules, and the metric-gaming divergence check (`NON_NEGOTIABLES.md` §9) are the parts of this system that most need to be *correct* and *stable*. Rust's type system makes illegal states (e.g. a recommendation with no grounding reference) unrepresentable rather than merely discouraged. This logic is deliberately kept out of TypeScript so it can never quietly regress when the UI is reworked. |
| Database | **SQLite** (via `rusqlite`/`sqlx`), single file | Given constraint; also the correct choice independently — zero-ops, single-file, trivially backed up (`NON_NEGOTIABLES.md` §8, sole ownership means the user should be able to point at one file and say "that's my data"), and more than capable of the query load of one user's academic/career history over a degree and beyond. |
| AI inference | **Hybrid: cloud LLM (primary) + local model (optional fallback)** | See `AI_PIPELINE.md` for the full design. Summary: reasoning quality for synthesis (turning computed signals into a ranked, justified recommendation in natural language) currently favors frontier cloud models, but every field sent off-device is minimized and the *deterministic scoring* (what's actually a fact) never leaves Rust. A local-model path (e.g. via Ollama) is a first-class fallback, not an afterthought, because a 5-year system with a hard cloud dependency is a system that breaks the day a vendor deprecates an API. |
| Internal messaging | **In-process event bus** (Rust, e.g. `tokio::sync::broadcast` + a command dispatcher) | Not an external message queue (Kafka, RabbitMQ, etc.) — there is one process and one user; external MQ infrastructure would be pure overhead. See `EVENT_SYSTEM.md`. |

## 3. Why This System Cannot Be "Just CRUD With a Chatbot Bolted On"

This is the most important architectural judgment call in the whole
project, so it gets its own section.

The obvious, wrong architecture for "AI-first personal assistant" is:
a database of tasks/deadlines, a chat window, and a system prompt that
tells an LLM to "be helpful." That architecture fails almost every
constraint in `NON_NEGOTIABLES.md`:

- It cannot guarantee grounding (§5) — an LLM free-associating over a
  loosely-described state can and will hallucinate trajectory claims.
- It cannot reliably distinguish fact from inference (§10, `CORE_PRINCIPLES.md`
  #10) — that requires the *scoring* to happen outside the LLM, with the
  LLM used only for synthesis and explanation of numbers it didn't invent.
- It cannot enforce the deep-work block (§3) — a chatbot is reactive by
  construction; it cannot refuse to let something be scheduled unless it's
  wired into the write path itself.
- It cannot implement the Challenge Layer (§4, `CORE_PRINCIPLES.md` #3) —
  challenging a decision *before* it's committed requires the reasoning
  engine to sit on the write path as an interceptor, not as a
  side-conversation the user can ignore.

Athena's architecture is instead: **deterministic Rust scoring produces
the facts and flags → the event/command system enforces where those facts
get to intervene (blocking writes, generating recommendations, escalating
drift) → the LLM's only job is to turn already-computed, already-grounded
signals into a well-reasoned sentence.** The LLM is a *writer*, not a
*decider*. This single decision is what makes almost every
`NON_NEGOTIABLES.md` clause enforceable in code rather than aspirational
in a prompt.

## 4. Layering

```
┌─────────────────────────────────────────────────────────┐
│  Presentation (React + TS)                               │
│  "Now" view · Trajectory view · Bottleneck view ·         │
│  Semester Setup · Decision Log — thin, stateless-ish,      │
│  talks to Rust ONLY via typed Tauri commands               │
└───────────────────────▲───────────────────────────────────┘
                         │ typed IPC (Tauri commands/events)
┌───────────────────────┴───────────────────────────────────┐
│  Application Layer (Rust)                                  │
│  Command handlers · Query handlers · Tauri command bindings│
└───────────────────────▲───────────────────────────────────┘
                         │
┌───────────────────────┴───────────────────────────────────┐
│  Domain Layer (Rust, pure — no I/O)                         │
│  Bottleneck detection · Drift scoring · Deep-work guard ·   │
│  Metric-divergence check · Priority resolution rules        │
└───────────────────────▲───────────────────────────────────┘
                         │
┌───────────────────────┴───────────────────────────────────┐
│  Reasoning/AI Layer (Rust orchestrator + LLM client)         │
│  Retrieval → grounding → synthesis → confidence labeling     │
└───────────────────────▲───────────────────────────────────┘
                         │
┌───────────────────────┴───────────────────────────────────┐
│  Event Bus (in-process)                                     │
│  Commands (interceptable, synchronous) · Events (async,     │
│  fire-and-forget notifications) — see EVENT_SYSTEM.md        │
└───────────────────────▲───────────────────────────────────┘
                         │
┌───────────────────────┴───────────────────────────────────┐
│  Data Layer (Rust repositories) → SQLite (single file)       │
│  Ingestion connectors (Codeforces, ICS import, CSV import)    │
└─────────────────────────────────────────────────────────────┘
```

The domain layer has **zero dependency on Tauri, SQLite, or the network**.
It is pure functions and data structures. This is the single highest-value
decision for 5-year survivability: it means the domain logic (what counts
as drift, what counts as a bottleneck, what the deep-work guard allows) can
be unit tested with no infrastructure, and can outlive a full rewrite of
the persistence layer, the UI framework, or the LLM vendor.

## 5. Principle → Mechanism Traceability

Every non-negotiable and core principle maps to a specific, checkable
architectural mechanism. This table is the contract between the philosophy
docs and the system.

| Source | Requirement | Architectural Mechanism |
|---|---|---|
| NON_NEG §1 | Trajectory over comfort; must say unwelcome things | Recommendation objects carry a `verdict` field independent of tone; the synthesis prompt is explicitly forbidden from softening a negative `verdict` (see AI_PIPELINE.md §4) |
| NON_NEG §2 | Never a bare reminder | `Recommendation` and `Alert` are the *only* user-facing surfaced types; there is no `Notification` type without a mandatory `reasoning` and `tradeoff` field. Enforced by the domain type system — a bare reminder is not a constructible value. |
| NON_NEG §3 | Deep work block is sacred | `DeepWorkGuard` domain rule sits on the `CommitScheduleItem` command path; any write touching 20:00–00:00 with a low-leverage classification is intercepted and requires explicit `override_confirmed=true` from the caller (see EVENT_SYSTEM.md §3) |
| NON_NEG §4 | No silent unilateral action | All state changes to deadlines, decisions, and schedule commitments flow through **Commands**, not direct DB writes; Commands are interceptable and always terminate in a UI confirmation for irreversible/high-stakes actions |
| NON_NEG §5 | Grounded in reality, never vibes | Every `Recommendation` row has a mandatory `grounded_in` set of foreign keys into source tables (deadlines, cgpa_snapshots, etc.); the LLM client rejects any synthesis output that references a claim not present in the retrieval payload (AI_PIPELINE.md §5) |
| NON_NEG §6 | Weaknesses tracked honestly | `bottlenecks` table has no "resolved by inactivity" state — only `resolved_by_evidence`; a bottleneck can only leave the active set when a linked data point shows measurable improvement |
| NON_NEG §7 | Adapts to semester, not reverse | `semesters` is a first-class table; almost every other table carries a `semester_id` FK; there is no global "weekly template" table anywhere in the schema (DATABASE_SCHEMA.md §2) |
| NON_NEG §8 | Privacy, sole ownership | No multi-user tables (no `users` table — single row config instead); no cloud sync service; all network calls are outbound-only, opt-in, logged (API_INTEGRATIONS.md) |
| NON_NEG §9 | No metric gaming | Domain layer explicitly separates `proxy_metrics` (tasks completed, hours logged) from `trajectory_metrics` (CGPA, rating, portfolio strength); a dedicated `DivergenceCheck` rule runs whenever both move in opposite directions and raises a flagged event |
| NON_NEG §10 | Fail loud, not silent | `confidence` and `data_freshness` are non-nullable fields on every `Recommendation`; below a threshold, the presentation layer is contractually required to render the low-confidence/staleness banner (not optional styling — the field is what the UI switches on) |
| CORE #1–2 | Reduce the decision, always give a reason | Priority Resolution is the primary query the frontend calls; it always returns a single ranked answer object plus a one-sentence reason field, never a bare list type |
| CORE #3 | Challenge, don't just comply | Challenge Layer (a Command interceptor subscribed to `DecisionSubmitted`) — see EVENT_SYSTEM.md §4 |
| CORE #4 | Protect deep work like capital | Same mechanism as NON_NEG §3, reframed as an allocation optimizer rather than a hard block once triggered — see MODULES.md §Deep Work Allocator |
| CORE #5 | Semester volatility as first-class input | Same as NON_NEG §7 |
| CORE #6 | Trajectory over task completion | Same as NON_NEG §9 |
| CORE #7 | Early signal beats late correction | Scheduled `DriftScan` background job (event-driven, not user-triggered) — see EVENT_SYSTEM.md §5 |
| CORE #8 | Bottleneck-first thinking | `current_bottleneck` is a materialized, always-available query, not something recomputed ad hoc — treated as a first-class piece of system state |
| CORE #9 | Present options only when genuinely ambiguous | Priority Resolution algorithm returns a ranked list only when top-two candidates are within an explicit closeness threshold; otherwise returns a single answer |
| CORE #10 | Honest about confidence | Same as NON_NEG §10 |
| CORE #11 | Minimal surface, maximum signal | Frontend intentionally has a small, fixed set of views (§7); no generic "add a widget" extensibility model |
| CORE #12 | Build for who you're becoming | `trajectory_metrics` model includes durable-capital-weighted scoring (research exposure, mathematical maturity indicators) as distinct from short-term convenience metrics |

## 6. Non-Functional Requirements

- **5-year maintainability by a single developer.** Drives: modular
  monolith over microservices, Rust domain layer isolated from
  UI/infra churn, minimal external dependencies, no bespoke DSLs.
- **Offline-first.** The app must be fully usable — priority resolution,
  logging, trajectory viewing — with zero network access. Only LLM
  synthesis and Codeforces sync require connectivity, and both degrade
  gracefully (cached last-good recommendation + explicit staleness flag,
  per NON_NEG §10) rather than blocking the app.
- **Data durability.** Single SQLite file + automatic local timestamped
  backups on a rolling window (see FOLDER_STRUCTURE.md `backups/`). No
  reliance on cloud backup as the only copy.
- **Auditable reasoning.** Every recommendation must be explainable after
  the fact — which data it used, what rule fired, what confidence it had —
  because a system that tells the user uncomfortable things (NON_NEG §1)
  must be able to justify itself when questioned.
- **Cold-start correctness.** At the start of a new semester with mostly
  empty data, the system must say "insufficient data" rather than degrade
  to guessing — this is a first-class state, not an edge case.

## 7. Presentation Surface (High-Level Only)

Per CORE #11, the frontend is intentionally small:

1. **Now** — the priority resolution answer, the current bottleneck, and
   any active challenge/pushback. The default screen.
2. **Trajectory** — CGPA trend, DSA/Codeforces trend, project/research
   status, all as time series against target lines.
3. **Semester Setup** — the re-derivation wizard run at the start of each
   term (courses, deadlines import, timetable).
4. **Decision Log** — the historical record of decisions, challenges
   issued, and how they resolved (accepted, overridden, revised).

No settings sprawl, no notification-preference matrix, no dashboard
builder. Detailed screen/component structure belongs in a future UI
design pass, not in this architecture document.

## 8. What Was Deliberately Not Built

- No multi-user support of any kind, anywhere in the stack (NON_NEG §8).
- No gamification primitives (streaks, badges, XP) — explicitly out of
  scope per `PROJECT_SCOPE.md`, and would directly violate NON_NEG §9 by
  creating a proxy metric optimized for engagement rather than trajectory.
- No general task manager / arbitrary to-do list — scope stays bound to
  the academic/career trajectory unless deliberately extended.
- No generic chatbot surface as the primary interface — chat may exist as
  a secondary interaction mode for follow-up questions on a
  recommendation, but it is never the thing the user has to initiate to
  get value (VISION.md: "Athena pushes; you don't have to pull").
