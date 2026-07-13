# API_INTEGRATIONS.md — Project Athena

## 0. Governing Rule

Every external integration is evaluated against one question first:
**does this integration make the system's facts more grounded, or does it
just add convenience at the cost of fragility/privacy?** Given the 5-year
survivability requirement and the sole-ownership constraint
(`NON_NEGOTIABLES.md` §8), convenience alone never justifies a new outbound
dependency. This is why several "obvious" integrations (institute portal
scraping, calendar cloud-sync, task-manager sync) are explicitly rejected
below rather than merely deferred.

## 1. Codeforces Public API

**Purpose:** feed `codeforces_snapshots` and inform DSA/competitive-
programming trajectory tracking (`PROJECT_SCOPE.md` §2.2, `USER_PROFILE.md`
"Codeforces" front).

**Integration shape:** read-only polling against Codeforces' public,
documented REST API (`user.rating`, `user.status`) using the user's
handle. No authentication beyond the handle is required for public data.

**Why this one is safe to depend on long-term:** it's a stable, documented,
public API with no ToS ambiguity about automated read access to one's own
public profile data, and it maps directly onto a trajectory metric
(`DATABASE_SCHEMA.md` §5) rather than a proxy metric.

**Failure handling:** on sync failure, `data_sources.last_synced_at`
simply doesn't advance; any recommendation grounded in Codeforces data
attaches a `data_freshness_note` (per `AI_PIPELINE.md` §3) rather than
silently using stale data as if current. The app never blocks on this
sync — it's opportunistic, scheduled, and best-effort.

**What is explicitly not built:** no scraping of Codeforces' non-API pages
(contest problem content, editorials) — out of scope and unnecessary; the
system only needs the rating/volume signal, not problem content.

## 2. LLM Provider API (Anthropic Claude, primary)

**Purpose:** Stage 4 synthesis in `AI_PIPELINE.md`.

**Integration shape:** standard `/v1/messages` calls, JSON-schema-
constrained output, narrow per-call payloads (only the specific
retrieval manifest for that synthesis, never a database dump).

**Why cloud despite local-first:** addressed fully in `AI_PIPELINE.md` §4
— synthesis quality benefit is real, and the risk is bounded because the
model cannot introduce facts, only phrase already-decided ones.

**Privacy handling:** payloads include course names, scores, deadline
titles, and derived scores — not the student's institute ID, full name in
raw form beyond what's needed for phrasing, or unrelated personal data.
No conversation history is retained server-side beyond what the provider's
standard API retention policy covers; the app does not build a persistent
cloud-side memory of the user.

**Failure handling:** falls back to the local model path (§3) or, failing
that, to the template-flattened Stage 6 output described in
`AI_PIPELINE.md` §2. The app is never blocked waiting on this call for
core functionality like logging a grade or viewing raw trajectory data.

## 3. Local LLM (Ollama or equivalent), Fallback

**Purpose:** offline/opt-out synthesis fallback (`AI_PIPELINE.md` §4).

**Integration shape:** a local HTTP call to a locally-running inference
server on `localhost`, no network egress at all. Model is a small
instruction-tuned model chosen for reliability of following the grounding-
constrained output schema over raw fluency.

**Why this matters for 5-year survivability:** removes a hard dependency
on any single cloud vendor remaining available, priced acceptably, or API-
compatible five years out. This is insurance, not the primary path.

## 4. Institute Timetable / Grades — Deliberately Not a Live Integration

**What was considered and rejected:** scraping IIT Hyderabad's student
portal or LMS for live timetable/grade data.

**Why rejected:**
- No public, documented API exists for this — any integration would be a
  scraper against a private, unversioned, authentication-gated system.
- Institute portals change their markup/flow without notice; a scraper is
  one of the highest-maintenance, most brittle things that could be built
  into a 5-year system, directly contradicting the maintainability
  requirement in `ARCHITECTURE.md` §6.
- Storing institute login credentials inside the app to enable scraping
  would itself be a security liability disproportionate to the value
  gained, and sits uncomfortably against the sole-ownership/privacy
  posture of `NON_NEGOTIABLES.md` §8 (why hold a second system's
  credentials at all if it can be avoided).

**What's built instead:** `athena-ingestion::csv_import` and
`athena-ingestion::ics_import` — the user exports their timetable/grades
(most institute portals support a CSV or calendar export) and imports it
through `Semester Setup`. This keeps semester re-derivation
(`NON_NEGOTIABLES.md` §7) manual-but-structured rather than
automatic-but-fragile. It is explicitly re-run every semester, which
matches the "re-derive, don't reuse" principle exactly — there is no
temptation to let a stale live-sync silently carry over last semester's
structure.

## 5. Calendar (.ics) Import

**Purpose:** bulk-load deadlines/exam dates from any calendar export
(Google Calendar, institute calendar, etc.) at Semester Setup time.

**Integration shape:** local file parse only — the user exports and
selects a file; there is no live calendar API polling/sync. This is a
one-time (per semester) ingestion, not an ongoing integration, which keeps
it in the "does this make facts more grounded" camp without adding a
standing external dependency or requiring OAuth against a calendar
provider.

## 6. Windows OS Integration

**Purpose:** native notifications for deep-work session start/close-out,
staleness alerts, and drift flags — via Tauri's notification and system
tray APIs.

**Why this counts as an "integration" worth documenting:** every
notification surfaced this way must still carry the mandatory reasoning
field (`NON_NEGOTIABLES.md` §2) — the OS notification is a *delivery
channel* for a `Recommendation`/`Alert` object, never a bare string
constructed ad hoc at the call site. Enforced by the fact that the tray/
notification module only accepts the typed `Recommendation`/`Alert`
domain object as input, not a raw string (see `MODULES.md` §7).

## 7. What Is Explicitly Never Built

- **No cloud backup/sync service integration** (Dropbox, OneDrive API,
  etc.) as a first-class feature. The user may put their local
  `backups/` folder under their own sync tool if they choose to, entirely
  outside the app's awareness — but Athena itself never talks to a cloud
  storage API, per `NON_NEGOTIABLES.md` §8 ("never a shared or exportable
  asset by default").
- **No third-party task manager sync** (Notion, Todoist, etc.) —
  `PROJECT_SCOPE.md` explicitly rules out general task management as a
  concept, so there's nothing for such an integration to serve.
- **No social/sharing integrations** — no export-to-share, no public
  profile, nothing multi-user-shaped, consistent with §8 and the explicit
  "never a multi-user or commercial product" language.
- **No analytics/telemetry SDKs.** A system whose entire purpose is
  protecting the privacy and sole ownership of one person's academic and
  performance data does not phone home usage data to a third party as a
  side effect of existing.

## 8. Network Access Summary

| Destination | Direction | Trigger | Can app function without it? |
|---|---|---|---|
| Codeforces API | Outbound, read-only | Scheduled sync | Yes — degrades to stale-flagged data |
| Anthropic API | Outbound | On-demand synthesis calls | Yes — falls back to local model, then template output |
| Local model server | Localhost only | Fallback synthesis | Yes — this *is* the no-network fallback |
| Anything else | None | — | — |

This table is the concrete answer to "is Athena local-first" — every
outbound call is optional, degrades gracefully, and is either read-only
(Codeforces) or fact-bounded by design (LLM synthesis, per
`AI_PIPELINE.md`).
