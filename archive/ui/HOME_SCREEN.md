# Home Screen

### Purpose
The very first thing the user sees. Answers one question only: **"What's the one thing right now?"**

Not a dashboard. Not a list. A single point of focus.

---

## Layout (top to bottom)

1. **Soft greeting + date** — small, quiet, top-left. ("Monday, July 14" — no "Good morning!" chirpiness; a simple time-of-day tone shift in background color instead: warm at morning, cool at night.)

2. **The One Thing** — center of screen, dominant.
   - A single large card showing the *next* item that needs attention (next class, next deadline, or next focus block — whichever is soonest).
   - Big icon + big time + short title (max ~4 words visible).
   - Tap → goes straight into that item's detail (no intermediate screen).

3. **Today's Shape** — a horizontal visual strip beneath the One Thing card.
   - A row of small dots/blocks representing today's items in time order (like Arc's tab dots) — no text, just shape + color by category.
   - Current time is marked with a soft vertical line moving across it in real time.
   - Tapping the strip opens Daily Planner.

4. **Load Indicator** — a single soft radial ring or bar (top-right corner), 3 states only: **Light / Steady / Full**. No numbers, no percentages. This is the "decision overload" gauge — visual only, colorblind-safe via fill pattern, not just hue.

5. **Quick Capture** — one floating button (bottom-right, thumb reach). Tapping opens a minimal one-line input to jot a task/thought with zero required fields — it gets auto-sorted later, never forces categorization now.

---

## What is deliberately NOT here
- No list of all tasks.
- No streaks, points, or motivational text.
- No unread counts, no inbox-style clutter.
- No login/setup friction — home screen is instantly usable.

---

## States

| State | Visual behavior |
|---|---|
| Nothing scheduled soon | The One Thing card shows a calm empty state — a soft shape and "Nothing right now" glyph, background lightens further. This is a *positive* state, styled as relief, not emptiness/failure. |
| Overloaded day | Load Indicator shows "Full" (soft amber fill, never red). Tapping it offers exactly one action: "Show me the essentials" → filters Daily Planner down to top 3 only. |
| Mid-task / Focus Block active | Home screen replaces the One Thing card with a minimal "Focusing on…" state and a Do Not Disturb glyph; everything else on screen dims 20%. |

---

## Motion
- The One Thing card gently cross-fades to the next item the moment the current one is done or time passes — no manual refresh needed.
- Load Indicator fill animates slowly (like a breathing motion) rather than snapping — reduces the "gauge spike" anxiety feeling.

---

## Inspiration notes
- **Apple**: Today-widget simplicity, one hero element per screen.
- **Arc**: the horizontal dot strip mirrors Arc's tab-space visualization — recognizable shape language, zero text.
- **Linear**: crisp, no-clutter status via shape/color, not paragraphs.
- **Notion**: soft card style, generous corner radius, quiet neutral background.
