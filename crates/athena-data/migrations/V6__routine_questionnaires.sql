-- V6__routine_questionnaires.sql
--
-- Adds storage for the daily and weekly routine questionnaires. Purely
-- additive (01_ARCHITECTURE.md §6.2, matching V2-V5's own precedent) —
-- no existing table, column, or row is touched.
--
-- Two separate tables rather than one table with a cadence column: the
-- two questionnaires ask genuinely different questions (daily is a
-- handful of quick fields meant to feed tonight's `available_minutes_
-- tonight` computation in `athena_domain::planner`; weekly is a longer,
-- reflective check-in with no equivalent same-day consumer). A shared
-- table would force every row to carry a wide set of mostly-NULL
-- columns depending on cadence, which every existing table in this
-- schema (courses, deadlines, schedule_disruptions) avoids by giving
-- each distinct shape its own table. `daily_routine_responses` and
-- `weekly_routine_responses` follow that same convention.
--
-- Daily questionnaire fields are chosen specifically to feed the
-- Adaptive Planner (`crates/athena-domain/src/planner.rs`):
--   - `energy_level` (1-5) and `hours_available_tonight` are read
--     directly by a future `replan` call as a second, user-reported
--     input alongside `available_minutes_tonight`'s existing
--     disruption-based computation — the module's own doc comment
--     already documents `estimated_minutes` as "a fixed per-leverage-
--     tier estimate, not a fabricated per-item number," and these two
--     fields are the honest, user-supplied numbers that estimate is
--     standing in for today.
--   - `had_disruption_today` / `disruption_note` give the same signal
--     `schedule_disruptions` captures, but as a lightweight opt-in
--     prompt rather than requiring the user to navigate to the
--     dedicated disruption-logging flow — this table does not
--     duplicate `schedule_disruptions`' row-per-event, cause-coded
--     shape; it is a same-day, best-effort corroborating signal only.
--   - `focus_rating` (1-5) is a retrospective self-report, stored for
--     trend purposes (`list_recent`) but not currently read by any
--     planner computation — an honest, undecorated field, matching
--     this schema's existing practice of storing more than is
--     immediately consumed (see `courses.target_grade`).
--
-- Weekly questionnaire fields are broader and reflective, feeding
-- longer-horizon trend/weakness-analysis surfaces
-- (`commands::ai::get_weekly_plan`, `get_weakness_analysis`) rather
-- than tonight's window specifically.
--
-- One row per day/week per submission (`insert`-only, no upsert) — a
-- resubmission on the same day is a new row, letting a user log an
-- energy shift partway through the day without losing the morning's
-- answer, mirroring `schedule_disruptions`' own append-only shape.

CREATE TABLE daily_routine_responses (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    date                      TEXT    NOT NULL,
    energy_level              INTEGER NOT NULL,
    hours_available_tonight   REAL    NOT NULL,
    had_disruption_today      INTEGER NOT NULL DEFAULT 0,
    disruption_note           TEXT,
    focus_rating              INTEGER NOT NULL,
    submitted_at              TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE weekly_routine_responses (
    id                          INTEGER PRIMARY KEY AUTOINCREMENT,
    week_starting               TEXT    NOT NULL,
    overall_energy_trend        INTEGER NOT NULL,
    satisfaction_with_progress  INTEGER NOT NULL,
    hardest_course_id           INTEGER,
    biggest_blocker             TEXT,
    hours_studied_estimate      REAL,
    wants_deep_work_adjustment  INTEGER NOT NULL DEFAULT 0,
    notes                       TEXT,
    submitted_at                 TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (hardest_course_id) REFERENCES courses(id)
);

CREATE INDEX idx_daily_routine_responses_date ON daily_routine_responses(date);
CREATE INDEX idx_weekly_routine_responses_week ON weekly_routine_responses(week_starting);
