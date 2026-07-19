import { useState } from 'react';
import {
  addDeadlinesToSemester,
  listClassroomAnnouncements,
  listClassroomCoursework,
  listGmailMessages,
  listNotionPages,
  type DeadlineCandidateInput,
  type DeadlineCategory,
} from '../../ipc/bindings';
import styles from './Semester.module.css';

type ConnectorKey = 'gmail' | 'google_classroom' | 'notion';

const CONNECTOR_LABELS: Record<ConnectorKey, string> = {
  gmail: 'Gmail',
  google_classroom: 'Google Classroom',
  notion: 'Notion',
};

/** A pulled row, staged for review before it becomes a real `deadlines` row. */
interface Candidate {
  key: string;
  include: boolean;
  title: string;
  category: DeadlineCategory;
  /** `YYYY-MM-DDTHH:MM`, or '' if the source gave no date and the user must pick one. */
  dueAt: string;
  notes: string | null;
}

/**
 * Semester screen's "Pull deadlines" action (workflow reform brief,
 * Part 1, item 3). Calls only the already-wired read commands
 * (`list_gmail_messages`, `list_classroom_coursework`,
 * `list_classroom_announcements`, `list_notion_pages`) — no new sync
 * logic — and normalizes whatever comes back into reviewable, editable
 * `deadlines` candidates. Gmail/Notion carry no due date in their
 * synced shape, so those rows start with an empty date the user fills
 * in before including them; Classroom coursework already has `due_at`
 * when the source provided one.
 */
export function PullDeadlinesPanel({ onAdded }: { onAdded: () => void | Promise<void> }) {
  const [connector, setConnector] = useState<ConnectorKey>('google_classroom');
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [pulling, setPulling] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handlePull = async () => {
    setPulling(true);
    setError(null);
    try {
      let rows: Candidate[] = [];
      if (connector === 'gmail') {
        const messages = await listGmailMessages();
        rows = messages.map((m) => ({
          key: `gmail-${m.message_id}`,
          include: false,
          title: m.subject?.trim() || '(no subject)',
          category: 'other' as DeadlineCategory,
          dueAt: '',
          notes: m.snippet ?? null,
        }));
      } else if (connector === 'notion') {
        const pages = await listNotionPages();
        rows = pages.map((p) => ({
          key: `notion-${p.page_id}`,
          include: false,
          title: p.title?.trim() || '(untitled page)',
          category: 'other' as DeadlineCategory,
          dueAt: '',
          notes: p.url,
        }));
      } else {
        const [coursework, announcements] = await Promise.all([
          listClassroomCoursework(),
          listClassroomAnnouncements(),
        ]);
        const courseworkRows: Candidate[] = coursework.map((c) => ({
          key: `coursework-${c.coursework_id}`,
          include: Boolean(c.due_at),
          title: c.title,
          category: 'academic' as DeadlineCategory,
          dueAt: c.due_at ?? '',
          notes: null,
        }));
        const announcementRows: Candidate[] = announcements.map((a) => ({
          key: `announcement-${a.announcement_id}`,
          include: false,
          title: a.text?.trim() ? a.text.slice(0, 80) : '(announcement)',
          category: 'other' as DeadlineCategory,
          dueAt: '',
          notes: null,
        }));
        rows = [...courseworkRows, ...announcementRows];
      }
      setCandidates(rows);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPulling(false);
    }
  };

  const updateCandidate = (key: string, patch: Partial<Candidate>) => {
    setCandidates((rows) => rows.map((r) => (r.key === key ? { ...r, ...patch } : r)));
  };

  const includedReady = candidates.filter((c) => c.include && c.title.trim() && c.dueAt);

  const handleAdd = async () => {
    if (includedReady.length === 0 || submitting) return;
    setSubmitting(true);
    setError(null);
    try {
      const payload: DeadlineCandidateInput[] = includedReady.map((c) => ({
        course_id: null,
        title: c.title.trim(),
        category: c.category,
        due_at: c.dueAt,
        leverage_class: 'medium',
        notes: c.notes,
      }));
      await addDeadlinesToSemester(payload);
      setCandidates([]);
      await onAdded();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className={styles.form}>
      <div className={styles.connectorRow}>
        <select
          className={styles.input}
          value={connector}
          onChange={(e) => {
            setConnector(e.target.value as ConnectorKey);
            setCandidates([]);
          }}
        >
          {(Object.keys(CONNECTOR_LABELS) as ConnectorKey[]).map((key) => (
            <option key={key} value={key}>
              {CONNECTOR_LABELS[key]}
            </option>
          ))}
        </select>
        <button type="button" className={styles.secondaryButton} onClick={handlePull} disabled={pulling}>
          {pulling ? 'Pulling…' : `Pull deadlines from ${CONNECTOR_LABELS[connector]}`}
        </button>
      </div>

      {error && <p className={`${styles.error} type-caption`}>{error}</p>}

      {candidates.length === 0 && !pulling ? (
        <p className="type-caption" style={{ color: 'var(--text-tertiary)' }}>
          Nothing pulled yet. If this connector isn&apos;t connected, use Settings to connect it first — pulling
          here just reads whatever was last synced.
        </p>
      ) : (
        <div className={styles.list}>
          {candidates.map((c) => (
            <div key={c.key} className={styles.candidateRow}>
              <input
                type="checkbox"
                checked={c.include}
                onChange={(e) => updateCandidate(c.key, { include: e.target.checked })}
              />
              <div className={styles.candidateFields}>
                <input
                  className={styles.input}
                  value={c.title}
                  onChange={(e) => updateCandidate(c.key, { title: e.target.value })}
                />
                <input
                  className={styles.input}
                  type="datetime-local"
                  value={c.dueAt}
                  onChange={(e) => updateCandidate(c.key, { dueAt: e.target.value })}
                />
                {c.include && !c.dueAt && (
                  <span className={`${styles.error} type-caption`}>Pick a due date to include this one.</span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {candidates.length > 0 && (
        <button
          type="button"
          className={styles.primaryButton}
          onClick={handleAdd}
          disabled={includedReady.length === 0 || submitting}
        >
          {submitting
            ? 'Adding…'
            : `Add ${includedReady.length || ''} deadline${includedReady.length === 1 ? '' : 's'}`.trim()}
        </button>
      )}
    </div>
  );
}
