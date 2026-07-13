# DATABASE_SCHEMA.md — Project Athena

## 0. Design Philosophy

Three rules govern every table in this schema, each pulled directly from
the foundational documents:

1. **Snapshots over overwrites.** Anywhere a value changes over time
   (CGPA, Codeforces rating, bottleneck status), we store a time series,
   not a single mutable field. `CORE_PRINCIPLES.md` #7 ("early signal
   beats late correction") is architecturally impossible if the schema
   only remembers the current value — trend detection requires history.
2. **Semester as a thread, not an assumption.** `semester_id` is a foreign
   key on nearly every table. There is no table anywhere that encodes a
   fixed weekly structure as global truth. This is the direct schema-level
   enforcement of `NON_NEGOTIABLES.md` §7.
3. **Every generated recommendation is accountable.** The `recommendations`
   and `decisions` tables exist specifically so that the "why" behind any
   past nudge can be reconstructed later — required by `NON_NEGOTIABLES.md`
   §1 (the system says uncomfortable things and must be able to justify
   them) and §10 (fail loud, traceably).

This is a single SQLite file. No multi-tenancy, no `users` table — single-
user is enforced by *absence* of a user dimension, not by a `WHERE
user_id = 1` convention that could accidentally be widened later
(NON_NEGOTIABLES.md §8).

## 1. Entity Overview

```
semesters ─┬─< courses ─┬─< grade_snapshots
           │            └─< deadlines
           ├─< dsa_practice_log
           ├─< codeforces_snapshots
           ├─< projects ─< project_status_snapshots
           ├─< research_activities
           ├─< deep_work_sessions
           ├─< bottlenecks
           ├─< drift_signals
           ├─< opportunities
           ├─< decisions
           └─< recommendations

user_profile (single row, versioned via user_profile_history)
data_sources (staleness/sync tracking, referenced by many tables)
event_log (append-only audit trail — see EVENT_SYSTEM.md)
```

## 2. Core Tables

### `semesters`
The first-class re-derivation unit required by `NON_NEGOTIABLES.md` §7 and
`CORE_PRINCIPLES.md` #5.

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| label | TEXT | e.g. "2026 Odd Sem" |
| start_date | DATE | |
| end_date | DATE | |
| status | TEXT | `setup_pending` \| `active` \| `closed` |
| deep_work_window_start | TIME | Defaults to 20:00, but stored per-semester in case the user's actual peak window ever needs revisiting — the *default* is fixed by USER_PROFILE.md, not the schema |
| deep_work_window_end | TIME | Defaults to 00:00 |
| created_at | DATETIME | |

Why `deep_work_window_*` lives here rather than a hardcoded constant:
`NON_NEGOTIABLES.md` §3 fixes 20:00–00:00 as sacred *today*, but a schema
that hardcodes it in application code rather than data would force a code
change to ever revisit it — and `USER_PROFILE.md` is explicitly a living
document. Storing it per-semester with a fixed default preserves the
non-negotiable while keeping the mechanism data-driven.

### `courses`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| semester_id | INTEGER FK → semesters | |
| name | TEXT | |
| credits | REAL | |
| is_weak_subject | BOOLEAN | Set by the bottleneck detector, not manually — see `bottlenecks` |

### `grade_snapshots`
Time series, never overwritten.

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| course_id | INTEGER FK → courses | |
| captured_at | DATETIME | |
| assessment_label | TEXT | e.g. "Midsem", "Quiz 2" |
| score_percent | REAL NULL | Nullable — a missing score is not zero |
| cgpa_at_capture | REAL NULL | Whole-CGPA snapshot alongside course-level detail, so trend queries don't need to reconstruct history from course averages |
| source_id | INTEGER FK → data_sources | |

### `deadlines`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| semester_id | INTEGER FK → semesters | |
| course_id | INTEGER FK → courses, NULL | NULL for non-course deadlines (project milestones, application deadlines) |
| title | TEXT | |
| due_at | DATETIME | |
| estimated_hours_remaining | REAL NULL | Explicitly nullable — must never be silently defaulted (NON_NEG §5) |
| status | TEXT | `open` \| `done` \| `missed` |
| leverage_class | TEXT | `high` \| `low` — used by the deep-work guard; classification logic lives in `athena-domain`, not hand-set casually |

### `dsa_practice_log`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| semester_id | INTEGER FK | |
| logged_at | DATETIME | |
| topic | TEXT | |
| problems_attempted | INTEGER | |
| problems_solved | INTEGER | |
| source_id | INTEGER FK → data_sources | Distinguishes manual entry from Codeforces-derived |

### `codeforces_snapshots`
Time series pulled via the Codeforces public API (see `API_INTEGRATIONS.md`).

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| captured_at | DATETIME | |
| rating | INTEGER | |
| max_rating | INTEGER | |
| problems_solved_total | INTEGER | |
| source_id | INTEGER FK → data_sources | |

### `projects` / `project_status_snapshots`
| Column | Type | Notes |
|---|---|---|
| projects.id | INTEGER PK | |
| projects.name | TEXT | |
| projects.target_relevance | TEXT | Free text — how it maps to internship/portfolio goals, per `PROJECT_SCOPE.md` §2.3 |
| project_status_snapshots.project_id | FK | |
| project_status_snapshots.captured_at | DATETIME | |
| project_status_snapshots.status | TEXT | `not_started` \| `active` \| `stalled` \| `shipped` |
| project_status_snapshots.portfolio_strength_score | REAL NULL | Computed by domain layer, not user-entered — kept nullable until enough signal exists |

### `research_activities`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| semester_id | INTEGER FK | |
| description | TEXT | |
| relevance | TEXT | `msc_admission` \| `quant_ml_hiring` \| `both` — mirrors `USER_PROFILE.md`'s explicit statement that the two long-term goals are one compounding trajectory, not competing tracks |
| status | TEXT | |
| started_at | DATE NULL | |

### `deep_work_sessions`
The concrete record backing `NON_NEGOTIABLES.md` §3 and `CORE_PRINCIPLES.md`
#4.

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| semester_id | INTEGER FK | |
| session_date | DATE | |
| allocated_activity | TEXT | What Athena assigned as highest-leverage use of the window |
| allocated_activity_ref | TEXT | Polymorphic ref (`deadline:123`, `project:4`, `dsa_topic:graphs`) |
| actual_activity | TEXT NULL | Filled in after the fact, if logged |
| protected | BOOLEAN | Whether the guard successfully blocked an intrusion that evening |
| override_reason | TEXT NULL | Populated only if the user explicitly overrode the guard |

This table is what makes "was the deep-work block actually protected"
a queryable fact over years, not an assumption — directly supporting
drift detection (a slow erosion of `protected=true` sessions is itself a
drift signal).

### `bottlenecks`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| opened_at | DATETIME | |
| category | TEXT | `weak_subject` \| `stalled_project` \| `missing_skill` \| `other` |
| ref | TEXT | Polymorphic reference to the underlying entity |
| description | TEXT | |
| status | TEXT | `active` \| `resolved_by_evidence` — **there is no `resolved_by_inactivity` or `dismissed` state**, per `NON_NEGOTIABLES.md` §6: a bottleneck cannot quietly disappear from being ignored |
| resolution_evidence_ref | TEXT NULL | Must point to the specific data point (a grade snapshot, a rating jump) that justified closing it |

### `drift_signals`
Output of the periodic drift scan (`CORE_PRINCIPLES.md` #7).

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| detected_at | DATETIME | |
| signal_type | TEXT | `grade_slip` \| `practice_volume_drop` \| `subject_avoidance` \| `deep_work_erosion` |
| severity | TEXT | `watch` \| `flag` \| `urgent` |
| evidence_refs | TEXT (JSON array) | Every drift signal must cite the rows that produced it |
| acknowledged_at | DATETIME NULL | |

### `opportunities`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| title | TEXT | |
| category | TEXT | `internship` \| `research_position` \| `competition` \| `application_deadline` |
| relevance_score | REAL NULL | Computed against current trajectory, not generic |
| apply_by | DATE NULL | |
| status | TEXT | `surfaced` \| `pursuing` \| `applied` \| `dismissed` |

### `decisions`
The Decision Challenge Layer's ledger (`CORE_PRINCIPLES.md` #3,
`NON_NEGOTIABLES.md` §4).

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| submitted_at | DATETIME | |
| decision_type | TEXT | e.g. `reschedule`, `deprioritize`, `drop_task` |
| description | TEXT | |
| challenged | BOOLEAN | Whether the Challenge Layer intervened |
| challenge_reasoning | TEXT NULL | |
| final_outcome | TEXT | `accepted_as_is` \| `revised_by_user` \| `overridden_by_user` |
| resolved_at | DATETIME | |

### `recommendations`
The output ledger of the Priority Resolution / Reasoning layer.

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| generated_at | DATETIME | |
| kind | TEXT | `priority_now` \| `deep_work_allocation` \| `bottleneck_alert` \| `opportunity_surface` \| `challenge` |
| verdict | TEXT | The ranked answer, in plain language |
| reasoning | TEXT | The mandatory one-sentence-minimum "why" (NON_NEG §2) |
| confidence | TEXT | `confirmed` \| `inferred` \| `insufficient_data` — never nullable (NON_NEG §10) |
| grounded_in | TEXT (JSON array of table:id refs) | Every fact cited must resolve to a real row; enforced at the reasoning-layer boundary, not just convention — see AI_PIPELINE.md §5 |
| data_freshness_note | TEXT NULL | Populated when any grounding fact is stale |
| user_response | TEXT NULL | `followed` \| `overridden` \| `ignored` — feeds future confidence calibration |

## 3. Supporting Tables

### `data_sources`
| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK | |
| kind | TEXT | `manual_entry` \| `codeforces_sync` \| `ics_import` \| `csv_import` |
| last_synced_at | DATETIME NULL | |
| staleness_threshold_hours | INTEGER | Beyond this, any recommendation grounded in this source must carry a staleness note |

### `user_profile` / `user_profile_history`
A single logical row (goals, target CGPA, institution, long-term goals),
versioned by history table rather than mutated in place — because
`USER_PROFILE.md` explicitly says this is revisited every semester and
after every exam cycle, and losing the prior version loses the ability to
see how the model of the user has itself evolved.

### `event_log`
Append-only. See `EVENT_SYSTEM.md` for the event taxonomy. This table is
the audit trail that makes the whole system's behavior explainable after
the fact — every command and event that touched state is recorded here
with a timestamp and payload, independent of the domain tables' current
state.

## 4. Explicitly Rejected Tables

Naming what does **not** exist is as important as the schema itself:

- **No `tasks` table for arbitrary to-dos.** `PROJECT_SCOPE.md` explicitly
  excludes general task management. Anything resembling a task is one of
  `deadlines`, `deep_work_sessions.allocated_activity`, or a
  `research_activities`/`projects` row — always tied to trajectory.
- **No `streaks` / `badges` / `points` tables.** Would be a pure proxy
  metric and a direct violation of `NON_NEGOTIABLES.md` §9.
- **No `weekly_template` table.** Would violate §7 by definition.
- **No `shared_with` / `collaborators` columns anywhere.** Single-tenant
  by omission, per §8.

## 5. Proxy vs. Trajectory Metrics — Explicit Separation

To make `NON_NEGOTIABLES.md` §9 enforceable rather than aspirational, the
schema never lets a "completion" fact and a "trajectory" fact live in the
same column or be conflated in a query without deliberate joining:

- **Proxy metrics** (things that are easy to game): `deadlines.status =
  'done'`, `dsa_practice_log.problems_attempted`.
- **Trajectory metrics** (the real objective): `grade_snapshots.*`,
  `codeforces_snapshots.rating`, `project_status_snapshots
  .portfolio_strength_score`.

The domain layer's `DivergenceCheck` (see `MODULES.md`) is the only code
path allowed to compare these two families, and it exists specifically to
flag when they diverge — e.g. deadlines are being marked done at a normal
rate while grade trajectory slides. This check has no schema equivalent of
"average completion rate" as a headline metric anywhere in the UI-facing
queries, intentionally.

## 6. Indexing Notes

Given single-user, single-machine scale, indexing is not a performance
concern in the traditional sense — total row counts over 5 years are in
the tens of thousands, not millions. Indexes are added for query
*ergonomics and correctness* (e.g. `semester_id` indexed on every table
that has it, `captured_at` indexed on every snapshot table for range
queries) rather than performance tuning.

## 7. Migration Philosophy

Migrations are **additive-only**. A column is never dropped or repurposed
in place across a semester boundary — it is deprecated (documented, no
longer written to) and only removed in a deliberate cleanup migration once
no historical query depends on it. This matters specifically because
`grade_snapshots`, `codeforces_snapshots`, and `deep_work_sessions` are
trend data — a destructive migration mid-history would corrupt exactly the
long-horizon signal (`CORE_PRINCIPLES.md` #7) the system exists to
preserve.
