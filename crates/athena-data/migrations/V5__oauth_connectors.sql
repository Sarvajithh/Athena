-- V5__oauth_connectors.sql
--
-- Adds the schema required for 07_INTEGRATIONS.md §1.8-§1.10 (the
-- 2026-07-17 OAuth amendment): Gmail, Google Classroom, and Notion.
-- Additive only, nothing dropped or repurposed (01_ARCHITECTURE.md
-- §6.2), matching V2/V3/V4's own precedent. Every table here is
-- justified directly against 07_INTEGRATIONS.md §1.8-§1.10 per
-- PROJECT_RULES.md Immutable Rule #7.
--
-- One deviation, forced by SQLite's own limitation rather than a design
-- choice: SQLite has no `ALTER TABLE ... ALTER CONSTRAINT`, so widening
-- `data_sources.source_key`'s and `data_sources.kind`'s CHECK
-- constraints to admit the three new source keys and the new
-- `oauth_poll` kind requires the standard SQLite rebuild pattern
-- (create-new, copy, drop-old, rename) rather than an in-place ALTER.
-- No data is lost — every existing row is copied forward unchanged.

CREATE TABLE data_sources_v5 (
    source_key    TEXT PRIMARY KEY CHECK (source_key IN (
                      'codeforces', 'leetcode', 'github', 'calendar_ics',
                      'pdf_import', 'csv_import', 'manual',
                      'gmail', 'google_classroom', 'notion'
                  )),
    kind          TEXT NOT NULL CHECK (kind IN ('poll', 'import', 'always_on', 'oauth_poll')),
    status        TEXT NOT NULL DEFAULT 'disconnected' CHECK (status IN (
                      'disconnected', 'idle', 'syncing', 'ok', 'error'
                  )),
    last_synced_at  TEXT,
    last_error      TEXT,
    config_json     TEXT,
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO data_sources_v5 (source_key, kind, status, last_synced_at, last_error, config_json, updated_at)
    SELECT source_key, kind, status, last_synced_at, last_error, config_json, updated_at FROM data_sources;

DROP TABLE data_sources;
ALTER TABLE data_sources_v5 RENAME TO data_sources;

-- New connectors seeded `disconnected`, same as every poll connector's
-- initial state (V4 precedent) — nothing here is reachable until the
-- user completes the OAuth flow.
INSERT INTO data_sources (source_key, kind, status) VALUES
    ('gmail',            'oauth_poll', 'disconnected'),
    ('google_classroom', 'oauth_poll', 'disconnected'),
    ('notion',           'oauth_poll', 'disconnected');

-- §1.8 — Gmail inbox metadata. `message_id` is globally unique per
-- Gmail's own API contract, so re-polling upserts the same logical
-- message rather than growing an unbounded duplicate history (unlike
-- codeforces_snapshots'/project_status_snapshots' append-only shape,
-- which models a changing trajectory metric over time — a given email
-- does not "change" the way a rating does).
CREATE TABLE gmail_message_snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id      TEXT    NOT NULL UNIQUE,
    thread_id       TEXT,
    sender          TEXT,
    subject         TEXT,
    received_at     TEXT,
    snippet         TEXT,
    fetched_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- §1.9 — Google Classroom. Three tables, one per resource type named in
-- §1.9 ("Courses, Assignments, Due dates, Announcements"), each
-- upserted by its own provider-issued ID for the same reason as Gmail
-- above (a course/assignment/announcement is a stable entity that gets
-- updated in place, not a repeating time-series point).
CREATE TABLE classroom_courses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    course_id       TEXT    NOT NULL UNIQUE,
    name            TEXT    NOT NULL,
    section         TEXT,
    fetched_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE classroom_coursework (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    coursework_id   TEXT    NOT NULL UNIQUE,
    course_id       TEXT    NOT NULL,
    title           TEXT    NOT NULL,
    due_at          TEXT,
    state           TEXT,
    fetched_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE classroom_announcements (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    announcement_id   TEXT    NOT NULL UNIQUE,
    course_id         TEXT    NOT NULL,
    text              TEXT,
    posted_at         TEXT,
    fetched_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- §1.10 — Notion. Reference metadata only (title/URL/parent/last-edited)
-- per §1.10's narrower-than-task-sync scope — no page content/body is
-- ever stored.
CREATE TABLE notion_pages (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    page_id               TEXT    NOT NULL UNIQUE,
    title                 TEXT,
    url                   TEXT,
    parent_database_id    TEXT,
    last_edited_at        TEXT,
    fetched_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_gmail_message_snapshots_received ON gmail_message_snapshots(received_at);
CREATE INDEX idx_classroom_coursework_course ON classroom_coursework(course_id, due_at);
CREATE INDEX idx_classroom_announcements_course ON classroom_announcements(course_id, posted_at);
CREATE INDEX idx_notion_pages_last_edited ON notion_pages(last_edited_at);
