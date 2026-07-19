import { useState } from 'react';
import { Card } from '../../components/shared/Card';
import { DensityToggle } from '../../components/shared/DensityToggle';
import { EmptyState } from '../../components/shared/EmptyState';
import { GraduationCap } from 'lucide-react';
import {
  addCourseToSemester,
  commitSemesterSetup,
  type CourseInput,
  type LeverageClass,
} from '../../ipc/bindings';
import { useBootstrap } from '../../state/bootstrapContext';
import { PullDeadlinesPanel } from './PullDeadlinesPanel';
import styles from './Semester.module.css';

interface SemesterScreenProps {
  /**
   * `first-run` when reached from `Onboarding` right after Profile
   * creation (no semester exists yet, no nav rail around it).
   * `standalone` for the always-reachable nav-rail destination this
   * screen normally is. Defaults to `standalone`, since `AppShell`
   * renders this screen with no props (workflow reform brief, Part 1).
   */
  mode?: 'first-run' | 'standalone';
  /** Called after the first semester is started, only meaningful in `first-run` mode. */
  onComplete?: () => void | Promise<void>;
}

const emptyCourseForm = () => ({
  code: '',
  title: '',
  credits: '4',
  leverageClass: 'medium' as LeverageClass,
});

/**
 * The Semester screen (workflow reform brief, Part 1): a single,
 * persistent, always-reachable place to start a new semester, add a
 * course to the active one, and pull deadlines from a connector — none
 * of it gated behind a one-time onboarding wizard. Replaces
 * `screens/SemesterSetup`'s five-step wizard entirely; the deep-work
 * window and generic CSV/PDF/ICS import steps that wizard bundled in
 * are out of scope here (deep-work window is set during Profile
 * creation and is not re-configured per-semester; generic import was
 * unrelated to "I have a new course" or "start a new term").
 *
 * Single-active-semester model: `create_semester` (called via
 * `commit_semester_setup`) already flips any previously-current
 * semester to inactive in the same transaction, and `courses`/
 * `deadlines` already carry `semester_id` — no schema change was
 * needed to support this.
 */
export default function Semester({ mode = 'standalone', onComplete }: SemesterScreenProps) {
  const { state, refresh } = useBootstrap();
  const isFirstRun = mode === 'first-run';
  const currentSemester = state?.current_semester ?? null;

  const [startingNew, setStartingNew] = useState(isFirstRun || !currentSemester);
  const [label, setLabel] = useState('');
  const [startsOn, setStartsOn] = useState('');
  const [endsOn, setEndsOn] = useState('');
  const [startError, setStartError] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);

  const [courseForm, setCourseForm] = useState(emptyCourseForm());
  const [addingCourse, setAddingCourse] = useState(false);
  const [courseError, setCourseError] = useState<string | null>(null);

  const courses = state?.courses ?? [];
  const deadlines = state?.deadlines ?? [];

  const handleStartSemester = async () => {
    if (!label.trim() || !startsOn || !endsOn || starting) return;
    setStarting(true);
    setStartError(null);
    try {
      await commitSemesterSetup({
        label: label.trim(),
        starts_on: startsOn,
        ends_on: endsOn,
        courses: [],
        deadlines: [],
        is_first_run: isFirstRun,
      });
      setLabel('');
      setStartsOn('');
      setEndsOn('');
      setStartingNew(false);
      if (isFirstRun) {
        await onComplete?.();
      } else {
        await refresh();
      }
    } catch (e) {
      setStartError(e instanceof Error ? e.message : String(e));
    } finally {
      setStarting(false);
    }
  };

  const handleAddCourse = async () => {
    if (!courseForm.code.trim() || !courseForm.title.trim() || addingCourse) return;
    setAddingCourse(true);
    setCourseError(null);
    try {
      const input: CourseInput = {
        code: courseForm.code.trim(),
        title: courseForm.title.trim(),
        credits: Number.parseInt(courseForm.credits, 10) || 0,
        leverage_class: courseForm.leverageClass,
        instructor: null,
        target_grade: null,
        meeting_pattern: [],
      };
      await addCourseToSemester(input);
      setCourseForm(emptyCourseForm());
      await refresh();
    } catch (e) {
      setCourseError(e instanceof Error ? e.message : String(e));
    } finally {
      setAddingCourse(false);
    }
  };

  // First-run, or no active semester at all: only the "start a
  // semester" form is shown — nothing else on this screen is
  // meaningful without a semester to attach it to.
  if (startingNew) {
    return (
      <div className={styles.screen}>
        {!isFirstRun && (
          <div className={styles.header}>
            <p className={`${styles.eyebrow} type-caption`}>Semester</p>
            <DensityToggle />
          </div>
        )}

        <Card className={styles.card}>
          <h2 className={`${styles.sectionTitle} type-body-medium`}>Start a new semester</h2>
          <div className={styles.form}>
            <label className={styles.field}>
              <span className="type-caption">Semester label</span>
              <input
                className={styles.input}
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder="e.g., Monsoon 2026"
              />
            </label>
            <div className={styles.fieldRow}>
              <label className={styles.field}>
                <span className="type-caption">Starts on</span>
                <input
                  className={styles.input}
                  type="date"
                  value={startsOn}
                  onChange={(e) => setStartsOn(e.target.value)}
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Ends on</span>
                <input
                  className={styles.input}
                  type="date"
                  value={endsOn}
                  onChange={(e) => setEndsOn(e.target.value)}
                />
              </label>
            </div>
            {startError && <p className={`${styles.error} type-caption`}>{startError}</p>}
            <div className={styles.actions}>
              {!isFirstRun && currentSemester && (
                <button type="button" className={styles.secondaryButton} onClick={() => setStartingNew(false)}>
                  Cancel
                </button>
              )}
              <button
                type="button"
                className={styles.primaryButton}
                onClick={handleStartSemester}
                disabled={!label.trim() || !startsOn || !endsOn || starting}
              >
                {starting ? 'Starting…' : 'Start semester'}
              </button>
            </div>
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div className={styles.screen}>
      <div className={styles.header}>
        <p className={`${styles.eyebrow} type-caption`}>Semester</p>
        <DensityToggle />
      </div>

      <Card className={styles.card}>
        <div className={styles.row}>
          <div className={styles.rowMeta}>
            <span className={`${styles.rowTitle} type-body-medium`}>{currentSemester?.label}</span>
            <span className={`${styles.rowDetail} type-caption`}>
              {currentSemester?.starts_on} – {currentSemester?.ends_on}
            </span>
          </div>
          <button type="button" className={styles.linkButton} onClick={() => setStartingNew(true)}>
            Start next semester
          </button>
        </div>
      </Card>

      <Card className={styles.card}>
        <h2 className={`${styles.sectionTitle} type-body-medium`}>Add course</h2>
        <div className={styles.form}>
          <div className={styles.fieldRow}>
            <label className={styles.field}>
              <span className="type-caption">Course code</span>
              <input
                className={styles.input}
                value={courseForm.code}
                onChange={(e) => setCourseForm((f) => ({ ...f, code: e.target.value }))}
                placeholder="e.g., CS5590"
              />
            </label>
            <label className={styles.field}>
              <span className="type-caption">Title</span>
              <input
                className={styles.input}
                value={courseForm.title}
                onChange={(e) => setCourseForm((f) => ({ ...f, title: e.target.value }))}
                placeholder="e.g., Statistical Machine Learning"
              />
            </label>
          </div>
          <div className={styles.fieldRow}>
            <label className={styles.field}>
              <span className="type-caption">Credits (optional)</span>
              <input
                className={styles.input}
                type="number"
                min="0"
                value={courseForm.credits}
                onChange={(e) => setCourseForm((f) => ({ ...f, credits: e.target.value }))}
              />
            </label>
            <label className={styles.field}>
              <span className="type-caption">Weight / leverage</span>
              <select
                className={styles.input}
                value={courseForm.leverageClass}
                onChange={(e) =>
                  setCourseForm((f) => ({ ...f, leverageClass: e.target.value as LeverageClass }))
                }
              >
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
              </select>
            </label>
          </div>
          {courseError && <p className={`${styles.error} type-caption`}>{courseError}</p>}
          <button
            type="button"
            className={styles.primaryButton}
            onClick={handleAddCourse}
            disabled={!courseForm.code.trim() || !courseForm.title.trim() || addingCourse}
          >
            {addingCourse ? 'Adding…' : 'Add course'}
          </button>
        </div>

        {courses.length > 0 && (
          <div className={styles.list}>
            {courses.map((c) => (
              <div key={c.id} className={styles.row}>
                <div className={styles.rowMeta}>
                  <span className={`${styles.rowTitle} type-body`}>
                    {c.code} — {c.title}
                  </span>
                  <span className={`${styles.rowDetail} type-caption`}>{c.credits} credits</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card className={styles.card}>
        <h2 className={`${styles.sectionTitle} type-body-medium`}>Pull deadlines</h2>
        <PullDeadlinesPanel onAdded={refresh} />
      </Card>

      <Card className={styles.card}>
        <h2 className={`${styles.sectionTitle} type-body-medium`}>This semester's deadlines</h2>
        {deadlines.length === 0 ? (
          <EmptyState
            icon={GraduationCap}
            title="No deadlines yet"
            description="Pull deadlines from a connector above, or check back once courses post assignments."
          />
        ) : (
          <div className={styles.list}>
            {deadlines.map((d) => (
              <div key={d.id} className={styles.row}>
                <div className={styles.rowMeta}>
                  <span className={`${styles.rowTitle} type-body`}>{d.title}</span>
                  <span className={`${styles.rowDetail} type-caption`}>{d.due_at}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
