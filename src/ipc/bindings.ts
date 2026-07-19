import { invoke } from "@tauri-apps/api/core";

/**
 * Mirrors the Rust `AppVersionInfo` struct returned by the
 * `get_app_version` command (crates/athena-app/src/commands/mod.rs).
 */
export interface AppVersionInfo {
  version: string;
}

/** Calls the one proof-of-life IPC command registered in S01. */
export async function getAppVersion(): Promise<AppVersionInfo> {
  return invoke<AppVersionInfo>("get_app_version");
}

/** Whether credential storage is currently using the local encrypted-file
 * fallback because the OS keychain backend is unavailable (Task 4). */
export async function isUsingKeychainFallback(): Promise<boolean> {
  return invoke<boolean>("is_using_keychain_fallback");
}

// ---------------------------------------------------------------------
// Onboarding + bootstrap — mirrors
// crates/athena-app/src/commands/{bootstrap,onboarding}.rs and the
// underlying athena-data repository row shapes (04_DATA_MODEL.md).
// Every interface below is a 1:1 mirror of a Rust struct's public
// fields; no reshaping happens on this side (01_ARCHITECTURE.md §2.1).
// ---------------------------------------------------------------------

export type LeverageClass = "high" | "medium" | "low";
export type DeadlineCategory = "academic" | "career" | "research" | "dsa" | "other";
export type DeadlineStatus = "open" | "done" | "missed";
export type CourseStatus = "active" | "completed" | "dropped";
export type Confidence = "confirmed" | "inferred" | "insufficient_data";

export interface MeetingSlot {
  day: string;
  start: string;
  end: string;
}

export interface ProfileRow {
  id: number;
  name: string;
  institute: string;
  program: string;
  current_semester_id: number | null;
  target_cgpa: number;
  current_cgpa: number | null;
  career_target: string;
  masters_target: string | null;
  codeforces_handle: string | null;
  deep_work_window_start: string;
  deep_work_window_end: string;
  timezone: string;
  /** `HH:MM`, 24-hour, local time — when the scheduled daily-questionnaire trigger fires (V7 migration). */
  routine_questionnaire_time: string;
  created_at: string;
  updated_at: string;
}

export interface SemesterRow {
  id: number;
  label: string;
  starts_on: string;
  ends_on: string;
  is_current: boolean;
  created_at: string;
}

export interface CourseRow {
  id: number;
  semester_id: number;
  code: string;
  title: string;
  credits: number;
  leverage_class: LeverageClass;
  instructor: string | null;
  target_grade: string | null;
  meeting_pattern: MeetingSlot[];
  status: CourseStatus;
  created_at: string;
}

export interface DeadlineRow {
  id: number;
  semester_id: number;
  course_id: number | null;
  title: string;
  category: DeadlineCategory;
  due_at: string;
  leverage_class: LeverageClass;
  status: DeadlineStatus;
  created_at: string;
  notes: string | null;
}

export interface DecisionRow {
  id: number;
  semester_id: number;
  decision_type: string;
  description: string;
  challenge_fired: boolean;
  challenge_reasoning: string | null;
  final_outcome: "kept" | "reversed" | "overridden" | null;
  decided_at: string;
}

export interface RankedCandidateDto {
  id: number;
  headline: string;
  reasoning: string;
}

export interface VerdictDto {
  headline: string;
  reasoning: string;
  confidence: Confidence;
  grounded_in_deadline_id: number | null;
  /** Populated only when the Closeness Threshold trips (09_DECISION_ENGINE.md §4). */
  runners_up: RankedCandidateDto[];
}

// ---------------------------------------------------------------------
// Adaptive Planner — mirrors crates/athena-app/src/commands/planner.rs
// and athena_data::repositories::disruption (08_ADAPTIVE_PLANNER.md).
// ---------------------------------------------------------------------

export type DisruptionType =
  | "external_interrupt"
  | "surprise_workload"
  | "cancelled_class"
  | "unexpected_opportunity"
  | "illness"
  | "early_finish";

export interface DisruptionRow {
  id: number;
  semester_id: number;
  date: string;
  disruption_type: DisruptionType;
  duration_minutes: number;
  affects_deep_work_window: boolean;
  linked_deadline_id: number | null;
  note: string | null;
  logged_at: string;
  recompute_triggered: boolean;
  recompute_headline: string | null;
  recompute_reasoning: string | null;
}

export interface DisruptionDto {
  id: number;
  date: string;
  disruption_type: DisruptionType;
  duration_minutes: number;
  affects_deep_work_window: boolean;
  linked_deadline_id: number | null;
  note: string | null;
  logged_at: string;
}

export interface BootstrapState {
  has_profile: boolean;
  profile: ProfileRow | null;
  current_semester: SemesterRow | null;
  courses: CourseRow[];
  deadlines: DeadlineRow[];
  career_deadlines: DeadlineRow[];
  decisions: DecisionRow[];
  verdict: VerdictDto;
  /** §3.1's `available_minutes_tonight`, after today's logged disruptions. */
  available_minutes_tonight: number;
  base_window_minutes: number;
  today_disruptions: DisruptionRow[];
  recent_disruptions: DisruptionRow[];
}

/**
 * The one read command every screen boots from (01_ARCHITECTURE.md §2.1).
 * `localDate` (`YYYY-MM-DD`, the user's local calendar day) is optional —
 * omit it to skip today's-disruption lookup (e.g. before onboarding).
 */
export async function getBootstrapState(localDate?: string): Promise<BootstrapState> {
  return invoke<BootstrapState>("get_bootstrap_state", { localDate: localDate ?? null });
}

export interface LogDisruptionInput {
  date: string;
  disruption_type: DisruptionType;
  duration_minutes: number;
  affects_deep_work_window: boolean;
  linked_deadline_id: number | null;
  note: string | null;
}

export interface ReplanResultDto {
  disruption: DisruptionDto;
  verdict: VerdictDto;
  available_minutes_tonight: number;
  base_window_minutes: number;
  substituted: boolean;
}

/** Logs one disruption and returns the Adaptive Planner's recomputed verdict (08_ADAPTIVE_PLANNER.md). */
export async function logDisruption(input: LogDisruptionInput): Promise<ReplanResultDto> {
  return invoke<ReplanResultDto>("log_disruption", { input });
}

/** The explainability trail behind every recompute (§5). */
export async function listRecentDisruptions(limit = 10): Promise<DisruptionDto[]> {
  return invoke<DisruptionDto[]>("list_recent_disruptions", { limit });
}

// ---------------------------------------------------------------------
// Daily / weekly routine questionnaires (V6 migration)
// ---------------------------------------------------------------------

export interface DailyRoutineResponseDto {
  id: number;
  date: string;
  energy_level: number;
  hours_available_tonight: number;
  had_disruption_today: boolean;
  disruption_note: string | null;
  focus_rating: number;
  submitted_at: string;
}

export interface SubmitDailyRoutineInput {
  date: string;
  energy_level: number;
  hours_available_tonight: number;
  had_disruption_today: boolean;
  disruption_note: string | null;
  focus_rating: number;
}

/** Submits today's quick check-in — energy, hours free tonight, focus. */
export async function submitDailyRoutineResponse(
  input: SubmitDailyRoutineInput,
): Promise<DailyRoutineResponseDto> {
  return invoke<DailyRoutineResponseDto>("submit_daily_routine_response", { input });
}

/** Whether `date` (`YYYY-MM-DD`) has already been answered — don't nag. */
export async function hasDailyRoutineResponse(date: string): Promise<boolean> {
  return invoke<boolean>("has_daily_routine_response", { date });
}

export async function listRecentDailyRoutineResponses(limit = 14): Promise<DailyRoutineResponseDto[]> {
  return invoke<DailyRoutineResponseDto[]>("list_recent_daily_routine_responses", { limit });
}

export interface WeeklyRoutineResponseDto {
  id: number;
  week_starting: string;
  overall_energy_trend: number;
  satisfaction_with_progress: number;
  hardest_course_id: number | null;
  biggest_blocker: string | null;
  hours_studied_estimate: number | null;
  wants_deep_work_adjustment: boolean;
  notes: string | null;
  submitted_at: string;
}

export interface SubmitWeeklyRoutineInput {
  week_starting: string;
  overall_energy_trend: number;
  satisfaction_with_progress: number;
  hardest_course_id: number | null;
  biggest_blocker: string | null;
  hours_studied_estimate: number | null;
  wants_deep_work_adjustment: boolean;
  notes: string | null;
}

/** Submits this week's longer, reflective check-in. */
export async function submitWeeklyRoutineResponse(
  input: SubmitWeeklyRoutineInput,
): Promise<WeeklyRoutineResponseDto> {
  return invoke<WeeklyRoutineResponseDto>("submit_weekly_routine_response", { input });
}

/** Whether `weekStarting` (`YYYY-MM-DD`, the week's Monday) has already been answered. */
export async function hasWeeklyRoutineResponse(weekStarting: string): Promise<boolean> {
  return invoke<boolean>("has_weekly_routine_response", { weekStarting });
}

export async function listRecentWeeklyRoutineResponses(limit = 8): Promise<WeeklyRoutineResponseDto[]> {
  return invoke<WeeklyRoutineResponseDto[]>("list_recent_weekly_routine_responses", { limit });
}

// Scheduled daily-questionnaire trigger's configurable fire time
// (routine_scheduler.rs / commands/routine.rs). `time` is `HH:MM`,
// 24-hour, matching the `<input type="time">` element's own value
// format one-to-one — no client-side reformatting needed either way.

export async function saveRoutineQuestionnaireTime(time: string): Promise<void> {
  return invoke<void>("save_routine_questionnaire_time", { time });
}

export async function getRoutineQuestionnaireTime(): Promise<string> {
  return invoke<string>("get_routine_questionnaire_time");
}

export interface CreateProfileInput {
  name: string;
  institute: string;
  program: string;
  target_cgpa: number;
  current_cgpa: number | null;
  career_target: string;
  masters_target: string | null;
  codeforces_handle: string | null;
  deep_work_window_start: string;
  deep_work_window_end: string;
  timezone: string;
}

/** Commits Profile Creation (03_ONBOARDING.md §2, Step 5). Returns the new `user_profile.id`. */
export async function createProfile(input: CreateProfileInput): Promise<number> {
  return invoke<number>("create_profile", { input });
}

export interface CourseInput {
  code: string;
  title: string;
  credits: number;
  leverage_class: LeverageClass;
  instructor: string | null;
  target_grade: string | null;
  meeting_pattern: MeetingSlot[];
}

export interface DeadlineInput {
  course_index: number | null;
  title: string;
  category: DeadlineCategory;
  due_at: string;
  leverage_class: LeverageClass;
  notes: string | null;
}

export interface CommitSemesterSetupInput {
  label: string;
  starts_on: string;
  ends_on: string;
  courses: CourseInput[];
  deadlines: DeadlineInput[];
  is_first_run: boolean;
}

/** Commits Semester Setup (03_ONBOARDING.md §3, Step 5). Returns the new `semesters.id`. */
export async function commitSemesterSetup(input: CommitSemesterSetupInput): Promise<number> {
  return invoke<number>("commit_semester_setup", { input });
}

/** Adds a single course to the *current* semester. Returns the new `courses.id`. */
export async function addCourseToSemester(input: CourseInput): Promise<number> {
  return invoke<number>("add_course_to_semester", { input });
}

export interface DeadlineCandidateInput {
  course_id: number | null;
  title: string;
  category: DeadlineCategory;
  due_at: string;
  leverage_class: LeverageClass;
  notes: string | null;
}

/** Inserts one or more pulled/normalized deadlines against the current semester. Returns their new `deadlines.id` values. */
export async function addDeadlinesToSemester(candidates: DeadlineCandidateInput[]): Promise<number[]> {
  return invoke<number[]>("add_deadlines_to_semester", { candidates });
}

// ---------------------------------------------------------------------
// Integrations — mirrors crates/athena-app/src/commands/integrations.rs
// (07_INTEGRATIONS.md). Every interface below is a 1:1 mirror of a Rust
// DTO's public fields, same "no reshaping on this side" rule as above.
// ---------------------------------------------------------------------

export type SourceKey =
  | "codeforces"
  | "leetcode"
  | "github"
  | "calendar_ics"
  | "pdf_import"
  | "csv_import"
  | "manual"
  | "gmail"
  | "google_classroom"
  | "notion";

export type SyncStatus = "disconnected" | "idle" | "syncing" | "ok" | "error";

export interface DataSourceDto {
  source_key: SourceKey;
  kind: "poll" | "import" | "always_on" | "oauth_poll";
  status: SyncStatus;
  last_synced_at: string | null;
  last_error: string | null;
  config_json: string | null;
  has_credential: boolean;
}

/** Every connector's current status (§5) — what the Connectors step boots from. */
export async function listDataSources(): Promise<DataSourceDto[]> {
  return invoke<DataSourceDto[]>("list_data_sources");
}

// --- Codeforces (§1.1) ---

export interface CodeforcesSnapshotDto {
  handle: string;
  rating: number | null;
  max_rating: number | null;
  rank: string | null;
  solved_count: number;
  fetched_at: string;
}

/** Saves the handle and syncs immediately. Never throws on a failed sync — read `status`/`last_error` off the result. */
export async function syncCodeforces(handle: string): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("sync_codeforces", { handle });
}

export async function getLatestCodeforcesSnapshot(): Promise<CodeforcesSnapshotDto | null> {
  return invoke<CodeforcesSnapshotDto | null>("get_latest_codeforces_snapshot");
}

// --- LeetCode (§1.2) ---

export interface DsaPracticeLogDto {
  handle: string;
  total_solved: number;
  easy_solved: number;
  medium_solved: number;
  hard_solved: number;
  fetched_at: string;
}

export async function syncLeetCode(handle: string): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("sync_leetcode", { handle });
}

export async function getLatestLeetCodeSnapshot(): Promise<DsaPracticeLogDto | null> {
  return invoke<DsaPracticeLogDto | null>("get_latest_leetcode_snapshot");
}

// --- GitHub (§1.3) ---

/** `token` empty/whitespace clears the stored token. Never leaves the keychain otherwise (§4). */
export async function saveGithubToken(token: string): Promise<void> {
  return invoke<void>("save_github_token", { token });
}

export async function deleteGithubToken(): Promise<void> {
  return invoke<void>("delete_github_token");
}

export interface LinkedGithubRepoDto {
  repo_full_name: string;
  added_at: string;
}

export async function linkGithubRepo(repoFullName: string): Promise<void> {
  return invoke<void>("link_github_repo", { repoFullName });
}

export async function unlinkGithubRepo(repoFullName: string): Promise<void> {
  return invoke<void>("unlink_github_repo", { repoFullName });
}

export async function listLinkedGithubRepos(): Promise<LinkedGithubRepoDto[]> {
  return invoke<LinkedGithubRepoDto[]>("list_linked_github_repos");
}

export interface ProjectStatusSnapshotDto {
  repo_full_name: string;
  commit_count_30d: number;
  open_pr_count: number;
  open_issue_count: number;
  last_commit_at: string | null;
  fetched_at: string;
}

/** Syncs every linked repo; a single repo's failure doesn't abort the rest (see Rust doc comment). */
export async function syncGithub(): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("sync_github");
}

export async function listProjectStatusSnapshots(): Promise<ProjectStatusSnapshotDto[]> {
  return invoke<ProjectStatusSnapshotDto[]>("list_project_status_snapshots");
}

// --- Gmail (§1.8, OAuth amendment) ---
//
// `start*Oauth` opens the system browser, waits for the user to
// complete consent, exchanges the code, stores tokens in the OS
// keychain, and runs the first sync — one round trip, same
// "save + sync immediately" precedent as `syncCodeforces`. Never
// throws on a failed sync; read `status`/`last_error` off the result.

/** Opens the browser for Gmail consent and runs the first sync once granted. */
export async function startGmailOauth(): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("start_gmail_oauth");
}

export async function disconnectGmail(): Promise<void> {
  return invoke<void>("disconnect_gmail");
}

export interface GmailMessageDto {
  message_id: string;
  thread_id: string | null;
  sender: string | null;
  subject: string | null;
  received_at: string | null;
  snippet: string | null;
  fetched_at: string;
}

export async function listGmailMessages(): Promise<GmailMessageDto[]> {
  return invoke<GmailMessageDto[]>("list_gmail_messages");
}

// --- Google Classroom (§1.9, OAuth amendment) ---

/** Opens the browser for Classroom consent and runs the first sync once granted. */
export async function startGoogleClassroomOauth(): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("start_google_classroom_oauth");
}

export async function disconnectGoogleClassroom(): Promise<void> {
  return invoke<void>("disconnect_google_classroom");
}

export interface ClassroomCourseDto {
  course_id: string;
  name: string;
  section: string | null;
  fetched_at: string;
}

export async function listClassroomCourses(): Promise<ClassroomCourseDto[]> {
  return invoke<ClassroomCourseDto[]>("list_classroom_courses");
}

export interface ClassroomCourseworkDto {
  coursework_id: string;
  course_id: string;
  title: string;
  due_at: string | null;
  state: string | null;
  fetched_at: string;
}

export async function listClassroomCoursework(): Promise<ClassroomCourseworkDto[]> {
  return invoke<ClassroomCourseworkDto[]>("list_classroom_coursework");
}

export interface ClassroomAnnouncementDto {
  announcement_id: string;
  course_id: string;
  text: string | null;
  posted_at: string | null;
  fetched_at: string;
}

export async function listClassroomAnnouncements(): Promise<ClassroomAnnouncementDto[]> {
  return invoke<ClassroomAnnouncementDto[]>("list_classroom_announcements");
}

// --- Notion (§1.10, OAuth amendment) ---

/** Opens the browser for Notion consent and runs the first sync once granted. */
export async function startNotionOauth(): Promise<DataSourceDto> {
  return invoke<DataSourceDto>("start_notion_oauth");
}

export async function disconnectNotion(): Promise<void> {
  return invoke<void>("disconnect_notion");
}

export interface NotionPageDto {
  page_id: string;
  title: string | null;
  url: string | null;
  parent_database_id: string | null;
  last_edited_at: string | null;
  fetched_at: string;
}

export async function listNotionPages(): Promise<NotionPageDto[]> {
  return invoke<NotionPageDto[]>("list_notion_pages");
}

// --- Calendar Import (§1.4), CSV Import (§1.6), PDF Import (§1.5) ---
//
// None of these three commands writes to the database: Semester Setup's
// wizard (where every one of these is triggered, per §1.4/§1.5/§1.6's
// own "through Semester Setup") runs before `commitSemesterSetup` ever
// creates the `semesters` row `deadlines` would need to reference. Each
// command below only parses/extracts and hands back rows already shaped
// like `DeadlineInput`, for the wizard to merge into its own local
// Deadlines-step state — reviewable, editable, removable — and commit
// the one existing way, alongside everything else (see the matching
// Rust doc comment in `commands::integrations` for the full reasoning).

/** A parsed source row, shaped exactly like `DeadlineInput` minus `course_index` (the wizard fills that in, if any). */
export interface ParsedDeadlineDto {
  title: string;
  category: DeadlineCategory;
  due_at: string;
  leverage_class: LeverageClass;
  notes: string | null;
}

/** `icsContent` is the raw `.ics` file text, already read client-side via the File API. Nothing is written to disk here — the caller stages the returned rows into the wizard's Deadlines step. */
export async function importCalendarIcs(icsContent: string): Promise<ParsedDeadlineDto[]> {
  return invoke<ParsedDeadlineDto[]>("import_calendar_ics", { icsContent });
}

export interface CsvRowDto {
  cells: Record<string, string>;
}

/** Parses only — the person still chooses which column means what before anything is staged. */
export async function previewCsvImport(csvContent: string): Promise<CsvRowDto[]> {
  return invoke<CsvRowDto[]>("preview_csv_import", { csvContent });
}

export interface CandidateAchievementDto {
  kind: "project" | "publication" | "certification";
  title: string;
  source_excerpt: string;
}

/** `pdfBase64` is the file's raw bytes, base64-encoded client-side (strip any `data:...;base64,` prefix first). Extraction only — nothing is written until the person confirms which candidates to keep and the caller stages them into the wizard's Deadlines step. */
export async function previewPdfImport(pdfBase64: string): Promise<CandidateAchievementDto[]> {
  return invoke<CandidateAchievementDto[]>("preview_pdf_import", { pdfBase64 });
}
// ---------------------------------------------------------------------
// AI layer (06_AI_ENGINE.md) — mirrors
// crates/athena-app/src/commands/ai.rs and `athena_reasoning::Recommendation`.
// Every capability below returns the identical shape: a typed verdict,
// grounded reasoning, a confidence class, the evidence IDs it cites, a
// freshness note, and its provenance (`"template"` when no LLM was
// available or configured — never a failure state, per §10's
// offline-first requirement). No screen should ever construct prompt
// text itself; these functions are the only AI-layer surface exposed
// to the frontend.
// ---------------------------------------------------------------------

/** Mirrors `athena_reasoning::Recommendation` — §11's mandatory output shape, identical across every capability below. */
export interface RecommendationDto {
  verdict: string;
  reasoning: string;
  confidence: Confidence;
  grounded_in: number[];
  data_freshness_note: string;
  /** Provenance: `"claude"`, `"ollama"`, or `"template"` (no LLM involved — still fully grounded, just less fluent). */
  source: string;
}

/** Daily Pass, on demand (§4.1) — phrases the same Priority Resolution verdict `getBootstrapState` already computes for Now. */
export async function getDailyBriefing(): Promise<RecommendationDto> {
  return invoke<RecommendationDto>("get_daily_briefing");
}

/** Weekly Digest, on demand (§4.2) — a rollup of the week's already-computed Adaptive Planner verdicts, not a new ranking. */
export async function getWeeklyPlan(): Promise<RecommendationDto> {
  return invoke<RecommendationDto>("get_weekly_plan");
}

/** Weakness Analysis, on demand (§4.4) — a presentation of already-graduated `drift_signals`/`bottlenecks` rows. Honestly returns `insufficient_data` until those tables exist in this schema. */
export async function getWeaknessAnalysis(): Promise<RecommendationDto> {
  return invoke<RecommendationDto>("get_weakness_analysis");
}

/** Saves the cloud provider's API key (§9) — stored exclusively in the OS keychain, same as the GitHub token; never persisted to SQLite. */
export async function saveAnthropicApiKey(key: string): Promise<void> {
  return invoke<void>("save_anthropic_api_key", { key });
}

export async function deleteAnthropicApiKey(): Promise<void> {
  return invoke<void>("delete_anthropic_api_key");
}

/** Whether a cloud provider key is currently configured — drives the AI settings panel's "connected" state without ever returning the key itself. */
export async function hasAnthropicApiKey(): Promise<boolean> {
  return invoke<boolean>("has_anthropic_api_key");
}

// Hugging Face token management (free tier — no billing required).
// Get a token at https://huggingface.co/settings/tokens (role: "Inference").
// Once saved, the HF provider slots in automatically after Anthropic and
// before Ollama. `source` in RecommendationDto will read "huggingface".

/** Saves the HF Inference API token to the OS keychain. Never stored in SQLite. */
export async function saveHfApiKey(key: string): Promise<void> {
  return invoke<void>("save_hf_api_key", { key });
}

export async function deleteHfApiKey(): Promise<void> {
  return invoke<void>("delete_hf_api_key");
}

/** Whether a HF token is currently configured — drives settings UI "connected" state. */
export async function hasHfApiKey(): Promise<boolean> {
  return invoke<boolean>("has_hf_api_key");
}
