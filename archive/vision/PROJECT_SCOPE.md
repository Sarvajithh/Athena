# PROJECT_SCOPE.md — Project Athena

This document defines the operating boundary of the system: what Athena is
responsible for, what it explicitly is not, and where the edges of "Phase
1" sit. This is a living document — scope should expand deliberately, not
by accident.

## In Scope — What Athena Owns

### 1. Priority Resolution ("What should I work on now?")
The single most important function of the system. Given current deadlines,
current weaknesses, current trajectory, and current time-of-day, Athena
produces a ranked, justified answer — not a list to choose from.

### 2. Trajectory Tracking
- CGPA: current state, trend, and gap to the 8.8+ target, broken down by
  subject/semester where possible.
- DSA/Codeforces: practice volume, topic coverage, rating trend, and gaps
  against internship/competitive benchmarks.
- ML Projects: status, portfolio strength, and relevance to target roles.
- Research: current exposure, ongoing work, and relevance to Oxford/Imperial
  MSc admissions.

### 3. Deadline & Academic Calendar Awareness
Ingesting the current semester's actual structure (courses, exams,
assignment deadlines, project milestones) — refreshed every semester rather
than assumed static.

### 4. Deep Work Allocation
Actively protecting and allocating the 8 PM–midnight window to the
single highest-leverage activity available, and actively resisting its
erosion by lower-value tasks.

### 5. Weakness & Bottleneck Detection
Identifying, naming, and persistently tracking the current single biggest
bottleneck standing between the user and the long-term goals — whether
that's a weak subject, a stalled skill, or a missing credential.

### 6. Opportunity Surfacing
Actively surfacing relevant opportunities (internship openings, research
positions, competitions, application deadlines) that are relevant to the
long-term goals, evaluated against current trajectory — not a generic feed.

### 7. Decision Challenge Layer
A standing function that evaluates user decisions and plans against the
trajectory data and pushes back, with reasoning, when a decision appears
misaligned with stated goals.

## Out of Scope (Explicitly)

- **Not a general task manager.** Athena does not track arbitrary personal
  errands unrelated to the academic/career trajectory unless the user
  explicitly extends its scope.
- **Not a habit-tracking or gamification app.** No streaks, badges, or
  engagement mechanics — those optimize for app usage, not for trajectory,
  which violates CORE_PRINCIPLES #6.
- **Not a multi-user or collaborative tool.** No sharing, no team features,
  no commercial packaging — this is reinforced by NON_NEGOTIABLES #8.
- **Not a passive calendar/reminder replacement.** Any output resembling a
  bare reminder without reasoning is out of scope by definition
  (NON_NEGOTIABLES #2).
- **Not a general-purpose chatbot.** Athena's value is in what it proactively
  surfaces and decides, not in being a conversational interface the user
  has to interrogate.
- **Not a financial planning or unrelated life-admin system** in Phase 1 —
  scope stays tightly bound to the academic/career trajectory described in
  USER_PROFILE.md unless deliberately expanded later.

## Phase Boundary (Current Phase: Documentation & Definition)

This current phase is **specification only**: the five foundational
documents (VISION, NON_NEGOTIABLES, CORE_PRINCIPLES, USER_PROFILE,
PROJECT_SCOPE). No code, no architecture decisions, no tool selection has
been made yet. Any future phase (data model design, ingestion pipeline,
recommendation engine, interface design) should explicitly reference back
to these five documents as its source of truth, and any conflict between a
future design decision and these documents should be resolved in favor of
these documents unless the documents themselves are deliberately revised.

## Success Criteria for This Documentation Phase
- All five documents are internally consistent with each other (no
  contradiction between a NON_NEGOTIABLE and a CORE_PRINCIPLE, for
  example).
- Every principle and constraint is traceable back to a real detail in
  USER_PROFILE.md — nothing generic or copy-pasted from a "typical
  productivity app" mental model.
- A future builder (even a future version of the user) could pick up only
  these five files and correctly infer what Athena should and shouldn't do,
  without needing additional context.
