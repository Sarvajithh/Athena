# FOLDER_STRUCTURE.md — Project Athena

## 0. Guiding Rule

The folder structure exists to make the layering in `ARCHITECTURE.md` §4
physically true, not just conceptually true. If it's easy to import domain
logic directly into a React component, someone will eventually do it under
deadline pressure, and the 5-year decoupling guarantee quietly dies. So the
structure below enforces boundaries with actual compiler/build barriers
(separate Rust crates, no cross-imports from `src/` into `src-tauri/`
except through generated IPC bindings) rather than relying on discipline.

## 1. Top Level

```
athena/
├── src/                      # React + TypeScript frontend
├── src-tauri/                # Rust backend (Tauri host + all domain logic)
├── docs/                     # This documentation set, kept in the repo
├── migrations/               # SQLite schema migrations (source of truth)
├── backups/                  # Local rolling backups (gitignored, runtime-only)
├── scripts/                  # Dev/build/release scripts
├── tests/                    # Cross-cutting integration tests (Rust + Playwright)
├── package.json
├── tauri.conf.json
└── README.md
```

Why `migrations/` sits at the top level rather than nested inside
`src-tauri/`: the database schema is a contract that outlives any
particular Rust module structure. Treating it as a top-level, versioned
artifact (see `DATABASE_SCHEMA.md`) makes it easy to reason about schema
evolution independent of code reorganization.

## 2. `src-tauri/` — Rust Backend

```
src-tauri/
├── Cargo.toml                # Workspace root
├── crates/
│   ├── athena-domain/         # Pure domain logic — NO I/O, NO tauri, NO sqlx
│   │   ├── src/
│   │   │   ├── priority/       # Priority resolution algorithm
│   │   │   ├── bottleneck/     # Bottleneck detection rules
│   │   │   ├── drift/          # Drift scoring
│   │   │   ├── deep_work/      # Deep-work guard + allocator
│   │   │   ├── divergence/     # Proxy-vs-trajectory metric divergence check
│   │   │   └── lib.rs
│   │   └── tests/              # Exhaustive unit tests — this crate has the
│   │                           # highest test-coverage bar in the project
│   │
│   ├── athena-data/            # Repository layer — owns all SQL
│   │   ├── src/
│   │   │   ├── repositories/    # One repository per aggregate (see MODULES.md)
│   │   │   ├── migrations_runner.rs
│   │   │   └── lib.rs
│   │
│   ├── athena-events/          # Event bus + command dispatcher
│   │   ├── src/
│   │   │   ├── commands/        # Interceptable, synchronous state changes
│   │   │   ├── events/          # Fire-and-forget async notifications
│   │   │   ├── bus.rs
│   │   │   └── lib.rs
│   │
│   ├── athena-reasoning/       # AI orchestration layer
│   │   ├── src/
│   │   │   ├── retrieval.rs     # Pulls grounded facts from athena-data
│   │   │   ├── synthesis.rs     # LLM prompt construction + call
│   │   │   ├── grounding.rs     # Validates every claim traces to a fact
│   │   │   ├── confidence.rs
│   │   │   ├── local_model.rs   # Offline fallback path
│   │   │   └── lib.rs
│   │
│   ├── athena-ingestion/       # External data connectors
│   │   ├── src/
│   │   │   ├── codeforces/
│   │   │   ├── ics_import/
│   │   │   ├── csv_import/
│   │   │   └── lib.rs
│   │
│   └── athena-app/             # The Tauri binary — wires everything together
│       ├── src/
│       │   ├── commands.rs      # #[tauri::command] bindings exposed to React
│       │   ├── scheduler.rs     # Background jobs (drift scans, staleness checks)
│       │   ├── main.rs
│       │   └── tray.rs          # Windows tray/notification integration
│       └── icons/
│
└── tests/                      # Cross-crate integration tests
```

Why a **Cargo workspace with separate crates** rather than one big crate
with modules: crate boundaries in Rust are enforced by the compiler —
`athena-domain` *physically cannot* import `sqlx` or `tauri` unless someone
deliberately adds that dependency to its `Cargo.toml`, which is a visible,
reviewable change. A `mod` boundary inside one crate is much easier to
erode silently over years of edits. This is the same reasoning as §4 in
`ARCHITECTURE.md`, made concrete.

## 3. `src/` — React Frontend

```
src/
├── screens/
│   ├── Now/                    # Primary screen — priority resolution + bottleneck
│   ├── Trajectory/             # CGPA / DSA / projects / research trends
│   ├── SemesterSetup/          # Re-derivation wizard
│   └── DecisionLog/            # History of decisions + challenges
│
├── components/                 # Shared, presentation-only components
│   ├── RecommendationCard/      # Renders {verdict, reason, confidence, grounding}
│   ├── ConfidenceBadge/         # Visual treatment for confidence/staleness
│   ├── ChallengeDialog/         # Blocking confirmation UI for Challenge Layer
│   └── TrendChart/
│
├── ipc/                        # Typed wrappers around Tauri `invoke()` calls —
│                               # the ONLY place allowed to call `invoke`
│   ├── priority.ts
│   ├── trajectory.ts
│   ├── decisions.ts
│   └── semester.ts
│
├── state/                      # Light client state (React Query / Zustand) —
│                               # caches server state, holds NO business logic
├── design/                     # Design tokens, theme — see frontend-design skill
├── App.tsx
└── main.tsx
```

Why `ipc/` is a hard chokepoint: every domain fact the UI renders must have
come from a Rust command response, never from client-side computation. This
keeps `NON_NEG §5` (grounded in reality) enforceable — it is structurally
impossible for a component to invent a CGPA trend line, because the only
way to get one is through a typed function that hits Rust.

Why no `screens/Settings/` sprawl: per `CORE_PRINCIPLES.md` #11, the
surface stays deliberately small. Configuration that does exist (LLM
provider choice, notification behavior) lives inside `SemesterSetup` or a
single compact preferences panel, not a dedicated screen family.

## 4. `migrations/`

```
migrations/
├── 0001_init.sql
├── 0002_semesters.sql
├── 0003_trajectory_metrics.sql
├── ...
└── README.md   # Migration philosophy: additive-only, never destructive,
                # see DATABASE_SCHEMA.md §7
```

## 5. `docs/`

```
docs/
├── ARCHITECTURE.md
├── DATABASE_SCHEMA.md
├── FOLDER_STRUCTURE.md
├── MODULES.md
├── AI_PIPELINE.md
├── EVENT_SYSTEM.md
├── API_INTEGRATIONS.md
└── foundational/            # Copies of VISION.md, NON_NEGOTIABLES.md,
                              # CORE_PRINCIPLES.md, PROJECT_SCOPE.md,
                              # USER_PROFILE.md — the source of truth these
                              # architecture docs must never contradict
```

Keeping the five foundational documents inside the repo (not just in the
user's head or a separate notes app) means every future architectural
decision can be checked against them in the same PR review, not from
memory.

## 6. `backups/`

```
backups/
└── athena-YYYY-MM-DD-HHmm.sqlite
```

Rolling local backups, gitignored, pruned to a retention window (e.g. last
30 daily + last 12 monthly). This directory is the concrete implementation
of "sole ownership" (NON_NEG §8) meeting "data durability" (ARCHITECTURE.md
§6) — the user's only copy of their trajectory data is never solely a
single unbacked file.
