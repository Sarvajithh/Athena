-- V3__schedule_disruptions.sql
--
-- Adds the one new table `08_ADAPTIVE_PLANNER.md` §5 calls for, flagged
-- and justified per PROJECT_RULES.md Immutable Rule #7: an auditable,
-- typed log of *why* a plan changed. Additive only, nothing dropped or
-- repurposed (01_ARCHITECTURE.md §6.2) — matches V2's own precedent.
--
-- Two deliberate, documented deviations from §5's literal shape, both
-- forced by a direct conflict between this document and the schema this
-- codebase actually has (V1/V2 migrations), per the brief's instruction
-- to resolve rather than silently ignore such a conflict:
--
-- 1. `linked_opportunity_id` is omitted. §5 has it referencing an
--    `opportunities` table that was never migrated in this schema
--    (`src/screens/Now/index.tsx` already documents this same gap for
--    its own Opportunity Feed section: "always-empty today because
--    their backing tables... don't exist in this schema, and this
--    change is explicitly scoped not to modify storage" — the same
--    reasoning applies here). `unexpected_opportunity` disruptions
--    (§4.4) still log fine; they just can't FK to a row that can't
--    exist yet.
--
-- 2. `recommendation_id_after` is replaced with two inline columns,
--    `recompute_headline` / `recompute_reasoning`, instead of an FK to a
--    `recommendations` table — no such table exists in this schema
--    either (verdicts are computed on demand by `athena-domain::priority`
--    / `athena-domain::planner`, never persisted as rows; see
--    `crates/athena-app/src/commands/bootstrap.rs`). Storing the
--    recompute's own headline/reasoning directly on the disruption row
--    achieves §5's stated goal ("the causal chain (disruption -> new
--    verdict) is itself queryable and auditable") without inventing a
--    table this codebase has no other reason to have.
CREATE TABLE schedule_disruptions (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    semester_id               INTEGER NOT NULL REFERENCES semesters(id),
    date                      TEXT    NOT NULL,
    disruption_type           TEXT    NOT NULL CHECK (disruption_type IN (
                                  'external_interrupt', 'surprise_workload', 'cancelled_class',
                                  'unexpected_opportunity', 'illness', 'early_finish'
                              )),
    duration_minutes         INTEGER NOT NULL DEFAULT 0,
    affects_deep_work_window INTEGER NOT NULL DEFAULT 0,
    linked_deadline_id       INTEGER REFERENCES deadlines(id),
    note                      TEXT,
    logged_at                 TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    recompute_triggered       INTEGER NOT NULL DEFAULT 0,
    recompute_headline        TEXT,
    recompute_reasoning       TEXT
);

CREATE INDEX idx_schedule_disruptions_semester ON schedule_disruptions(semester_id);
CREATE INDEX idx_schedule_disruptions_date ON schedule_disruptions(date);