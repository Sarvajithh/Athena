# Project Athena — UI Guidelines
### Designed for one brain, one semester, one calm surface.

This is not a productivity app. It is a **low-friction nervous system** for a student who gets overwhelmed by too many choices at once. Every rule below exists to answer one question before any other: *does this reduce decision load, or add to it?*

---

## 1. Design Philosophy

**One primary action per screen.** If two things compete for attention, one of them is wrong.

**Silence is a feature.** Empty space is not "unfinished" — it is the product working correctly. When in doubt, remove, don't add.

**Show state, don't ask questions.** The UI should tell the user what's true ("2 things today") rather than prompt them to decide something ("What would you like to do?").

**Nothing shouts.** No red badges, no exclamation marks, no urgency language. Overload is communicated through *softness fading to focus*, never alarm.

**Reversible over risky.** Every action can be undone. This removes the micro-anxiety of "am I allowed to do this."

---

## 2. Inspiration Map

| Source | What we borrow |
|---|---|
| **Apple (HIG)** | Restraint, generous whitespace, large legible type, physical/tactile motion, "it just works" defaults |
| **Arc Browser** | Spaces as context-switching, calm color-coded organization, sidebar that hides until needed, delight in small transitions |
| **Linear** | Speed, keyboard-first flow, monochrome-first UI with color used only as signal, crisp status indicators, zero decoration |
| **Notion** | Flexible visual blocks, soft neutral palette, toggling between density levels (simple ↔ detailed) |

We are **not** copying feature sets — we are copying *restraint*.

---

## 3. Visual Language

### Color
- Base palette: **neutral, low-saturation** (warm greys / soft off-white or true dark mode charcoal).
- One **accent color** for "the one thing that matters right now" — nothing else uses it.
- Category colors (for classes, projects) are **muted pastels**, never neon, never pure red.
- Red is reserved *only* for something the user must physically not miss (e.g., an exam in 2 hours) — used as a **dot**, never a wall of color.
- No color = no data yet. Grey means "not started," not "bad."

### Typography
- One typeface family, two weights max (Regular, Semibold).
- Large type by default — headlines read like a glance, not a paragraph.
- Body copy is avoided wherever an icon, shape, or spatial position can replace it.
- Numbers (counts, times) are the largest thing on any screen — they are the fastest thing a stressed brain can parse.

### Iconography over words
- Every recurring concept (class, task, deadline, energy level, focus block) gets **one consistent glyph**. The user should recognize meaning by shape before reading anything.
- Text labels appear only on first-use or on tap-and-hold ("tooltip on demand"), not by default.

### Spacing
- Minimum 24px breathing room around any tappable element.
- Card-based layout; cards never touch. A crowded screen is treated as a bug.
- Max **5 visible items** per view before the rest collapses into "+N more" — the system decides what's foldable, not the user.

### Motion
- Transitions are slow and physical (250–400ms ease-out), never snappy or bouncy — bounce reads as "urgent," which we avoid.
- New items *fade + rise* into place, they never slide in fast from an edge (that reads as a notification/interruption).
- Completing a task has a small, satisfying, quiet animation — reward without noise.

---

## 4. Interaction Principles

1. **Default to the smallest possible view.** Detail is opt-in (tap to expand), never forced.
2. **No modal pop-ups for planning decisions.** Modals interrupt; Athena never interrupts. Prompts appear inline, dismissible with one tap.
3. **No streaks, no guilt mechanics, no red "overdue" counters.** Missed items resurface gently, reframed as "still here" — not as failure.
4. **Undo everywhere.** Delete, reschedule, and skip are all one tap to reverse.
5. **Search/command bar (Linear-style) is the escape hatch** for power moments, but is never required for normal use.
6. **The app never asks "how are you feeling today" as text input.** Feelings are captured via a single tap on a 3–5 state visual scale (see Daily Planner), never a text box — typing is friction.

---

## 5. Information Density Levels

Every major screen supports two states, toggled with a single control (top-right, consistent placement across app):

- **Calm mode (default):** icons, counts, color states only.
- **Detail mode:** adds labels, times, and sub-items — opt-in, remembers last choice per screen.

---

## 6. Notification Philosophy

- Notifications are **batched**, delivered at most twice a day (morning glance, evening wind-down), never as a constant drip.
- No badge numbers that grow unbounded. Badges cap visually at a soft shape ("a few things") rather than an exact anxiety-inducing number once past a threshold.
- Nothing pings while the user is in a Focus Block (see Daily Planner).

---

## 7. Accessibility & Safety Nets

- All color-based status also has a shape/icon equivalent (colorblind-safe).
- Text scales with system settings without breaking layout (cards reflow, never truncate critical info).
- Everything works one-handed on mobile; primary actions sit in thumb-reach zone.

---

## 8. The One Test

Before shipping any screen, ask:
> **"If I glance at this for one second while overwhelmed, do I know what to do next?"**

If the answer isn't an immediate yes, the screen has too much on it.
