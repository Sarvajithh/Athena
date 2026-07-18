import { useEffect, useState } from 'react';
import {
  deleteGithubToken,
  disconnectGmail,
  disconnectGoogleClassroom,
  disconnectNotion,
  getLatestCodeforcesSnapshot,
  getLatestLeetCodeSnapshot,
  linkGithubRepo,
  listClassroomAnnouncements,
  listClassroomCourses,
  listClassroomCoursework,
  listDataSources,
  listGmailMessages,
  listLinkedGithubRepos,
  listNotionPages,
  saveGithubToken,
  startGmailOauth,
  startGoogleClassroomOauth,
  startNotionOauth,
  syncCodeforces,
  syncGithub,
  syncLeetCode,
  unlinkGithubRepo,
  type ClassroomAnnouncementDto,
  type ClassroomCourseDto,
  type ClassroomCourseworkDto,
  type CodeforcesSnapshotDto,
  type DataSourceDto,
  type DsaPracticeLogDto,
  type GmailMessageDto,
  type LinkedGithubRepoDto,
  type NotionPageDto,
  type SourceKey,
} from '../../ipc/bindings';
import { SyncStatusBadge } from '../../components/shared/SyncStatusBadge';

function findSource(sources: DataSourceDto[], key: SourceKey): DataSourceDto | undefined {
  return sources.find((s) => s.source_key === key);
}

/**
 * Account/OAuth connectors (07_INTEGRATIONS.md §1), relocated here from
 * Semester Setup's wizard. These are a standing relationship with an
 * external account — connecting once should keep working across every
 * future semester without being re-prompted inside a once-a-term
 * wizard — so Settings, not the wizard, is where they belong. The
 * panels themselves are unchanged, just moved; only the file-based
 * import mechanisms (calendar/.ics, PDF, CSV) stayed in Semester
 * Setup's Import step, since those genuinely feed the Deadlines step
 * during initial setup rather than being an ongoing account link.
 */
export function ConnectorsSection({ styles }: { styles: Record<string, string> }) {
  const [sources, setSources] = useState<DataSourceDto[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    listDataSources()
      .then(setSources)
      .catch((e) => setLoadError(e instanceof Error ? e.message : String(e)));
  }, []);

  const refreshSources = () => listDataSources().then(setSources).catch(() => undefined);

  return (
    <section className={styles.section}>
      <h2 className={`${styles.sectionTitle} type-body-medium`}>Connected accounts</h2>
      <p className={`${styles.sectionDescription} type-caption`}>
        Connect whichever of these you use — each is optional, and Athena works the same either way. Connecting or
        disconnecting here never affects an in-progress Semester Setup.
      </p>
      {loadError && <p className={`${styles.error} type-caption`}>{loadError}</p>}

      <CodeforcesPanel styles={styles} source={findSource(sources, 'codeforces')} onSynced={refreshSources} />
      <LeetCodePanel styles={styles} source={findSource(sources, 'leetcode')} onSynced={refreshSources} />
      <GithubPanel styles={styles} source={findSource(sources, 'github')} onSynced={refreshSources} />
      <GmailPanel styles={styles} source={findSource(sources, 'gmail')} onSynced={refreshSources} />
      <ClassroomPanel styles={styles} source={findSource(sources, 'google_classroom')} onSynced={refreshSources} />
      <NotionPanel styles={styles} source={findSource(sources, 'notion')} onSynced={refreshSources} />
    </section>
  );
}

// ---------------------------------------------------------------------
// Codeforces (§1.1)
// ---------------------------------------------------------------------

function CodeforcesPanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [handle, setHandle] = useState('');
  const [busy, setBusy] = useState(false);
  const [snapshot, setSnapshot] = useState<CodeforcesSnapshotDto | null>(null);

  useEffect(() => {
    getLatestCodeforcesSnapshot().then(setSnapshot).catch(() => undefined);
  }, [source?.last_synced_at]);

  const handleSync = async () => {
    if (!handle.trim() || busy) return;
    setBusy(true);
    try {
      await syncCodeforces(handle.trim());
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <div className={styles.fieldRow}>
        <label className={styles.field}>
          <span className="type-caption">Codeforces handle</span>
          <input
            className={styles.input}
            value={handle}
            onChange={(e) => setHandle(e.target.value)}
            placeholder="e.g., tourist"
          />
        </label>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleSync} disabled={busy || !handle.trim()}>
            {busy ? 'Syncing…' : 'Connect & sync'}
          </button>
        </div>
      </div>
      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />{' '}
          {snapshot && `Rating ${snapshot.rating ?? '—'} · ${snapshot.solved_count} solved`}
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// LeetCode (§1.2)
// ---------------------------------------------------------------------

function LeetCodePanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [handle, setHandle] = useState('');
  const [busy, setBusy] = useState(false);
  const [snapshot, setSnapshot] = useState<DsaPracticeLogDto | null>(null);

  useEffect(() => {
    getLatestLeetCodeSnapshot().then(setSnapshot).catch(() => undefined);
  }, [source?.last_synced_at]);

  const handleSync = async () => {
    if (!handle.trim() || busy) return;
    setBusy(true);
    try {
      await syncLeetCode(handle.trim());
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <div className={styles.fieldRow}>
        <label className={styles.field}>
          <span className="type-caption">LeetCode username</span>
          <input className={styles.input} value={handle} onChange={(e) => setHandle(e.target.value)} />
        </label>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleSync} disabled={busy || !handle.trim()}>
            {busy ? 'Syncing…' : 'Connect & sync'}
          </button>
        </div>
      </div>
      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />{' '}
          {snapshot && `${snapshot.total_solved} solved (${snapshot.easy_solved}E/${snapshot.medium_solved}M/${snapshot.hard_solved}H)`}
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// GitHub (§1.3)
// ---------------------------------------------------------------------

function GithubPanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [token, setToken] = useState('');
  const [repoInput, setRepoInput] = useState('');
  const [repos, setRepos] = useState<LinkedGithubRepoDto[]>([]);
  const [busy, setBusy] = useState(false);

  const refreshRepos = () => listLinkedGithubRepos().then(setRepos).catch(() => undefined);

  useEffect(() => {
    refreshRepos();
  }, []);

  const handleSaveToken = async () => {
    if (!token.trim() || busy) return;
    setBusy(true);
    try {
      await saveGithubToken(token.trim());
      setToken('');
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  const handleDisconnect = async () => {
    setBusy(true);
    try {
      await deleteGithubToken();
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  const handleAddRepo = async () => {
    if (!repoInput.trim() || busy) return;
    setBusy(true);
    try {
      await linkGithubRepo(repoInput.trim());
      setRepoInput('');
      await refreshRepos();
    } finally {
      setBusy(false);
    }
  };

  const handleRemoveRepo = async (repoFullName: string) => {
    setBusy(true);
    try {
      await unlinkGithubRepo(repoFullName);
      await refreshRepos();
    } finally {
      setBusy(false);
    }
  };

  const handleSync = async () => {
    setBusy(true);
    try {
      await syncGithub();
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <p className="type-caption">
        A read-only personal access token (repo scope, or public_repo for public repos only) — stored in your OS
        keychain, never in Athena's database.
      </p>
      <div className={styles.fieldRow}>
        <label className={styles.field}>
          <span className="type-caption">Personal access token</span>
          <input
            className={styles.input}
            type="password"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder={source?.has_credential ? 'Token saved — enter a new one to replace it' : 'ghp_…'}
          />
        </label>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleSaveToken} disabled={busy || !token.trim()}>
            Save token
          </button>
        </div>
        {source?.has_credential && (
          <div className={styles.field}>
            <span className="type-caption">&nbsp;</span>
            <button type="button" className={styles.removeButton} onClick={handleDisconnect} disabled={busy}>
              Disconnect
            </button>
          </div>
        )}
      </div>

      <div className={styles.fieldRow}>
        <label className={styles.field}>
          <span className="type-caption">Link a repo (owner/name)</span>
          <input
            className={styles.input}
            value={repoInput}
            onChange={(e) => setRepoInput(e.target.value)}
            placeholder="e.g., octocat/Hello-World"
          />
        </label>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleAddRepo} disabled={busy || !repoInput.trim()}>
            Add repo
          </button>
        </div>
      </div>

      {repos.length > 0 && (
        <ul>
          {repos.map((repo) => (
            <li key={repo.repo_full_name} className="type-caption">
              {repo.repo_full_name}{' '}
              <button type="button" className={styles.removeButton} onClick={() => handleRemoveRepo(repo.repo_full_name)} disabled={busy}>
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}

      {repos.length > 0 && (
        <button type="button" className={styles.secondaryButton} onClick={handleSync} disabled={busy}>
          Sync now
        </button>
      )}

      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// Gmail (§1.8, OAuth amendment)
// ---------------------------------------------------------------------

function GmailPanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [messages, setMessages] = useState<GmailMessageDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refreshMessages = () => listGmailMessages().then(setMessages).catch(() => undefined);

  useEffect(() => {
    if (source?.status === 'ok') refreshMessages();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source?.last_synced_at]);

  const handleConnect = async () => {
    setBusy(true);
    setError(null);
    try {
      await startGmailOauth();
      onSynced();
      await refreshMessages();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDisconnect = async () => {
    setBusy(true);
    try {
      await disconnectGmail();
      setMessages([]);
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <p className="type-caption">
        Opens your browser for Gmail consent, then syncs recent messages relevant to deadlines and coursework.
      </p>
      <div className={styles.fieldRow}>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleConnect} disabled={busy}>
            {busy ? 'Connecting…' : source?.has_credential ? 'Reconnect' : 'Connect Gmail'}
          </button>
        </div>
        {source?.has_credential && (
          <div className={styles.field}>
            <span className="type-caption">&nbsp;</span>
            <button type="button" className={styles.removeButton} onClick={handleDisconnect} disabled={busy}>
              Disconnect
            </button>
          </div>
        )}
      </div>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      {messages.length > 0 && (
        <p className="type-caption">
          {messages.length} recent message{messages.length === 1 ? '' : 's'} synced.
        </p>
      )}
      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// Google Classroom (§1.9, OAuth amendment)
// ---------------------------------------------------------------------

function ClassroomPanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [courses, setCourses] = useState<ClassroomCourseDto[]>([]);
  const [coursework, setCoursework] = useState<ClassroomCourseworkDto[]>([]);
  const [announcements, setAnnouncements] = useState<ClassroomAnnouncementDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refreshAll = () => {
    listClassroomCourses().then(setCourses).catch(() => undefined);
    listClassroomCoursework().then(setCoursework).catch(() => undefined);
    listClassroomAnnouncements().then(setAnnouncements).catch(() => undefined);
  };

  useEffect(() => {
    if (source?.status === 'ok') refreshAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source?.last_synced_at]);

  const handleConnect = async () => {
    setBusy(true);
    setError(null);
    try {
      await startGoogleClassroomOauth();
      onSynced();
      refreshAll();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDisconnect = async () => {
    setBusy(true);
    try {
      await disconnectGoogleClassroom();
      setCourses([]);
      setCoursework([]);
      setAnnouncements([]);
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <p className="type-caption">
        Opens your browser for Google Classroom consent, then syncs courses, coursework, and announcements.
      </p>
      <div className={styles.fieldRow}>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleConnect} disabled={busy}>
            {busy ? 'Connecting…' : source?.has_credential ? 'Reconnect' : 'Connect Classroom'}
          </button>
        </div>
        {source?.has_credential && (
          <div className={styles.field}>
            <span className="type-caption">&nbsp;</span>
            <button type="button" className={styles.removeButton} onClick={handleDisconnect} disabled={busy}>
              Disconnect
            </button>
          </div>
        )}
      </div>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      {(courses.length > 0 || coursework.length > 0 || announcements.length > 0) && (
        <p className="type-caption">
          {courses.length} course{courses.length === 1 ? '' : 's'}, {coursework.length} coursework item
          {coursework.length === 1 ? '' : 's'}, {announcements.length} announcement
          {announcements.length === 1 ? '' : 's'} synced.
        </p>
      )}
      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------
// Notion (§1.10, OAuth amendment)
// ---------------------------------------------------------------------

function NotionPanel({
  styles,
  source,
  onSynced,
}: {
  styles: Record<string, string>;
  source: DataSourceDto | undefined;
  onSynced: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [pages, setPages] = useState<NotionPageDto[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refreshPages = () => listNotionPages().then(setPages).catch(() => undefined);

  useEffect(() => {
    if (source?.status === 'ok') refreshPages();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [source?.last_synced_at]);

  const handleConnect = async () => {
    setBusy(true);
    setError(null);
    try {
      await startNotionOauth();
      onSynced();
      await refreshPages();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDisconnect = async () => {
    setBusy(true);
    try {
      await disconnectNotion();
      setPages([]);
      onSynced();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className={styles.repeatRow}>
      <p className="type-caption">
        Opens your browser for Notion consent, then syncs pages you've shared with the integration.
      </p>
      <div className={styles.fieldRow}>
        <div className={styles.field}>
          <span className="type-caption">&nbsp;</span>
          <button type="button" className={styles.secondaryButton} onClick={handleConnect} disabled={busy}>
            {busy ? 'Connecting…' : source?.has_credential ? 'Reconnect' : 'Connect Notion'}
          </button>
        </div>
        {source?.has_credential && (
          <div className={styles.field}>
            <span className="type-caption">&nbsp;</span>
            <button type="button" className={styles.removeButton} onClick={handleDisconnect} disabled={busy}>
              Disconnect
            </button>
          </div>
        )}
      </div>
      {error && <p className={`${styles.error} type-caption`}>{error}</p>}
      {pages.length > 0 && (
        <p className="type-caption">
          {pages.length} page{pages.length === 1 ? '' : 's'} synced.
        </p>
      )}
      {source && (
        <p className="type-caption">
          <SyncStatusBadge status={source.status} />
          {source.status === 'error' && source.last_error ? ` — ${source.last_error}` : ''}
        </p>
      )}
    </div>
  );
}
