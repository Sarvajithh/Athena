import { useState } from 'react';
import type { FormEvent } from 'react';
import { Sparkles } from 'lucide-react';
import { Card } from '../../components/shared/Card';
import { Icon } from '../../components/shared/Icon';
import { createProfile } from '../../ipc/bindings';
import type { WizardStep } from '../../mock/types';
import { WizardStepShell } from '../../components/shared/WizardStepShell';
import styles from './ProfileWizard.module.css';

interface ProfileWizardProps {
  /** Called after `create_profile` commits successfully. */
  onCreated: () => void | Promise<void>;
}

interface FormState {
  name: string;
  institute: string;
  program: string;
  targetCgpa: string;
  currentCgpa: string;
  careerTarget: string;
  mastersTarget: string;
  codeforcesHandle: string;
  deepWorkStart: string;
  deepWorkEnd: string;
}

const EMPTY_FORM: FormState = {
  name: '',
  institute: '',
  program: '',
  targetCgpa: '',
  currentCgpa: '',
  careerTarget: '',
  mastersTarget: '',
  codeforcesHandle: '',
  deepWorkStart: '20:00',
  deepWorkEnd: '22:00',
};

const STEP_LABELS = ['Identity', 'Trajectory', 'Standing', 'Deep-work window', 'Confirm'];

function stepsFor(current: number): WizardStep[] {
  return STEP_LABELS.map((label, index) => ({
    id: label.toLowerCase(),
    label,
    status: index < current ? 'complete' : index === current ? 'current' : 'upcoming',
  }));
}

/**
 * Profile Creation — a one-time-per-install flow (03_ONBOARDING.md §2),
 * not reachable from the nav rail. Nothing collected here is decorative:
 * every field maps directly to a `user_profile` column
 * (04_DATA_MODEL.md §1), and no field is pre-filled with real personal
 * data — only neutral placeholder hints the person must actively type
 * over.
 */
export function ProfileWizard({ onCreated }: ProfileWizardProps) {
  const [step, setStep] = useState(0);
  const [form, setForm] = useState<FormState>(EMPTY_FORM);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const update = <K extends keyof FormState>(key: K, value: FormState[K]) =>
    setForm((prev) => ({ ...prev, [key]: value }));

  const canAdvanceFromStep = (index: number): boolean => {
    switch (index) {
      case 0:
        return form.name.trim().length > 0 && form.institute.trim().length > 0 && form.program.trim().length > 0;
      case 1:
        return form.targetCgpa.trim().length > 0 && form.careerTarget.trim().length > 0;
      case 2:
        return true; // Codeforces handle is optional (03_ONBOARDING.md §2 Step 3).
      case 3:
        return form.deepWorkStart.length > 0 && form.deepWorkEnd.length > 0;
      default:
        return true;
    }
  };

  const handleNext = () => {
    if (!canAdvanceFromStep(step)) return;
    setStep((s) => Math.min(s + 1, STEP_LABELS.length - 1));
  };

  const handleBack = () => setStep((s) => Math.max(s - 1, 0));

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      // Real system timezone, never a hardcoded guess — not collected
      // as a form field (03_ONBOARDING.md §2 asks nothing about it).
      const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
      await createProfile({
        name: form.name.trim(),
        institute: form.institute.trim(),
        program: form.program.trim(),
        target_cgpa: Number.parseFloat(form.targetCgpa),
        current_cgpa: form.currentCgpa.trim() ? Number.parseFloat(form.currentCgpa) : null,
        career_target: form.careerTarget.trim(),
        masters_target: form.mastersTarget.trim() ? form.mastersTarget.trim() : null,
        codeforces_handle: form.codeforcesHandle.trim() ? form.codeforcesHandle.trim() : null,
        deep_work_window_start: form.deepWorkStart,
        deep_work_window_end: form.deepWorkEnd,
        timezone,
      });
      await onCreated();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className={styles.wrapper}>
      <div className={styles.eyebrow}>
        <Icon icon={Sparkles} size="inline" />
        <span className="type-micro">Welcome to Athena</span>
      </div>
      <h1 className={`${styles.title} type-headline`}>Let&rsquo;s set up your profile</h1>
      <p className={`${styles.subtitle} type-body`}>
        A one-time step. This grounds every verdict Athena gives you in your actual academic standing and goals —
        nothing here is invented on your behalf.
      </p>

      <WizardStepShell steps={stepsFor(step)} />

      <Card className={styles.card}>
        <form onSubmit={handleSubmit} className={styles.form}>
          {step === 0 && (
            <>
              <label className={styles.field}>
                <span className="type-caption">Name</span>
                <input
                  className={styles.input}
                  value={form.name}
                  onChange={(e) => update('name', e.target.value)}
                  placeholder="Your full name"
                  required
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Institute</span>
                <input
                  className={styles.input}
                  value={form.institute}
                  onChange={(e) => update('institute', e.target.value)}
                  placeholder="e.g., IIT Hyderabad"
                  required
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Program</span>
                <input
                  className={styles.input}
                  value={form.program}
                  onChange={(e) => update('program', e.target.value)}
                  placeholder="e.g., B.Tech, AI"
                  required
                />
              </label>
            </>
          )}

          {step === 1 && (
            <>
              <label className={styles.field}>
                <span className="type-caption">Target CGPA</span>
                <input
                  className={styles.input}
                  type="number"
                  step="0.01"
                  min="0"
                  max="10"
                  value={form.targetCgpa}
                  onChange={(e) => update('targetCgpa', e.target.value)}
                  placeholder="e.g., 8.8"
                  required
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Current CGPA (optional — leave blank if this is your first semester)</span>
                <input
                  className={styles.input}
                  type="number"
                  step="0.01"
                  min="0"
                  max="10"
                  value={form.currentCgpa}
                  onChange={(e) => update('currentCgpa', e.target.value)}
                  placeholder="e.g., 7.9"
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Career target</span>
                <input
                  className={styles.input}
                  value={form.careerTarget}
                  onChange={(e) => update('careerTarget', e.target.value)}
                  placeholder="e.g., Quant research roles"
                  required
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Master&rsquo;s target (optional)</span>
                <input
                  className={styles.input}
                  value={form.mastersTarget}
                  onChange={(e) => update('mastersTarget', e.target.value)}
                  placeholder="e.g., MS in CS, applying Fall 2027"
                />
              </label>
            </>
          )}

          {step === 2 && (
            <label className={styles.field}>
              <span className="type-caption">Codeforces handle (optional)</span>
              <input
                className={styles.input}
                value={form.codeforcesHandle}
                onChange={(e) => update('codeforcesHandle', e.target.value)}
                placeholder="Your Codeforces handle"
              />
              <span className={`${styles.hint} type-caption`}>
                Kept on your profile. Athena does not sync or fetch anything from Codeforces on your behalf yet.
              </span>
            </label>
          )}

          {step === 3 && (
            <>
              <label className={styles.field}>
                <span className="type-caption">Deep-work window start</span>
                <input
                  className={styles.input}
                  type="time"
                  value={form.deepWorkStart}
                  onChange={(e) => update('deepWorkStart', e.target.value)}
                  required
                />
              </label>
              <label className={styles.field}>
                <span className="type-caption">Deep-work window end</span>
                <input
                  className={styles.input}
                  type="time"
                  value={form.deepWorkEnd}
                  onChange={(e) => update('deepWorkEnd', e.target.value)}
                  required
                />
              </label>
            </>
          )}

          {step === 4 && (
            <div className={styles.review}>
              <p className={`${styles.reviewLine} type-body`}>
                <strong>{form.name || 'Unnamed'}</strong> · {form.institute} · {form.program}
              </p>
              <p className={`${styles.reviewLine} type-body`}>
                Target CGPA {form.targetCgpa || '—'}
                {form.currentCgpa ? `, currently ${form.currentCgpa}` : ''} · {form.careerTarget}
              </p>
              {form.mastersTarget && <p className={`${styles.reviewLine} type-body`}>Master&rsquo;s: {form.mastersTarget}</p>}
              {form.codeforcesHandle && (
                <p className={`${styles.reviewLine} type-body`}>Codeforces: {form.codeforcesHandle}</p>
              )}
              <p className={`${styles.reviewLine} type-body`}>
                Deep-work window: {form.deepWorkStart}–{form.deepWorkEnd}
              </p>
            </div>
          )}

          {error && <p className={`${styles.error} type-caption`}>{error}</p>}

          <div className={styles.actions}>
            {step > 0 && (
              <button type="button" className={styles.secondaryButton} onClick={handleBack} disabled={submitting}>
                Back
              </button>
            )}
            {step < STEP_LABELS.length - 1 ? (
              <button
                type="button"
                className={styles.primaryButton}
                onClick={handleNext}
                disabled={!canAdvanceFromStep(step)}
              >
                Continue
              </button>
            ) : (
              <button type="submit" className={styles.primaryButton} disabled={submitting}>
                {submitting ? 'Creating profile…' : 'Create profile'}
              </button>
            )}
          </div>
        </form>
      </Card>
    </div>
  );
}
