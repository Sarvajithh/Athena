import { DensityToggle } from '../../components/shared/DensityToggle';
import { AdaptivePlannerCard } from './AdaptivePlannerCard';
import { AiInsightCard } from './AiInsightCard';
import { DeepWorkAllocationCard } from './DeepWorkAllocationCard';
import { HealthStrip } from './HealthStrip';
import { MissionStrip } from './MissionStrip';
import styles from './Now.module.css';
import { QuickLaunch } from './QuickLaunch';
import { RoutineQuestionnaireCard } from './RoutineQuestionnaireCard';
import { VerdictCard } from './VerdictCard';

import { LoadingState } from '../../components/shared/LoadingState';
import { useBootstrap } from '../../state/bootstrapContext';
import { useNavigation } from '../../state/navigationContext';

function formatWindow(start: string, end: string): string {
  const format = (hhmm: string) => {
    const parts = hhmm.split(':');
    const h = Number.parseInt(parts[0] ?? '0', 10);
    const m = parts[1] ?? '00';
    const period = h >= 12 ? 'PM' : 'AM';
    const hour12 = h % 12 === 0 ? 12 : h % 12;
    return `${hour12}:${m} ${period}`;
  };
  return `Tonight's deep-work window · ${format(start)}–${format(end)}`;
}

/**
 * Now — the Athena OS Home screen (05_OS_HOME.md). Answers one
 * question: what's the single highest-leverage thing right now, and
 * why (§1's governing test). Every section renders from
 * `get_bootstrap_state` — real persisted profile/semester/deadline
 * data — with the verdict itself computed by the deterministic
 * `athena-domain::priority`/`athena-domain::planner` engine
 * (09_DECISION_ENGINE.md; 08_ADAPTIVE_PLANNER.md). No mock values
 * anywhere in this tree.
 *
 * Structural hierarchy, top to bottom, matches §2 exactly, with one
 * addition below §1 for the Adaptive Planner (08_ADAPTIVE_PLANNER.md):
 *   0. Mission strip            — always shown, real `user_profile` fields
 *   1. Recommended Action       — the dominant verdict card
 *   1a. AI insight              — on-demand, collapsed by default (new)
 *   1b. Adaptive planner        — log a disruption, see the recompute (new)
 *   2. Weakness Snapshot        — intentionally omitted, see note below
 *   3. Today's Intelligence     — intentionally omitted, see note below
 *   4. Health Strip             — Semester · Career · Masters
 *   5. Opportunity Feed         — intentionally omitted, see note below
 *   6. Quick Launch             — bottom, lowest emphasis
 *
 * Sections 2, 3, and 5 are conditionally rendered by design (§2: "take
 * zero vertical space when there's nothing real to show"). All three
 * are always-empty today because their backing tables
 * (`bottlenecks`, `drift_signals`, `opportunities`,
 * `project_status_snapshots`, `research_activities`) don't exist in
 * this schema, and this change is explicitly scoped not to modify
 * storage beyond `schedule_disruptions` (08_ADAPTIVE_PLANNER.md §5).
 * Per §2's own rule — "an empty bottleneck section is not shown as
 * 'no bottlenecks! 🎉' — it simply isn't there" — the correct,
 * spec-faithful render of "always empty" is exactly what a genuinely
 * inactive one would already look like: absent. Nothing here fakes
 * data to fill those three sections.
 */
export default function Now() {
  const { state, loading, refresh } = useBootstrap();
  const { navigate } = useNavigation();

  if (loading && !state) {
    return (
      <div className={styles.screen}>
        <LoadingState shape="verdict" />
      </div>
    );
  }

  if (!state) {
    return null;
  }

  const allocation = state.profile
    ? {
        windowLabel: formatWindow(state.profile.deep_work_window_start, state.profile.deep_work_window_end),
        // No deep-work block allocation engine exists yet (out of scope
        // for this change) — an honest empty state, not fabricated blocks.
        blocks: [],
      }
    : { windowLabel: "Tonight's deep-work window", blocks: [] };

  return (
    <div className={styles.screen}>
      <div className={styles.header}>
        <p className={`${styles.eyebrow} type-caption`}>Now</p>
        <DensityToggle />
      </div>

      {state.profile ? <MissionStrip profile={state.profile} /> : null}

      <VerdictCard verdict={state.verdict} />

      <AiInsightCard />

      <AdaptivePlannerCard
        semesterActive={state.current_semester !== null}
        availableMinutesTonight={state.available_minutes_tonight}
        baseWindowMinutes={state.base_window_minutes}
        todayDisruptions={state.today_disruptions}
        recentDisruptions={state.recent_disruptions}
        onLogged={refresh}
      />

      <RoutineQuestionnaireCard semesterActive={state.current_semester !== null} courses={state.courses} />

      <div className={styles.secondaryRow}>
        <DeepWorkAllocationCard allocation={allocation} />
      </div>

      {state.profile ? (
        <section className={styles.section}>
          <h2 className={`${styles.sectionTitle} type-body-medium`}>Semester · Career · Masters</h2>
          <HealthStrip
            profile={state.profile}
            careerDeadlines={state.career_deadlines}
            onOpenTrajectory={() => navigate('trajectory')}
          />
        </section>
      ) : null}

      <QuickLaunch
        onOpenSemesterSetup={() => navigate('semester-setup')}
        onOpenDecisionLog={() => navigate('decision-log')}
      />
    </div>
  );
}