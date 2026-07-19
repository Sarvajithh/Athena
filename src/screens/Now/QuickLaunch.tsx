import styles from './QuickLaunch.module.css';

interface QuickLaunchProps {
  onOpenSemester: () => void;
  onOpenDecisionLog: () => void;
}

/**
 * Section 6 — Quick Launch (05_OS_HOME.md §9): the bottom of the
 * screen, deliberately the lowest-emphasis section — small text links,
 * never buttons or icons in a toolbar.
 *
 * The full spec lists six entries. Four of them ("Log a grade," "Log
 * DSA practice," "Log deep-work outcome," "Add a deadline") each write
 * one typed row directly — but their backing tables
 * (`grade_snapshots`, `dsa_practice_log`, `deep_work_sessions`) don't
 * exist in this schema, and this change is explicitly scoped not to
 * modify storage. Rather than wire a button to a write path that
 * doesn't exist, or invent a new modal (spec's own `ModalLayer` is
 * documented as "a hard ceiling, not a starting point" — exactly two
 * named exceptions, neither of which is a quick-capture form), only
 * the two entries that are pure navigation with "no data created" are
 * implemented here — which is also exactly how §9 itself describes
 * those two: "plain navigation, no data created."
 */
export function QuickLaunch({ onOpenSemester, onOpenDecisionLog }: QuickLaunchProps) {
  return (
    <nav className={styles.launch} aria-label="Quick launch">
      <button type="button" className={`${styles.link} type-caption`} onClick={onOpenSemester}>
        Open Semester
      </button>
      <span className={styles.dot} aria-hidden="true">
        ·
      </span>
      <button type="button" className={`${styles.link} type-caption`} onClick={onOpenDecisionLog}>
        Open Decision Log
      </button>
    </nav>
  );
}
