import { useState } from 'react';
import Semester from '../Semester';
import { useBootstrap } from '../../state/bootstrapContext';
import { ProfileWizard } from './ProfileWizard';
import styles from './Onboarding.module.css';

interface OnboardingProps {
  /** Called once the first semester is started — the whole flow is done. */
  onComplete: () => void | Promise<void>;
}

/**
 * The pre-AppShell flow (03_ONBOARDING.md §0/§1): Profile creation,
 * then starting the first semester, continuous with no visible seam
 * and no nav rail. `App.tsx` renders this in place of `AppShell`
 * whenever `get_bootstrap_state` reports no profile, or a profile with
 * no current semester yet (e.g. the app was closed mid-onboarding,
 * after Profile creation but before a semester was started).
 *
 * The "start a semester" half of this flow is now the same persistent
 * Semester screen (`screens/Semester`) reachable later from the nav
 * rail — not a separate one-time wizard — per the workflow reform
 * brief's Part 1 (SemesterSetup's 5-step wizard is retired entirely).
 */
export function Onboarding({ onComplete }: OnboardingProps) {
  const { state, refresh } = useBootstrap();
  const [profileJustCreated, setProfileJustCreated] = useState(false);

  const hasProfile = Boolean(state?.has_profile) || profileJustCreated;

  return (
    <div className={styles.stage}>
      {hasProfile ? (
        <Semester mode="first-run" onComplete={onComplete} />
      ) : (
        <ProfileWizard
          onCreated={async () => {
            setProfileJustCreated(true);
            await refresh();
          }}
        />
      )}
    </div>
  );
}
