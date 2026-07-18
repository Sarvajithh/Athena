import { useEffect, useState } from 'react';
import {
  importCalendarIcs,
  listDataSources,
  previewCsvImport,
  previewPdfImport,
  type CandidateAchievementDto,
  type CsvRowDto,
  type DataSourceDto,
  type DeadlineCategory,
  type LeverageClass,
  type ParsedDeadlineDto,
  type SourceKey,
} from '../../ipc/bindings';
import { SyncStatusBadge } from '../../components/shared/SyncStatusBadge';

/** What this step hands back to the wizard's own Deadlines-step state — every import mechanism funnels through this one shape. */
export interface StagedDeadline {
  title: string;
  category: DeadlineCategory;
  dueAt: string;
  leverageClass: LeverageClass;
  notes: string;
}

interface ImportStepProps {
  styles: Record<string, string>;
  onStageDeadlines: (rows: StagedDeadline[]) => void;
}

function fromParsed(row: ParsedDeadlineDto): StagedDeadline {
  return {
    title: row.title,
    category: row.category,
    dueAt: row.due_at,
    leverageClass: row.leverage_class,
    notes: row.notes ?? '',
  };
}

function readFileAsText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ''));
    reader.onerror = () => reject(reader.error ?? new Error('could not read file'));
    reader.readAsText(file);
  });
}

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = String(reader.result ?? '');
      // `readAsDataURL` produces "data:<mime>;base64,<payload>" — only
      // the payload is meaningful to the backend's decoder.
      const commaIndex = result.indexOf(',');
      resolve(commaIndex >= 0 ? result.slice(commaIndex + 1) : result);
    };
    reader.onerror = () => reject(reader.error ?? new Error('could not read file'));
    reader.readAsDataURL(file);
  });
}

function findSource(sources: DataSourceDto[], key: SourceKey): DataSourceDto | undefined {
  return sources.find((s) => s.source_key === key);
}

/**
 * Semester Setup's Import step (07_INTEGRATIONS.md §1.4/§1.5/§1.6).
 *
 * This step used to be called "Connectors" and also hosted every
 * account-based/OAuth connector (Codeforces, LeetCode, GitHub, Gmail,
 * Google Classroom, Notion). Those now live exclusively in Settings —
 * account connectors are a standing, revisitable relationship with an
 * external service, not a one-time semester-setup action, so they don't
 * belong inside a wizard that only runs at the start of each term. Only
 * the three *file-based* import mechanisms remain here, because the
 * wizard's Deadlines step genuinely benefits from being able to
 * bulk-seed itself from a calendar export, a transcript/resume PDF, or
 * a grade/timetable CSV during initial setup — none of those require an
 * account connection, so none of them belong in Settings.
 *
 * Connecting (or not) anything here never blocks the wizard from
 * continuing — the "Continue" button below this step has no dependency
 * on anything here succeeding.
 */
export function ImportStep({ styles, onStageDeadlines }: ImportStepProps) {
  const [sources, setSources] = useState<DataSourceDto[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    listDataSources()
      .then(setSources)
      .catch((e) => setLoadError(e instanceof Error ? e.message : String(e)));
  }, []);

  const refreshSources = () => listDataSources().then(setSources).catch(() => undefined);

  return (
    <div className={styles.form}>
      <p className="type-body">
        Optionally seed your Deadlines step from a file — each of these is optional, and Athena works the same
        either way. Account connectors (Codeforces, LeetCode, GitHub, Gmail, Google Classroom, Notion) now live in
        Settings, since they're an ongoing connection rather than a one-time setup step.
      </p>
      {loadError && <p className={`${styles.error} type-caption`}>{loadError}</p>}

      <CalendarImportPanel styles={styles} source={findSource(sources, 'calendar_ics')} onStageDeadlines={onStageDeadlines} onSynced={refreshSources} />
      <PdfImportPanel styles={styles} source={findSource(sources, 'pdf_import')} onStageDeadlines={onStageDeadlines} onSynced={refreshSources} />
      <CsvImportPanel styles={styles} source={findSource(sources, 'csv_import')} onStageDeadlines={onStageDeadlines} onSynced={refreshSources} />

      <div className={styles.repeatRow}>
        <p className="type-caption">
          Manual entry — the Courses and Deadlines steps you've already filled in — is always available, even with
          nothing above ever used.
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------
// Calendar Import (§1.4)
// ---------------------------------------------------------------------

function CalendarImportPanel({
  styles,
  source,
  onStageDeadlines,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onStageDeadlines: (rows: StagedDeadline[]) => void;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleFile = async (file: File | undefined) => {
    if (!file || busy) return;
    setBusy(true);
    setMessage(null);
    try {
      const content = await readFileAsText(file);
      const parsed = await importCalendarIcs(content);
      onStageDeadlines(parsed.map(fromParsed));
      setMessage(`Added ${parsed.length} event${parsed.length === 1 ? '' : 's'} to your Deadlines step.`);
      onSynced();
    } catch (e) {
      setMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <label className={styles.field}>
        <span className="type-caption">Import a calendar (.ics) file</span>
        <input
          className={styles.input}
          type="file"
          accept=".ics,text/calendar"
          disabled={busy}
          onChange={(e) => handleFile(e.target.files?.[0])}
        />
      </label>
      {message && <p className="type-caption">{message}</p>}
      {source && <SyncStatusBadge status={source.status} />}
    </div>
  );
}

// ---------------------------------------------------------------------
// Resume/Transcript PDF Import (§1.5)
// ---------------------------------------------------------------------

function PdfImportPanel({
  styles,
  source,
  onStageDeadlines,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onStageDeadlines: (rows: StagedDeadline[]) => void;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [candidates, setCandidates] = useState<CandidateAchievementDto[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const handleFile = async (file: File | undefined) => {
    if (!file || busy) return;
    setBusy(true);
    setError(null);
    try {
      const base64 = await readFileAsBase64(file);
      const found = await previewPdfImport(base64);
      setCandidates(found);
      setSelected(new Set(found.map((_, i) => i)));
      onSynced();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggle = (index: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const handleConfirm = () => {
    const rows: StagedDeadline[] = candidates
      .filter((_, i) => selected.has(i))
      .map((c) => ({
        title: c.title,
        category: c.kind === 'publication' ? 'research' : 'career',
        dueAt: '',
        leverageClass: 'medium',
        notes: `Imported from resume/transcript (${c.kind}): ${c.source_excerpt}`,
      }));
    onStageDeadlines(rows);
    setCandidates([]);
    setSelected(new Set());
  };

  return (
    <div className={styles.repeatRow}>
      <label className={styles.field}>
        <span className="type-caption">Import a resume or transcript (PDF)</span>
        <input className={styles.input} type="file" accept="application/pdf" disabled={busy} onChange={(e) => handleFile(e.target.files?.[0])} />
      </label>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      {candidates.length > 0 && (
        <>
          <p className="type-caption">Found {candidates.length} possible achievement(s) — confirm which to add. Each needs a date added in the Deadlines step.</p>
          <ul>
            {candidates.map((c, i) => (
              <li key={i} className="type-caption">
                <label>
                  <input type="checkbox" checked={selected.has(i)} onChange={() => toggle(i)} /> [{c.kind}] {c.title}
                </label>
              </li>
            ))}
          </ul>
          <button type="button" className={styles.secondaryButton} onClick={handleConfirm} disabled={selected.size === 0}>
            Add {selected.size} to Deadlines
          </button>
        </>
      )}
      {source && <SyncStatusBadge status={source.status} />}
    </div>
  );
}

// ---------------------------------------------------------------------
// CSV Import (§1.6)
// ---------------------------------------------------------------------

const NO_MAPPING = '__none__';

function CsvImportPanel({
  styles,
  source,
  onStageDeadlines,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onStageDeadlines: (rows: StagedDeadline[]) => void;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [rows, setRows] = useState<CsvRowDto[]>([]);
  const [headers, setHeaders] = useState<string[]>([]);
  const [titleCol, setTitleCol] = useState(NO_MAPPING);
  const [dueAtCol, setDueAtCol] = useState(NO_MAPPING);
  const [error, setError] = useState<string | null>(null);

  const handleFile = async (file: File | undefined) => {
    if (!file || busy) return;
    setBusy(true);
    setError(null);
    try {
      const content = await readFileAsText(file);
      const parsed = await previewCsvImport(content);
      setRows(parsed);
      const first = parsed[0];

      setHeaders(first ? Object.keys(first.cells) : []);
      onSynced();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const canImport = titleCol !== NO_MAPPING && dueAtCol !== NO_MAPPING && rows.length > 0;

  const handleImport = () => {
    if (!canImport) return;
    const staged: StagedDeadline[] = rows.map((row) => ({
      title: row.cells[titleCol] ?? '',
      category: 'academic',
      dueAt: row.cells[dueAtCol] ?? '',
      leverageClass: 'medium',
      notes: '',
    }));
    onStageDeadlines(staged);
    setRows([]);
    setHeaders([]);
  };

  return (
    <div className={styles.repeatRow}>
      <label className={styles.field}>
        <span className="type-caption">Import a grade/timetable export (CSV)</span>
        <input className={styles.input} type="file" accept=".csv,text/csv" disabled={busy} onChange={(e) => handleFile(e.target.files?.[0])} />
      </label>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      {headers.length > 0 && (
        <>
          <p className="type-caption">{rows.length} row(s) found. Map columns to import as deadlines:</p>
          <div className={styles.fieldRow}>
            <label className={styles.field}>
              <span className="type-caption">Title column</span>
              <select className={styles.input} value={titleCol} onChange={(e) => setTitleCol(e.target.value)}>
                <option value={NO_MAPPING}>Choose a column…</option>
                {headers.map((h) => (
                  <option key={h} value={h}>
                    {h}
                  </option>
                ))}
              </select>
            </label>
            <label className={styles.field}>
              <span className="type-caption">Due-date column</span>
              <select className={styles.input} value={dueAtCol} onChange={(e) => setDueAtCol(e.target.value)}>
                <option value={NO_MAPPING}>Choose a column…</option>
                {headers.map((h) => (
                  <option key={h} value={h}>
                    {h}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <button type="button" className={styles.secondaryButton} onClick={handleImport} disabled={!canImport}>
            Add {rows.length} to Deadlines
          </button>
        </>
      )}
      {source && <SyncStatusBadge status={source.status} />}
    </div>
  );
}
