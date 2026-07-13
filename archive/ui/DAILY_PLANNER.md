# Daily Planner

### Purpose
The **execution surface** — where the user actually works through today. Optimized for the ADHD moment of "I don't know what to do next, just tell me."

---

## Layout

### Top: "Now" strip
- A single large card, always pinned at top, showing **only** the current or next item — same visual language as Home Screen's One Thing card, but here it's persistent while scrolling.
- A soft progress ring around it fills as time passes through that block.

### Middle: Today's vertical timeline
- A single column, chronological, top to bottom.
- Each item = a compact row: icon, time, title (title only in Detail Mode — Calm mode shows icon + time only, relying on color/icon memory).
- **Only 5 rows visible without scrolling** — matches the "max 5 visible items" rule. If today has more, the rest collapse under a soft "+3 more" tap-to-reveal, never dumped all at once.
- Completed items don't disappear — they fade to 40% opacity and drop to the bottom of view, so the user gets quiet visual proof of progress without a jarring "poof."

### Bottom: Energy Check (optional, never required)
- A single row of 3–5 soft faces/shapes (not emoji, custom minimal glyphs) the user can tap once, no text entry, to log how they're feeling. Entirely optional, always skippable, never blocks anything.

---

## Focus Block Mode
- Tapping the "Now" card can start a **Focus Block**: the rest of the timeline dims to near-invisible, only the current task + a soft countdown ring remain.
- All notifications are suppressed system-wide during this mode (see UI_GUIDELINES §6).
- Ending a Focus Block shows one gentle question, visual only: "Done / Need more time / Skip" — three large tap targets, no typing.

## Reordering / Rescheduling
- Long-press + drag any row to move it later today or to another day (drag to a small "tomorrow" tab at the screen edge — Linear-style edge-drop target).
- Skipping an item never marks it "missed" in red — it just moves to tomorrow automatically with a small quiet indicator ("moved" tag, neutral grey).

## Overload-day variant
- If Load Indicator = Full, Daily Planner opens by default in a **"Top 3" filtered view**: only the three most essential items shown, everything else collapsed under "Show full day." This is the single biggest ADHD-overload intervention in the whole app — reduce the list before the user even asks.

---

## Empty state
- No items today → full screen calm illustration + one line: "Open day." No prompts to "add something," no guilt.

---

## Inspiration notes
- **Linear**: the "Now" persistent card mirrors Linear's active-issue focus pattern.
- **Apple**: Focus Mode / Do Not Disturb integration language and iconography.
- **Notion**: soft fade-to-bottom for completed items, similar to checked-off toggle lists.
- **Arc**: edge-drop targets for quick reorganization.
