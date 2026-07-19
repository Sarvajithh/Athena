-- V4__integrations.sql
--
-- Adds the schema required to implement 07_INTEGRATIONS.md's Version 1
-- connectors end-to-end: Codeforces, LeetCode, GitHub, Calendar Import
-- (.ics), Resume/Transcript Import (PDF), CSV Import, and Manual Import.
-- Additive only, nothing dropped or repurposed (01_ARCHITECTURE.md
-- §6.2), matching V2/V3's own precedent, and every table below is
-- justified directly against 07_INTEGRATIONS.md per PROJECT_RULES.md
-- Immutable Rule #7.
--
-- Three deliberate, documented deviations from 07_INTEGRATIONS.md's
-- literal text, all forced by a direct conflict between that document
-- and the schema this codebase actually has (V1-V3 migrations), per the
-- brief's instruction to resolve rather than silently ignore such a
-- conflict:
--
-- 1. §1.2 (LeetCode) says it feeds "a supplementary trajectory-metric
--    column alongside `codeforces_snapshots`" and flags that as its own
--    cited schema change (Immutable Rule #7). `codeforces_snapshots`
--    itself does not exist yet in this schema either — Codeforces was
--    never actually built despite §1.1 calling it "as specified,
--    unchanged." Both tables are created here, for the first time, as
--    one reviewed deliverable (this migration), rather than LeetCode
--    silently depending on a table that doesn't exist.
--
-- 2. §1.3 (GitHub) and §1.5 (PDF Import) both say they feed
--    `project_status_snapshots` / `research_activities`. Neither table
--    exists in this schema (04_DATA_MODEL.md's tables actually migrated
--    are `courses` and `deadlines` — see V2). Resolved per-source:
--      - GitHub's polled commit/PR/issue data is continuous,
--        provider-shaped telemetry that does not fit `deadlines`'
--        point-in-time shape — it gets the one new table this document
--        justifies, `project_status_snapshots`, scoped to exactly the
--        fields §1.3 names (commit cadence, PR/issue counts).
--      - PDF Import's extracted facts (§1.5: "a project, a publication,
--        a certification... mapped directly onto existing typed
--        entities") are, after user confirmation, one-time achievement
--        records — the same shape `deadlines` already holds (title,
--        category, a date, notes). They are inserted as `deadlines`
--        rows with `category = 'career'` or `'research'` and
--        `status = 'done'` (already happened, not upcoming) instead of
--        inventing `research_activities`, honoring "mapped directly
--        onto existing typed entities" using the existing entity that
--        actually exists.
--
-- 3. §1.4 (Calendar Import) and §5 both describe an "existing ICS
--    parser" living in `athena-domain`. `athena-domain`'s own lib.rs
--    states it takes "nothing beyond the Rust standard library" and is
--    "pure reasoning rules, zero I/O" — file parsing is I/O. The parser
--    is implemented in `athena-ingestion` instead (that crate's own
--    lib.rs doc comment already names "ICS import" as one of its
--    reasons to exist), preserving `athena-domain`'s zero-I/O
--    invariant. Calendar Import's output lands in `deadlines`, exactly
--    as §1.4 specifies — only the parser's crate location changes.
--
-- `data_sources` is the one table every integration in this document
-- shares: it is 07_INTEGRATIONS.md §5's own vocabulary
-- ("`data_sources.last_synced_at`") made real. One row per connector,
-- seeded here (operational bootstrap state, not illustrative sample
-- product data, so it does not fall under Immutable Rule #7's ban on
-- seed data) so every screen surfacing connector status has a real row
-- to query from the very first launch, before any sync has ever run.

CREATE TABLE data_sources (
    source_key    TEXT PRIMARY KEY CHECK (source_key IN (
                      'codeforces', 'leetcode', 'github', 'calendar_ics',
                      'pdf_import', 'csv_import', 'manual'
                  )),
    kind          TEXT NOT NULL CHECK (kind IN ('poll', 'import', 'always_on')),
    status        TEXT NOT NULL DEFAULT 'disconnected' CHECK (status IN (
                      'disconnected', 'idle', 'syncing', 'ok', 'error'
                  )),
    last_synced_at  TEXT,
    last_error      TEXT,
    -- Per-connector configuration (handle/username, linked repo list,
    -- token-present flag — never the token itself, see §4/`keychain.rs`)
    -- as JSON text, mirroring `meeting_pattern`'s precedent (V2) for
    -- "small, connector-specific shape that doesn't earn its own table."
    config_json     TEXT,
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO data_sources (source_key, kind, status) VALUES
    ('codeforces',   'poll',      'disconnected'),
    ('leetcode',     'poll',      'disconnected'),
    ('github',       'poll',      'disconnected'),
    ('calendar_ics', 'import',    'idle'),
    ('pdf_import',   'import',    'idle'),
    ('csv_import',   'import',    'idle'),
    -- Manual entry (§1.7) has no connection to lose and nothing to
    -- sync — it is always available, by construction, whenever every
    -- other integration is disconnected (07_INTEGRATIONS.md's own
    -- framing: "the always-available fallback").
    ('manual',       'always_on', 'ok');

-- §1.1/§1.2 — trajectory metrics. Two tables, one per provider, per the
-- cited-schema-change requirement in §1.2 rather than overloading one
-- table with a `source` discriminator column that would blur two
-- providers' different fields (rating vs. difficulty-bucketed counts).
CREATE TABLE codeforces_snapshots (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    handle        TEXT    NOT NULL,
    rating        INTEGER,
    max_rating    INTEGER,
    rank          TEXT,
    solved_count  INTEGER NOT NULL DEFAULT 0,
    fetched_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE dsa_practice_log (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    source         TEXT    NOT NULL CHECK (source IN ('leetcode')),
    handle         TEXT    NOT NULL,
    total_solved   INTEGER NOT NULL DEFAULT 0,
    easy_solved    INTEGER NOT NULL DEFAULT 0,
    medium_solved  INTEGER NOT NULL DEFAULT 0,
    hard_solved    INTEGER NOT NULL DEFAULT 0,
    fetched_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- §1.3 — repos the user explicitly links (never a full account scan).
CREATE TABLE linked_github_repos (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_full_name   TEXT    NOT NULL UNIQUE, -- e.g. "octocat/Hello-World"
    added_at         TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE project_status_snapshots (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_full_name      TEXT    NOT NULL,
    commit_count_30d    INTEGER NOT NULL DEFAULT 0,
    open_pr_count       INTEGER NOT NULL DEFAULT 0,
    open_issue_count    INTEGER NOT NULL DEFAULT 0,
    last_commit_at      TEXT,
    fetched_at          TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_codeforces_snapshots_handle_fetched ON codeforces_snapshots(handle, fetched_at);
CREATE INDEX idx_dsa_practice_log_handle_fetched ON dsa_practice_log(handle, fetched_at);
CREATE INDEX idx_project_status_snapshots_repo_fetched ON project_status_snapshots(repo_full_name, fetched_at);
