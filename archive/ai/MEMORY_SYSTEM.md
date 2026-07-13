# MEMORY_SYSTEM.md — The Substrate Everything Else Depends On

## 1. Why Memory Is the Real Product

Anyone can build a system that answers questions well. What makes Athena a Chief of Staff instead of a chatbot is that it remembers *you* — your patterns, your commitments, your excuses — and holds you to them across time. Memory is not a feature of Athena. It is Athena.

## 2. Four Memory Types

Athena maintains four distinct kinds of memory, each with a different purpose and a different decay rate.

### 2.1 Episodic Memory — "What happened"
Raw, timestamped record of events: conversations, decisions made, commitments stated, deadlines mentioned, emotional tone at the time. This is the ground truth everything else is distilled from.

- High volume, low abstraction.
- Retained for a rolling window (e.g., a semester), then compressed into semantic memory rather than kept verbatim forever.

### 2.2 Semantic Memory — "What's true about the user"
Distilled, durable facts and generalizations extracted from episodic memory: goals, values, constraints, relationships, recurring context (e.g., "user is a founder + full-time student," "user's biggest constraint is time, not money").

- Low volume, high abstraction.
- Updated by summarization, not by every event — semantic memory should feel stable, not jumpy.

### 2.3 Procedural / Habit Memory — "What the user actually does"
Patterns of behavior inferred from repetition: when they work best, what they always postpone, how they respond to pressure, what kind of tasks they abandon. This is *behavioral*, not *stated* — habit memory tracks the gap between what the user says they'll do and what they do.

- Built from recurrence detection (see Section 4).
- This is the memory type that lets Athena say "you do this every time" — arguably its single most valuable capability.

### 2.4 Decision Memory — "What was decided, and why"
A structured log of significant decisions: the decision, the reasoning given, the alternatives considered, the predicted outcome, and (later) the actual outcome. This is what lets the Decision Engine say "last time you reasoned this way, it didn't hold up."

- Explicitly append-only. Past decisions are never edited, only annotated with outcomes.

## 3. Memory Lifecycle

```
Capture → Distill → Store → Retrieve → Reinforce/Decay
```

1. **Capture** — every interaction is logged as episodic memory with metadata: timestamp, topic, stated commitment (if any), emotional/urgency signal.
2. **Distill** — periodically (daily/weekly, see Agent docs), episodic entries are compressed: repeated facts get promoted to semantic memory, repeated behaviors get promoted to habit memory.
3. **Store** — memory is organized by *theme* (e.g., "sleep," "client X," "fundraising"), not just chronology, so retrieval can be associative.
4. **Retrieve** — when reasoning about a current situation, Athena pulls the relevant slice across all four memory types, not just the most recent conversation.
5. **Reinforce or Decay** — a pattern seen again strengthens its confidence score; a pattern not seen in a long time decays in weight but is not deleted (old context can still matter for semester-level reasoning).

## 4. Habit Detection — How Athena Learns Patterns

A habit is not declared by the user; it's *inferred* by Athena from repetition. The process:

1. **Tag every event** with a behavior category (e.g., "postponed task," "worked late," "skipped review," "made impulsive decision").
2. **Track recurrence** of each category over a rolling window.
3. **Assign a confidence score** — a pattern mentioned once is a coincidence; the same pattern 3+ times across different contexts is a habit.
4. **Distinguish context-bound vs. general habits** — "always procrastinates on emails" is different from "always procrastinates under ambiguity." The second is a more useful, more general insight and should be preferred when the evidence supports it.
5. **Surface only past the Signal Threshold** (see AI_DESIGN.md §6) — a habit is only raised proactively if it's costing the user something material.

## 5. Confidence and Contradiction Handling

Every stored belief about the user (semantic or habit memory) carries a **confidence score**, not a binary truth value. This matters because people change.

- New evidence that contradicts an existing belief doesn't overwrite it silently — it creates a **tension flag**.
- Tension flags are resolved by asking the user directly, briefly: "You used to prioritize X over Y — has that changed, or is this an exception?"
- This prevents two failure modes: Athena being stuck with a stale model of the user, and Athena flip-flopping its model based on one data point.

## 6. Forgetting and Privacy

- Episodic memory decays into summary form — raw transcripts are not kept indefinitely; they're compressed into semantic/habit memory and then the specifics can be dropped.
- The user can always ask "what do you know about me" and get an honest, legible answer — no memory should exist that the user can't inspect.
- The user can explicitly instruct Athena to forget a specific thread; this deletes it from episodic memory and re-triggers distillation so downstream semantic/habit conclusions are recalculated without it.

## 7. What Gets Written Where (Illustrative, Not Exhaustive)

| Event | Episodic | Semantic | Habit | Decision |
|---|---|---|---|---|
| "I want to launch by March" | ✓ | ✓ (goal) | | |
| Misses self-imposed deadline (3rd time) | ✓ | | ✓ (habit: underestimates timelines) | |
| Chooses to hire contractor over doing it themselves | ✓ | | | ✓ |
| Mentions being tired at 11pm repeatedly | ✓ | | ✓ (habit: works late, may affect decision quality) | |

## 8. Why This Design, Not a Simpler One

A single flat memory log would make Athena *rememberful* but not *insightful* — it could recall facts but not infer patterns. Separating memory into these four types is what allows the Trajectory, Decision, Opportunity, and Weakness Engines to each ask a different question of the same underlying history, instead of all doing keyword search over a transcript.
