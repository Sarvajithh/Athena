import { useEffect, useState } from 'react';
import { ClipboardList } from 'lucide-react';
import { Card } from '../../components/shared/Card';
import { Icon } from '../../components/shared/Icon';
import {
  hasDailyRoutineResponse,
  hasWeeklyRoutineResponse,
  submitDailyRoutineResponse,
  submitWeeklyRoutineResponse,
  type CourseRow,
} from '../../ipc/bindings';
import styles from './RoutineQuestionnaireCard.module.css';

/** `YYYY-MM-DD` for the user's local calendar day, same convention as
 * `AdaptivePlannerCard`'s `logDisruption` call. */
function localDateToday(): string {
  return new Date().toLocaleDateString('en-CA');
}

/** `YYYY-MM-DD` for the Monday of the current local week — the weekly
 * questionnaire's cadence key. */
function localWeekStart(): string {
  const now = new Date();
  const day = now.getDay(); // 0 = Sunday
  const diffToMonday = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToMonday);
  return monday.toLocaleDateString('en-CA');
}

interface RoutineQuestionnaireCardProps {
  semesterActive: boolean;
  courses: CourseRow[];
}

/**
 * Prompts the daily and/or weekly routine questionnaire when due, and
 * offers no prompt at all once both are already answered for their
 * current cadence — checked against `has_daily_routine_response` /
 * `has_weekly_routine_response` on mount so this never nags (Task 2's
 * "check an already-answered-today state before showing it").
 *
 * A manual "answer now" trigger also exists in Settings for anyone who
 * dismissed this card and wants to answer anyway.
 */
export function RoutineQuestionnaireCard({ semesterActive, courses }: RoutineQuestionnaireCardProps) {
  const [dailyDue, setDailyDue] = useState(false);
  const [weeklyDue, setWeeklyDue] = useState(false);
  const [checked, setChecked] = useState(false);
  const [mode, setMode] = useState<'daily' | 'weekly' | null>(null);

  useEffect(() => {
    let cancelled = false;
    const today = localDateToday();
    const weekStart = localWeekStart();
    Promise.all([hasDailyRoutineResponse(today), hasWeeklyRoutineResponse(weekStart)])
      .then(([dailyAnswered, weeklyAnswered]) => {
        if (cancelled) return;
        setDailyDue(!dailyAnswered);
        setWeeklyDue(!weeklyAnswered);
        setChecked(true);
      })
      .catch(() => {
        if (!cancelled) setChecked(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!checked || !semesterActive || (!dailyDue && !weeklyDue)) {
    return null;
  }

  return (
    <Card className={styles.card}>
      <div className={styles.header}>
        <div className={styles.headerLabel}>
          <Icon icon={ClipboardList} size="action" />
          <span className="type-body-medium">Quick check-in</span>
        </div>
      </div>

      {mode === null ? (
        <div className={styles.form}>
          <p className="type-caption">
            {dailyDue && weeklyDue
              ? "You haven't answered today's check-in or this week's review yet."
              : dailyDue
                ? "You haven't answered today's check-in yet."
                : "You haven't answered this week's review yet."}
          </p>
          <div className={styles.header}>
            {dailyDue && (
              <button type="button" className={styles.toggleButton} onClick={() => setMode('daily')}>
                Answer today's check-in
              </button>
            )}
            {weeklyDue && (
              <button type="button" className={styles.toggleButton} onClick={() => setMode('weekly')}>
                Answer this week's review
              </button>
            )}
          </div>
        </div>
      ) : mode === 'daily' ? (
        <DailyForm
          onDone={() => {
            setDailyDue(false);
            setMode(null);
          }}
          onCancel={() => setMode(null)}
        />
      ) : (
        <WeeklyForm
          courses={courses}
          onDone={() => {
            setWeeklyDue(false);
            setMode(null);
          }}
          onCancel={() => setMode(null)}
        />
      )}
    </Card>
  );
}

export function DailyForm({ onDone, onCancel }: { onDone: () => void; onCancel: () => void }) {
  const [energyLevel, setEnergyLevel] = useState(3);
  const [hoursAvailable, setHoursAvailable] = useState(2);
  const [hadDisruption, setHadDisruption] = useState(false);
  const [disruptionNote, setDisruptionNote] = useState('');
  const [focusRating, setFocusRating] = useState(3);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async () => {
    setSubmitting(true);
    setError(null);
    try {
      await submitDailyRoutineResponse({
        date: localDateToday(),
        energy_level: energyLevel,
        hours_available_tonight: hoursAvailable,
        had_disruption_today: hadDisruption,
        disruption_note: hadDisruption && disruptionNote.trim() ? disruptionNote.trim() : null,
        focus_rating: focusRating,
      });
      onDone();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className={styles.form}>
      <label className={styles.field}>
        <span className="type-caption">Energy today (1 low – 5 high)</span>
        <input
          type="number"
          min={1}
          max={5}
          className={styles.numberInput}
          value={energyLevel}
          onChange={(e) => setEnergyLevel(Number(e.target.value))}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">Hours you expect free tonight</span>
        <input
          type="number"
          min={0}
          max={16}
          step={0.5}
          className={styles.numberInput}
          value={hoursAvailable}
          onChange={(e) => setHoursAvailable(Number(e.target.value))}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">
          <input
            type="checkbox"
            checked={hadDisruption}
            onChange={(e) => setHadDisruption(e.target.checked)}
            disabled={submitting}
          />{' '}
          Something disrupted today
        </span>
      </label>
      {hadDisruption && (
        <label className={styles.field}>
          <span className="type-caption">What happened (optional)</span>
          <input
            type="text"
            className={styles.textInput}
            value={disruptionNote}
            onChange={(e) => setDisruptionNote(e.target.value)}
            disabled={submitting}
          />
        </label>
      )}
      <label className={styles.field}>
        <span className="type-caption">Focus so far today (1 low – 5 high)</span>
        <input
          type="number"
          min={1}
          max={5}
          className={styles.numberInput}
          value={focusRating}
          onChange={(e) => setFocusRating(Number(e.target.value))}
          disabled={submitting}
        />
      </label>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      <div className={styles.header}>
        <button type="button" className={styles.submitButton} onClick={handleSubmit} disabled={submitting}>
          {submitting ? 'Saving…' : 'Save check-in'}
        </button>
        <button type="button" className={styles.toggleButton} onClick={onCancel} disabled={submitting}>
          Not now
        </button>
      </div>
    </div>
  );
}

export function WeeklyForm({
  courses,
  onDone,
  onCancel,
}: {
  courses: CourseRow[];
  onDone: () => void;
  onCancel: () => void;
}) {
  const [energyTrend, setEnergyTrend] = useState(3);
  const [satisfaction, setSatisfaction] = useState(3);
  const [hardestCourseId, setHardestCourseId] = useState<string>('');
  const [biggestBlocker, setBiggestBlocker] = useState('');
  const [hoursStudied, setHoursStudied] = useState<string>('');
  const [wantsAdjustment, setWantsAdjustment] = useState(false);
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async () => {
    setSubmitting(true);
    setError(null);
    try {
      await submitWeeklyRoutineResponse({
        week_starting: localWeekStart(),
        overall_energy_trend: energyTrend,
        satisfaction_with_progress: satisfaction,
        hardest_course_id: hardestCourseId ? Number.parseInt(hardestCourseId, 10) : null,
        biggest_blocker: biggestBlocker.trim() ? biggestBlocker.trim() : null,
        hours_studied_estimate: hoursStudied.trim() ? Number.parseFloat(hoursStudied) : null,
        wants_deep_work_adjustment: wantsAdjustment,
        notes: notes.trim() ? notes.trim() : null,
      });
      onDone();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className={styles.form}>
      <label className={styles.field}>
        <span className="type-caption">Overall energy trend this week (1 low – 5 high)</span>
        <input
          type="number"
          min={1}
          max={5}
          className={styles.numberInput}
          value={energyTrend}
          onChange={(e) => setEnergyTrend(Number(e.target.value))}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">Satisfaction with your progress (1 low – 5 high)</span>
        <input
          type="number"
          min={1}
          max={5}
          className={styles.numberInput}
          value={satisfaction}
          onChange={(e) => setSatisfaction(Number(e.target.value))}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">Hardest course this week (optional)</span>
        <select
          className={styles.select}
          value={hardestCourseId}
          onChange={(e) => setHardestCourseId(e.target.value)}
          disabled={submitting}
        >
          <option value="">None in particular</option>
          {courses.map((c) => (
            <option key={c.id} value={c.id}>
              {c.code} — {c.title}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span className="type-caption">Biggest blocker this week (optional)</span>
        <input
          type="text"
          className={styles.textInput}
          value={biggestBlocker}
          onChange={(e) => setBiggestBlocker(e.target.value)}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">Roughly how many hours did you study? (optional)</span>
        <input
          type="number"
          min={0}
          step={0.5}
          className={styles.numberInput}
          value={hoursStudied}
          onChange={(e) => setHoursStudied(e.target.value)}
          disabled={submitting}
        />
      </label>
      <label className={styles.field}>
        <span className="type-caption">
          <input
            type="checkbox"
            checked={wantsAdjustment}
            onChange={(e) => setWantsAdjustment(e.target.checked)}
            disabled={submitting}
          />{' '}
          I'd like to adjust my deep-work window
        </span>
      </label>
      <label className={styles.field}>
        <span className="type-caption">Anything else (optional)</span>
        <input
          type="text"
          className={styles.textInput}
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          disabled={submitting}
        />
      </label>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      <div className={styles.header}>
        <button type="button" className={styles.submitButton} onClick={handleSubmit} disabled={submitting}>
          {submitting ? 'Saving…' : 'Save review'}
        </button>
        <button type="button" className={styles.toggleButton} onClick={onCancel} disabled={submitting}>
          Not now
        </button>
      </div>
    </div>
  );
}
