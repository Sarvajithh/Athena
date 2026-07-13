# TRAJECTORY_ENGINE.md — Projecting the Path, Not the Plan

## 1. Core Idea

Most planning tools track the *plan*. The Trajectory Engine tracks the *path* — where the user is actually headed if current habits, decisions, and rates of progress simply continue, independent of what the user intends. The gap between plan and path is the single most useful thing Athena can surface.

> Plans describe intentions. Trajectories describe momentum. Athena is in the momentum business.

## 2. Inputs

The Trajectory Engine draws on all four memory types, but weights them differently than other engines:

- **Habit memory** (heaviest weight) — actual behavior over time is the best predictor of future behavior.
- **Decision memory** — past decisions and their real outcomes, not just intentions.
- **Semantic memory** — the stated goal, used as the *comparison point*, not the projection input.
- **Episodic memory** — recent events, used to detect whether momentum is accelerating or decaying.

## 3. What "Projecting Forward" Means, Conceptually

Given the current rate and direction of behavior (not stated goals), the engine reasons:

1. **Rate** — at the current pace, when (if ever) does the stated goal actually get reached?
2. **Direction** — is recent behavior converging toward the goal, diverging from it, or orthogonal to it entirely?
3. **Volatility** — is progress steady, or lurching (bursts of intense work followed by long stalls)? Volatile trajectories are flagged even if the average rate looks fine, because volatility itself predicts burnout or missed windows.
4. **Compounding effects** — small recurring habits (a skipped weekly review, a chronically late start) are modeled as compounding, not one-off — this is what lets Athena say "at this rate, you're 3 months behind your own deadline" instead of just "you missed a task."

## 4. Output Format

The Trajectory Engine doesn't output a number for its own sake — it outputs a **comparison**: stated destination vs. projected destination, with the size of the gap and the specific behavior driving it.

Example of the kind of judgment this produces (illustrative, not a template to fill mechanically):
> "You said you wanted to launch by March. Based on the last six weeks of actual output, the projected date is late May. The main driver is that Tuesdays and Wednesdays consistently produce zero deep-work hours — not a lack of time overall."

## 5. When the Trajectory Engine Runs

- Lightweight version: Daily Agent (single-question check: "does today still point at this week's target?")
- Medium version: Weekly Agent (does this week's pattern still point at the stated goal?)
- Full version: Semester Agent (does the whole period's trajectory match the stated life/work direction?)

## 6. Distinguishing Signal from Noise

A single bad week is not a trajectory change. The engine requires:
- **Sustained deviation** — at least 2–3 review cycles at the relevant cadence showing the same direction of drift, OR
- **Structural deviation** — a single change large enough to mechanically alter the outcome (e.g., losing a key collaborator, a stated goal quietly dropped from three consecutive weekly reviews).

This mirrors the Signal Threshold in AI_DESIGN.md — the Trajectory Engine is conservative about declaring drift, because false alarms destroy the credibility that makes real alarms worth listening to.

## 7. Relationship to the Other Engines

- Trajectory Engine answers "where is this headed."
- Decision Engine answers "was this specific choice sound."
- Weakness Engine answers "what recurring flaw is driving the bad decisions."
- Opportunity Engine answers "what upside is being left on the table given this trajectory."

Trajectory is the integrator — it's the one engine that looks at the *combined effect* of all decisions and habits over time, rather than any single instance.
