# Dashboard

### Purpose
A single **glanceable overview** of everything active in the user's life right now — courses, deadlines, energy, and career threads — without ever feeling like a control panel.

Think: Apple Health's "summary" tab, not a business analytics tool.

---

## Layout

A grid of **4–6 soft cards**, each representing one life area. No more than 6 ever shown — extras collapse into a "More spaces" tile (Arc-style).

### Card types

1. **This Week** card
   - Shape: a 7-dot row (Mon–Sun), today highlighted with a filled ring.
   - Each dot's fill level = how full that day is (empty/half/full), color-neutral.
   - Tap → Weekly Planner.

2. **Courses** card (this semester)
   - Small stack of colored tabs, one per course, like Arc space icons.
   - A soft progress ring around each tab shows "how far through the semester" that course is.
   - Tap a tab → filtered view of that course's items.

3. **Next Deadline** card
   - Just one deadline: icon, days-remaining as a big number, muted color unless <48 hrs (then a single soft amber dot appears — not the whole card).
   - Tap → Semester View, scrolled to that item.

4. **Energy / Load** card
   - Same 3-state Light/Steady/Full ring as Home Screen, but shown as a **7-day trend** — a simple soft wave line, no axis numbers, no labels. Purely a shape: "is this week trending calmer or heavier."

5. **Career Thread** card
   - A single visual progress ring for the active long-term goal (e.g., internship applications, portfolio project).
   - Tap → Career View.

6. **Quick Wins** card (optional, appears only if relevant)
   - Shows up to 3 small, low-effort tasks the user could clear in minutes — surfaced automatically, never by user tagging. Disappears entirely if none exist (no empty "0 quick wins" clutter).

---

## Interaction
- Cards can be **reordered by long-press + drag** (Notion-style), but nothing else about them is customizable — no toggling data fields, no configuration screens. Simplicity is enforced by constraint, not choice paralysis.
- Tapping any card **zooms** into its full view (shared-element transition) rather than navigating away abruptly — keeps spatial continuity so the user doesn't feel "lost."

## What's deliberately absent
- No raw numbers-heavy stat grid.
- No red "3 overdue" banner. Overdue items are folded quietly into the relevant card with a soft "still here" dot, never a red alert.
- No ads, tips, or "did you know" content — nothing competes with the six cards.

---

## Empty / Calm States
- Start of semester (no data yet): cards show soft placeholder illustrations with one-tap "Add your first course" — never a blank grid of question marks.
- Between semesters / break: Dashboard swaps to a **Rest Mode** layout — just the Career card and a calm illustration. All academic cards hide themselves rather than show "0 items."

---

## Inspiration notes
- **Apple**: Health app's ring-and-card glanceability.
- **Arc**: space-tab metaphor for courses, zoom transitions.
- **Linear**: crisp minimal cards, information density kept intentionally low.
- **Notion**: drag-to-reorder blocks, soft neutral card backgrounds.
