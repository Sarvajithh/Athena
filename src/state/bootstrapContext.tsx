import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { getBootstrapState, type BootstrapState } from '../ipc/bindings';

interface BootstrapContextValue {
  /** `null` while the very first load is in flight, or if it failed. */
  state: BootstrapState | null;
  loading: boolean;
  error: string | null;
  /** Re-fetches from `get_bootstrap_state` — called after onboarding commits. */
  refresh: () => Promise<void>;
}

const BootstrapContext = createContext<BootstrapContextValue | undefined>(undefined);

/**
 * Wraps the whole app. This is the single IPC round trip every screen's
 * real data flows from (01_ARCHITECTURE.md §2.1's canonical read path),
 * replacing Sprint 2's per-screen static mock fixture imports.
 */
export function BootstrapProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<BootstrapState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Local calendar day, per the user's own clock — the Adaptive
      // Planner's `schedule_disruptions.date` is a local date, not UTC
      // (08_ADAPTIVE_PLANNER.md §5), so "today" is computed here rather
      // than in athena-domain/athena-data (neither takes a date/time
      // dependency — see ipc/bindings.ts's `getBootstrapState` doc comment).
      const localDate = new Date().toLocaleDateString('en-CA'); // YYYY-MM-DD
      const next = await getBootstrapState(localDate);
      setState(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const value = useMemo(() => ({ state, loading, error, refresh }), [state, loading, error, refresh]);

  return <BootstrapContext.Provider value={value}>{children}</BootstrapContext.Provider>;
}

export function useBootstrap(): BootstrapContextValue {
  const ctx = useContext(BootstrapContext);
  if (!ctx) {
    throw new Error('useBootstrap must be used within a BootstrapProvider');
  }
  return ctx;
}