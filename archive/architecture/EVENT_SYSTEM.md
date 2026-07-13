# EVENT_SYSTEM.md — Project Athena

## 1. Why an Event System at All

Two requirements from the foundational docs cannot be satisfied by plain
CRUD with function calls:

- **`NON_NEGOTIABLES.md` §4** — no decision is made silently on the user's
  behalf. This means state changes need a place to be *stopped and
  questioned* before they commit, without every single call site in the
  codebase having to remember to check five different rules.
- **`CORE_PRINCIPLES.md` #7** — drift must be caught as an early trend, not
  discovered when a screen happens to be opened. This means the system
  needs to *react* to state changes on its own initiative, independent of
  whether the user is looking at anything.

Both point to the same architectural shape: **decouple "something
happened" from "something must now check/react to it."** That's an event
system. It also happens to be the mechanism that keeps `athena-domain`
pure (`ARCHITECTURE.md` §4): rules subscribe to events/commands rather
than being called directly and unpredictably from a dozen places.

Given single-process, single-user scale, this is implemented as an
**in-process bus** (`tokio::sync::broadcast` for events, a typed
dispatcher for commands) — not an external message queue. An external MQ
would add operational surface (a broker to run, monitor, and keep alive on
a Windows desktop app) for zero benefit when there's one process and one
consumer.

## 2. Two Kinds of Messages: Commands vs. Events

This distinction is the core design decision of this document, and it's
what makes the Challenge Layer possible.

| | **Command** | **Event** |
|---|---|---|
| Meaning | "Please do this" — a requested state change | "This already happened" — a fact |
| Timing | Synchronous, interceptable *before* commit | Asynchronous, fire-and-forget *after* commit |
| Can be blocked? | Yes — interceptors can require confirmation or reject | No — by the time it fires, it's history |
| Examples | `CommitScheduleItem`, `SubmitDecision`, `MarkDeadlineDone` | `DeadlineIngested`, `DriftDetected`, `SemesterRolledOver`, `RecommendationGenerated` |

Why both are needed rather than just one: if everything were an event
(fire-and-forget), the Challenge Layer (`NON_NEGOTIABLES.md` §4,
`CORE_PRINCIPLES.md` #3) would be structurally impossible — you cannot
challenge something that already happened without it feeling like nagging
after the fact, which `CORE_PRINCIPLES.md` #3 explicitly rules out ("push
back... then respects your final call," not "push back after the fact").
If everything were a command (always interceptable, always synchronous),
background reactive work like drift scanning would have no natural trigger
and would have to poll — wasteful and it misses the "reacts continuously"
requirement in `VISION.md`.

## 3. Command Flow (Interceptable)

```
User/UI                Command Dispatcher         Interceptors            Data Layer
   │  SubmitDecision          │                        │                       │
   ├─────────────────────────▶│                        │                       │
   │                          │  run registered         │                       │
   │                          │  interceptors in order  │                       │
   │                          ├────────────────────────▶│                       │
   │                          │                        │ (Challenge Layer,      │
   │                          │                        │  Deep Work Guard,      │
   │                          │                        │  Divergence Check)     │
   │                          │◀────────────────────────┤                       │
   │                          │  result: Clear |         │                       │
   │                          │  RequiresConfirmation    │                       │
   │◀─────────────────────────┤ (with reasoning)         │                       │
   │  [if RequiresConfirmation, block for user response]  │                       │
   │  ConfirmOverride / Revise │                        │                       │
   ├─────────────────────────▶│                        │                       │
   │                          │  commit ───────────────────────────────────────▶│
   │                          │  emit resulting Event(s) to the event bus        │
```

Key property: **interceptors never silently allow or silently block** —
they return either `Clear` (nothing to say) or `RequiresConfirmation`
carrying a mandatory `reasoning` string. There is no code path that
returns a bare boolean, because a bare boolean is exactly the "passive
reminder" pattern `NON_NEGOTIABLES.md` §2 forbids at the recommendation
layer — the same discipline is applied here at the interception layer.

**Registered interceptors, in fixed evaluation order:**

1. **Deep Work Guard** (`athena-domain::deep_work`) — blocks low-leverage
   commitments inside the sacred window.
2. **Decision Challenge Layer** — evaluates the decision against current
   trajectory/drift/bottleneck state.
3. **Divergence Check** — flags (does not block) if a decision would push
   proxy-metric behavior further from trajectory metrics.

Interceptors are additive and independently testable — a new one can be
registered without touching the dispatcher or any existing interceptor,
satisfying the modularity requirement in `ARCHITECTURE.md` §1.

## 4. The Decision Challenge Layer, Concretely

This is the mechanism behind `CORE_PRINCIPLES.md` #3's example verbatim:
user submits `SubmitDecision { type: reschedule, description: "move DSA
practice to next week" }`.

1. Dispatcher routes the command through the interceptor chain.
2. The Challenge interceptor calls `athena-domain::drift` and
   `athena-domain::bottleneck` with the decision's implied state change
   applied hypothetically (never actually committed yet).
3. If the hypothetical state trips a drift or bottleneck rule (e.g. DSA
   practice already trending down for 6 days, current bottleneck is
   `weak_subject: DSA`), the interceptor returns `RequiresConfirmation`
   with `reasoning` synthesized by `athena-reasoning` from the domain
   verdict — never invented independently by the LLM (see
   `AI_PIPELINE.md` §5).
4. The UI shows a single, blocking `ChallengeDialog` — once. If the user
   confirms the original decision, revises it, or cancels, that outcome is
   final and recorded in `decisions.final_outcome`. The same decision is
   never re-challenged, honoring "then respects your final call."
5. On resolution, the command commits and a `DecisionResolved` event
   fires, which the drift scanner and bottleneck tracker both subscribe to
   (so a repeatedly-overridden challenge on the same bottleneck can itself
   become a future drift signal — but that's a *new* decision cycle later,
   not a re-litigation of this one).

## 5. Event Flow (Reactive, Non-Blocking)

Events are for the parts of the system that must notice things on their
own initiative:

| Event | Emitted by | Subscribers |
|---|---|---|
| `DeadlineIngested` | Ingestion / manual entry | Priority Resolution cache invalidation |
| `GradeSnapshotRecorded` | Grade entry | Drift scorer, Bottleneck detector |
| `CodeforcesSynced` | `athena-ingestion::codeforces` | Drift scorer |
| `DeepWorkSessionClosed` | End-of-day scheduler trigger | Deep Work allocator (feeds next day), Drift scorer (protection-rate trend) |
| `DriftDetected` | `athena-domain::drift` (via scheduled `DriftScan`) | Recommendation generator, UI (Now screen banner) |
| `BottleneckOpened` / `BottleneckResolved` | Bottleneck detector | Recommendation generator |
| `SemesterRolledOver` | Semester Setup completion | *Everything* — this is the "re-derive, don't reuse" trigger per `NON_NEGOTIABLES.md` §7; it forces a fresh bottleneck scan and clears any stale cached priority answer |
| `RecommendationGenerated` | `athena-reasoning` | UI, `event_log` |
| `DecisionResolved` | Command dispatcher (post-Challenge Layer) | `event_log`, drift scorer |
| `DataSourceStale` | Scheduler staleness check | Recommendation generator (must attach `data_freshness_note`) |

Every event is persisted to `event_log` (see `DATABASE_SCHEMA.md` §3)
regardless of subscribers — this is what makes the system's behavior
reconstructable years later, which matters for a system whose entire job
is to be trusted with uncomfortable truths (`NON_NEGOTIABLES.md` §1): if
the user ever asks "why did Athena tell me that," the answer must be
findable, not lost.

## 6. Why `DriftScan` Is Scheduler-Triggered, Not Event-Triggered

Drift is a trend property — it can't be detected from any single event
(one grade snapshot alone doesn't show a slope). So `DriftScan` runs on a
timer (daily), reading the accumulated state rather than reacting to one
change. This is a deliberate exception to "everything reactive": the
Scheduler (`MODULES.md` §7) exists specifically because some analysis is
inherently periodic, not event-shaped, and forcing it into an event
pattern would just mean the "event" is an artificial timer tick anyway —
better to name that honestly as a scheduled job.

## 7. Failure Semantics

If an interceptor or subscriber panics or errors, the **command layer
fails closed** (the state change does not commit, the user sees an
explicit error) and the **event layer fails open per-subscriber** (one
subscriber's failure — e.g. the LLM call in `RecommendationGenerated`
timing out — does not prevent other subscribers, like `event_log`
persistence, from completing). This asymmetry matters: a failed
recommendation should never look like a *silent* pass (`NON_NEGOTIABLES.md`
§10) but also should never block the user from, say, logging a grade
because the AI layer is offline (`ARCHITECTURE.md` §6, offline-first).
