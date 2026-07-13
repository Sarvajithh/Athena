# Semester View

### Purpose
The **zoomed-out map** — gives the user a sense of "where am I in the semester" without ever feeling like a wall-sized spreadsheet.

---

## Layout

### Horizontal timeline (primary)
- One continuous horizontal ribbon representing the full semester, today marked with a soft vertical marker (like a "you are here" pin) — always centered on first open.
- Each course = a **horizontal lane** (Notion-style swimlane), color matches its Dashboard tab color.
- Milestones (exams, project due dates, breaks) appear as small icon markers along each lane — no text by default, tap to reveal title/date in a small popover.
- Scrub left/right (Apple Photos-style horizontal scrub) to move through the semester; pinch to zoom between "weeks" and "months" density.

### Density levels
- **Month zoom (default landing):** lanes show only major milestones — exams, big deadlines, breaks. Deliberately sparse.
- **Week zoom:** lanes fill in with regular assignments/classes — same visual language as Weekly Planner blocks, so switching between the two feels continuous.

### Vertical "Big Picture" strip (secondary, top of screen)
- A simple horizontal bar divided into the semester's phases (e.g., "Early / Midterms / Late / Finals") — soft color gradient, today's position marked. This answers "how far through the semester am I" in one glance without dates or math.

---

## Overload protection
- Weeks with many overlapping milestones across lanes get a soft "heavy week" underline (a muted horizontal band under that week, not a color explosion) — clicking it jumps into Weekly Planner for that week pre-filtered to essentials.

## Break / rest periods
- Breaks (spring break, holidays) are rendered as a visually distinct **calm band** across all lanes — textured, lighter, clearly "protected" — reinforcing rest as a real, visible part of the semester rather than a gap in the data.

## Adding milestones
- Tap-and-hold on a lane at a point in time → quick-add popover (icon + title + date, pre-filled from tap position). Same minimal-input pattern as Weekly Planner.

---

## Navigation
- Pinch out from here → collapses to a single-row "semester pill" for use inside Dashboard's Courses card (shared visual language, so the user always recognizes the same shapes at different zoom levels).
- Pinch in on any week → drops into Weekly Planner.

---

## Inspiration notes
- **Notion**: swimlane/timeline database view as the direct structural reference.
- **Apple Photos**: pinch-to-zoom scrubbing between density levels.
- **Arc**: consistent color-tab language carried from Dashboard down to lane color.
- **Linear**: milestone markers kept minimal, icon-first, expand only on demand.
