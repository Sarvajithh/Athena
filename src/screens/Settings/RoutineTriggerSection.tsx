import { useState } from 'react';
import { DailyForm, WeeklyForm } from '../Now/RoutineQuestionnaireCard';
import { useBootstrap } from '../../state/bootstrapContext';

/**
 * Manual "answer now" entry point for the daily/weekly routine
 * questionnaires (Task 2's required Settings trigger), for anyone who
 * dismissed the Now-screen prompt, already answered today/this week
 * and wants to log a mid-day update, or just wants to see the forms.
 * Reuses `Now/RoutineQuestionnaireCard.tsx`'s exact `DailyForm`/
 * `WeeklyForm` — no duplicate form logic.
 */
export function RoutineTriggerSection({ styles }: { styles: Record<string, string> }) {
  const { state } = useBootstrap();
  const [mode, setMode] = useState<'daily' | 'weekly' | null>(null);
  const [justSaved, setJustSaved] = useState<'daily' | 'weekly' | null>(null);

  return (
    <section className={styles.section}>
      <h2 className={`${styles.sectionTitle} type-body-medium`}>Routine check-ins</h2>
      <p className={`${styles.sectionDescription} type-caption`}>
        The daily check-in and weekly review normally prompt on the Now screen when due. Answer either one manually
        here any time — a new answer here counts as today's (or this week's) response either way.
      </p>

      {mode === null ? (
        <div className={styles.fieldRow}>
          <button type="button" className={styles.secondaryButton} onClick={() => setMode('daily')}>
            Answer daily check-in
          </button>
          <button type="button" className={styles.secondaryButton} onClick={() => setMode('weekly')}>
            Answer weekly review
          </button>
        </div>
      ) : mode === 'daily' ? (
        <DailyForm
          onDone={() => {
            setJustSaved('daily');
            setMode(null);
          }}
          onCancel={() => setMode(null)}
        />
      ) : (
        <WeeklyForm
          courses={state?.courses ?? []}
          onDone={() => {
            setJustSaved('weekly');
            setMode(null);
          }}
          onCancel={() => setMode(null)}
        />
      )}

      {justSaved && (
        <p className="type-caption">
          {justSaved === 'daily' ? "Today's check-in" : "This week's review"} saved.
        </p>
      )}
    </section>
  );
}
