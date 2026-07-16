import { useState } from 'react';
import { CloudLightning, Sparkles } from 'lucide-react';
import { Card } from '../../components/shared/Card';
import { EmptyState } from '../../components/shared/EmptyState';
import { Icon } from '../../components/shared/Icon';
import { Timeline } from '../../components/shared/Timeline';
import { logDisruption, type DisruptionRow, type DisruptionType } from '../../ipc/bindings';
import styles from './AdaptivePlannerCard.module.css';

interface AdaptivePlannerCardProps {
  semesterActive: boolean;
  availableMinutesTonight: number;
  baseWindowMinutes: number;
  todayDisruptions: DisruptionRow[];
  recentDisruptions: DisruptionRow[];
  /** Re-fetches `get_bootstrap_state` so the recomputed verdict, updated
   * window, and refreshed disruption log all land in one place — the
   * same pattern onboarding already uses after a commit. */
  onLogged: () => Promise<void>;
}

/** Mirrors the six `disruption_type` values (08_ADAPTIVE_PLANNER.md §4). */
const DISRUPTION_TYPES: { value: DisruptionType; label: string; affectsWindowDefault: boolean }[] = [
  { value: 'external_interrupt', label: 'Interrupted (e.g. a visitor)', affectsWindowDefault: true },
  { value: 'surprise_workload', label: 'Surprise workload (e.g. pop quiz)', affectsWindowDefault: false },
  { value: 'cancelled_class', label: 'Class cancelled (freed time)', affectsWindowDefault: false },
  { value: 'unexpected_opportunity', label: 'Unexpected opportunity', affectsWindowDefault: false },
  { value: 'illness', label: "Sick — can't work tonight", affectsWindowDefault: true },
  { value: 'early_finish', label: 'Finished early (freed time)', affectsWindowDefault: false },
];

/** §4.3/§4.6: these two types add time back rather than take it away. */
const GAINS_TIME = new Set<DisruptionType>(['cancelled_class', 'early_finish']);

function formatDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString(undefined, { hour: 'numeric', minute: '2-digit', month: 'short', day: 'numeric' });
}

/**
 * The Adaptive Planner's one write surface (08_ADAPTIVE_PLANNER.md):
 * logs a `ScheduleDisruption` and immediately shows the recomputed,
 * explainable verdict it produced, plus a running log of every
 * disruption logged so far — the causal chain §5 requires be queryable
 * and auditable, not just correct under the hood.
 */
export function AdaptivePlannerCard({
  semesterActive,
  availableMinutesTonight,
  baseWindowMinutes,
  todayDisruptions,
  recentDisruptions,
  onLogged,
}: AdaptivePlannerCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [type, setType] = useState<DisruptionType>('external_interrupt');
  const [minutes, setMinutes] = useState(30);
  const [note, setNote] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [lastReasoning, setLastReasoning] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selected = DISRUPTION_TYPES.find((t) => t.value === type) ?? DISRUPTION_TYPES[0]!;
  const isOpportunity = type === 'unexpected_opportunity';

  async function handleSubmit() {
    setSubmitting(true);
    setError(null);
    try {
      const localDate = new Date().toLocaleDateString('en-CA');
      const result = await logDisruption({
        date: localDate,
        disruption_type: type,
        duration_minutes: isOpportunity ? 0 : minutes,
        affects_deep_work_window: selected.affectsWindowDefault,
        linked_deadline_id: null,
        note: note.trim() ? note.trim() : null,
      });
      setLastReasoning(result.verdict.reasoning);
      setNote('');
      await onLogged();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  const windowChanged = availableMinutesTonight !== baseWindowMinutes;

  return (
    <Card className={styles.card}>
      <div className={styles.header}>
        <div className={styles.headerLabel}>
          <Icon icon={CloudLightning} size="action" />
          <span className="type-body-medium">Adaptive planner</span>
        </div>
        <button type="button" className={styles.toggleButton} onClick={() => setExpanded((v) => !v)}>
          {expanded ? 'Close' : 'Log a disruption'}
        </button>
      </div>

      {windowChanged ? (
        <p className={`${styles.windowNote} type-caption`}>
          Tonight's window: {availableMinutesTonight} min (base {baseWindowMinutes} min) — recomputed from{' '}
          {todayDisruptions.length} disruption{todayDisruptions.length === 1 ? '' : 's'} logged today.
        </p>
      ) : null}

      {expanded ? (
        <div className={styles.form}>
          <label className={styles.field}>
            <span className="type-caption">What happened</span>
            <select
              className={styles.select}
              value={type}
              onChange={(e) => setType(e.target.value as DisruptionType)}
              disabled={!semesterActive || submitting}
            >
              {DISRUPTION_TYPES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </label>

          {!isOpportunity ? (
            <label className={styles.field}>
              <span className="type-caption">{GAINS_TIME.has(type) ? 'Minutes freed up' : 'Minutes lost'}</span>
              <input
                type="number"
                min={0}
                max={720}
                step={5}
                className={styles.numberInput}
                value={minutes}
                onChange={(e) => setMinutes(Number(e.target.value))}
                disabled={!semesterActive || submitting}
              />
            </label>
          ) : null}

          <label className={styles.field}>
            <span className="type-caption">Note (optional)</span>
            <input
              type="text"
              className={styles.textInput}
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="e.g. Friend visiting unexpectedly"
              disabled={!semesterActive || submitting}
            />
          </label>

          {error ? <p className={`${styles.error} type-caption`}>{error}</p> : null}

          <button
            type="button"
            className={styles.submitButton}
            onClick={handleSubmit}
            disabled={!semesterActive || submitting}
          >
            {submitting ? 'Recomputing…' : 'Log and recompute'}
          </button>

          {lastReasoning ? (
            <p className={`${styles.lastReasoning} type-caption`}>
              <Icon icon={Sparkles} size="inline" /> {lastReasoning}
            </p>
          ) : null}
        </div>
      ) : null}

      {recentDisruptions.length === 0 ? (
        <EmptyState
          icon={CloudLightning}
          title="No disruptions logged yet"
          description="When tonight doesn't go to plan, log it here and the recommended action recomputes with the reason shown."
        />
      ) : (
        <Timeline
          className={styles.timeline}
          entries={recentDisruptions.map((d) => ({
            key: String(d.id),
            content: (
              <div className={styles.entryBody}>
                <div className={styles.entryHeader}>
                  <span className="type-body-medium">{DISRUPTION_TYPES.find((t) => t.value === d.disruption_type)?.label ?? d.disruption_type}</span>
                  <span className={`${styles.entryTime} type-caption`}>{formatDate(d.logged_at)}</span>
                </div>
                {d.note ? <p className={`${styles.entryNote} type-caption`}>{d.note}</p> : null}
                {d.recompute_reasoning ? (
                  <p className={`${styles.entryReasoning} type-caption`}>{d.recompute_reasoning}</p>
                ) : null}
              </div>
            ),
          }))}
        />
      )}
    </Card>
  );
}