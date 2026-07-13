# Weekly Planner

### Purpose
A **spatial**, low-text view of the week that lets the user see shape and balance — not a dense scheduling grid.

---

## Layout

- **7 vertical columns** (Mon–Sun), Arc-space style — each column is a soft rounded panel, today's column is very slightly raised/highlighted (not colored red, just a subtle elevation + border).
- Inside each column: **blocks, not lines of text.** Each block is a colored rounded rectangle sized roughly to duration, with only an icon + time — title text appears only in Detail Mode (see UI_GUIDELINES).
- Weekends are visually "lighter" (softer background) by default to visually cue rest — user can override per week.

## Density Toggle
- **Calm mode (default):** blocks show icon + color only. The whole week reads as a color pattern — user can judge "is this a heavy week" purely by looking at block density, without reading anything.
- **Detail mode:** blocks expand slightly to show time + short title.

## The Balance Bar
- A single horizontal bar above the grid, divided into soft segments by category (Classes / Study / Work / Personal / Free).
- No numbers — just proportion, like a stacked color bar. This replaces any "X hours of work this week" text with a shape the eye can parse instantly.

## Adding items
- Tap-and-hold on any column → a soft radial "quick add" appears with 4 icon choices (Class / Task / Study Block / Personal). One tap picks the type, a single field for title, done. No multi-step forms.
- Drag-and-drop to reschedule (Linear-style instant feedback, item snaps with a soft magnetic animation to the nearest slot).

## Overload Protection
- If a day's blocks exceed a healthy density threshold, that column's background shifts very slightly warmer (never red) and a small "breather" icon appears at the top of the column — tapping it offers to auto-suggest moving one flexible item to a lighter day. The system proposes; the user approves with one tap. Never auto-moves without consent.

## Free time
- Unscheduled time is not empty/white space that feels like failure — it's rendered as a soft textured pattern ("breathing room"), explicitly framed as intentional, protected time.

---

## Navigation
- Swipe left/right to move between weeks (physical, Apple-Calendar-like inertia).
- Pinch to zoom out → collapses into Semester View (spatial continuity between the two screens).

---

## Inspiration notes
- **Arc**: column-as-space metaphor, soft elevation for "active" context.
- **Linear**: drag-and-drop precision and snap feedback.
- **Notion**: block-based visual building, color-first categorization.
- **Apple Calendar**: swipe-week navigation, physical inertia.
