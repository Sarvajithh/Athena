-- V7__routine_questionnaire_schedule.sql
--
-- Adds the one configurable field the scheduled daily-questionnaire
-- trigger needs: what local time of day it should fire
-- (`routine_questionnaire_time`, `HH:MM`, 24-hour). Purely additive
-- (01_ARCHITECTURE.md §6.2, same precedent as V2-V6) — no existing
-- table, column, or row is touched.
--
-- Lives on `user_profile` rather than a new table: `user_profile`
-- already carries the other two schedule-shaped fields this feature
-- needs to reason about time-of-day at all, `deep_work_window_start`
-- and `deep_work_window_end` (V2 migration) — same `TEXT`, `HH:MM`
-- convention, same single-row-per-installation cardinality. A new
-- one-row `routine_schedule` table would just duplicate that
-- cardinality for a single extra column with no independent lifecycle
-- of its own (it is never listed, versioned, or queried apart from the
-- profile it belongs to), which is exactly the shape V2's own
-- `courses`-table doc comment argues against elsewhere in this schema.
--
-- `NOT NULL DEFAULT '20:00'`: every existing `user_profile` row (there
-- is at most one, since it's a single-row table by product rule) gets
-- a real, honest default the moment this migration runs, so
-- `get_routine_questionnaire_time` never has to special-case a NULL
-- column — only the "no profile row exists yet at all" case (pre-
-- onboarding), which it already handles separately.

ALTER TABLE user_profile
    ADD COLUMN routine_questionnaire_time TEXT NOT NULL DEFAULT '20:00';
