-- V2__onboarding_schema.sql
--
-- Adds the tables required to implement 03_ONBOARDING.md end-to-end:
-- Profile creation, Semester Setup, and the honest (currently-empty)
-- read models `Now`, `Trajectory`, and `Decision Log` query instead of
-- Sprint 2's static mock fixtures.
--
-- Every table/column here is justified directly by 04_DATA_MODEL.md
-- (§1 Profile, §3 Semester, §2 Courses, §5 Deadlines, §9 History) per
-- PROJECT_RULES.md Immutable Rule #7 — additive only, nothing dropped or
-- repurposed (01_ARCHITECTURE.md §6.2).
--
-- One deliberate, documented addition beyond 04_DATA_MODEL.md's literal
-- `user_profile` field list: `codeforces_handle`. 03_ONBOARDING.md §5.1
-- states the handle collected in Profile Step 3 "is kept on the
-- profile," but 04_DATA_MODEL.md §1's `user_profile` shape does not list
-- such a column — a direct conflict between the two governing documents.
-- Resolved here, minimally and additively, in favor of making the
-- explicit onboarding requirement satisfiable, per the brief's own
-- instruction to resolve (not silently ignore) a direct implementation
-- conflict.
--
-- No table below stores sample/illustrative values from 04_DATA_MODEL.md
-- — those are shape-only examples, not seed data.

-- §1 Profile — single current-state row + append-only history.
CREATE TABLE user_profile (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    name                    TEXT    NOT NULL,
    institute               TEXT    NOT NULL,
    program                 TEXT    NOT NULL,
    current_semester_id     INTEGER REFERENCES semesters(id),
    target_cgpa             REAL    NOT NULL,
    current_cgpa            REAL,
    career_target           TEXT    NOT NULL,
    masters_target          TEXT,
    codeforces_handle       TEXT,
    deep_work_window_start  TEXT    NOT NULL,
    deep_work_window_end    TEXT    NOT NULL,
    timezone                TEXT    NOT NULL,
    created_at              TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at              TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE user_profile_history (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    user_profile_id   INTEGER NOT NULL REFERENCES user_profile(id),
    semester_id       INTEGER REFERENCES semesters(id),
    recorded_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    reason            TEXT    NOT NULL CHECK (reason IN ('semester_rollover', 'manual_edit', 'onboarding')),
    snapshot          TEXT    NOT NULL,
    changed_fields    TEXT    NOT NULL
);

-- §3 Semester — no multi-tenancy dimension anywhere (01_ARCHITECTURE.md §6.1).
CREATE TABLE semesters (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT    NOT NULL,
    starts_on   TEXT    NOT NULL,
    ends_on     TEXT    NOT NULL,
    is_current  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- §2 Courses — meeting_pattern is a fixed attribute carried as JSON on
-- the course row, not a separate table (04_DATA_MODEL.md §2's explicit
-- reasoning against a `class_schedule` table).
CREATE TABLE courses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    semester_id     INTEGER NOT NULL REFERENCES semesters(id),
    code            TEXT    NOT NULL,
    title           TEXT    NOT NULL,
    credits         INTEGER NOT NULL,
    leverage_class  TEXT    NOT NULL CHECK (leverage_class IN ('high', 'medium', 'low')),
    instructor      TEXT,
    target_grade    TEXT,
    meeting_pattern TEXT,
    status          TEXT    NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'dropped')),
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- §5 Deadlines — single canonical table for academic/career/research/dsa/other.
CREATE TABLE deadlines (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    semester_id     INTEGER NOT NULL REFERENCES semesters(id),
    course_id       INTEGER REFERENCES courses(id),
    title           TEXT    NOT NULL,
    category        TEXT    NOT NULL CHECK (category IN ('academic', 'career', 'research', 'dsa', 'other')),
    due_at          TEXT    NOT NULL,
    leverage_class  TEXT    NOT NULL CHECK (leverage_class IN ('high', 'medium', 'low')),
    status          TEXT    NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'done', 'missed')),
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    notes           TEXT
);

-- §9 History — audit tables. `decisions` has no write path yet (the
-- Decision Challenge Layer / Deep Work Guard are future sprints per
-- 01_ARCHITECTURE.md §1.1); it is created now so Decision Log queries a
-- real, honestly-empty table instead of a mock fixture.
CREATE TABLE decisions (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    semester_id          INTEGER NOT NULL REFERENCES semesters(id),
    decision_type        TEXT    NOT NULL,
    description          TEXT    NOT NULL,
    challenge_fired      INTEGER NOT NULL DEFAULT 0,
    challenge_reasoning  TEXT,
    final_outcome        TEXT    CHECK (final_outcome IN ('kept', 'reversed', 'overridden')),
    decided_at           TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- event_log — append-only, every event whether subscribed or not
-- (01_ARCHITECTURE.md §6.4).
CREATE TABLE event_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type   TEXT NOT NULL,
    payload      TEXT NOT NULL,
    occurred_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_courses_semester ON courses(semester_id);
CREATE INDEX idx_deadlines_semester ON deadlines(semester_id);
CREATE INDEX idx_deadlines_status ON deadlines(status);
CREATE INDEX idx_user_profile_history_profile ON user_profile_history(user_profile_id);
