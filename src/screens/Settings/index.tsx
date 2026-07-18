import { useEffect, useState } from 'react';
import { DensityToggle } from '../../components/shared/DensityToggle';
import {
  deleteAnthropicApiKey,
  deleteHfApiKey,
  hasAnthropicApiKey,
  hasHfApiKey,
  isUsingKeychainFallback,
  saveAnthropicApiKey,
  saveHfApiKey,
} from '../../ipc/bindings';
import { ApiKeyPanel } from './ApiKeyPanel';
import { ConnectorsSection } from './ConnectorsSection';
import { RoutineTriggerSection } from './RoutineTriggerSection';
import styles from './Settings.module.css';

/**
 * Settings — a fifth flat nav-rail destination hosting the AI provider
 * key-management surface (06_AI_ENGINE.md §9's `save/has/delete
 * *ApiKey` commands in `crates/athena-app/src/commands/ai.rs`, typed in
 * `ipc/bindings.ts` but previously called nowhere in the frontend).
 *
 * This is a route, not a `ModalLayer` addition: `ModalLayer.tsx`/
 * `modalContext.tsx` document exactly two named interruptive-surface
 * exceptions (`'challenge' | 'deep-work-guard'`, both typed into
 * `ActiveModal` itself) and call that set "a hard ceiling, not a
 * starting point" — key management is a persistent, revisitable
 * settings surface, not an interruption, so it doesn't fit that
 * layer's contract. No prior settings-style screen exists anywhere in
 * this codebase (`MissionStrip.tsx`'s doc comment notes there isn't
 * even a Profile-editing entry point yet), so this is a new,
 * intentionally minimal pattern: a flat screen, same shell as every
 * other route, no sub-navigation.
 *
 * Two providers are exposed, matching bindings.ts exactly: Anthropic
 * (cloud, paid) and Hugging Face (free-tier Inference API). Ollama, the
 * third provider `RecommendationDto.source` can report, requires no key
 * (it's a local install) and so has no panel here — nothing in
 * `ipc/bindings.ts` exposes an Ollama configuration command to call.
 *
 * This screen also hosts every account/OAuth data-source connector
 * (Codeforces, LeetCode, GitHub, Gmail, Google Classroom, Notion — see
 * `ConnectorsSection.tsx`), relocated here from Semester Setup's
 * wizard. An account connection is a standing relationship that should
 * persist across every future semester, not something re-prompted
 * inside a once-a-term wizard, so it lives in Settings instead. Only
 * Semester Setup's file-based import mechanisms (calendar/.ics, PDF,
 * CSV — see `SemesterSetup/ImportStep.tsx`) remain in the wizard.
 */
export default function Settings() {
  const [fallbackActive, setFallbackActive] = useState(false);

  useEffect(() => {
    isUsingKeychainFallback().then(setFallbackActive).catch(() => undefined);
  }, []);

  return (
    <div className={styles.screen}>
      <div className={styles.header}>
        <p className={`${styles.eyebrow} type-caption`}>Settings</p>
        <DensityToggle />
      </div>

      {fallbackActive && (
        <p className={`${styles.sectionDescription} type-caption`}>
          Your device's secure keychain isn't available right now, so any keys or tokens you save below are being
          stored in an encrypted file in Athena's own app-data folder instead. They still never touch the database,
          and everything works the same either way.
        </p>
      )}

      <section className={styles.section}>
        <h2 className={`${styles.sectionTitle} type-body-medium`}>AI providers</h2>
        <p className={`${styles.sectionDescription} type-caption`}>
          Connecting a provider makes Daily Briefing, Weekly Plan, and Weakness Analysis more fluent — Athena's
          verdicts are fully computed either way (06_AI_ENGINE.md §10's offline-first guarantee); an unconnected
          provider only changes the wording, never the underlying recommendation.
        </p>

        <ApiKeyPanel
          label="Anthropic"
          description="Used for Claude-powered phrasing of the Daily Briefing, Weekly Plan, and Weakness Analysis."
          placeholder="sk-ant-…"
          hasKey={hasAnthropicApiKey}
          saveKey={saveAnthropicApiKey}
          deleteKey={deleteAnthropicApiKey}
        />

        <ApiKeyPanel
          label="Hugging Face"
          description="Free-tier alternative — no billing required. Slots in automatically after Anthropic and before Ollama."
          placeholder="hf_…"
          helpUrl="https://huggingface.co/settings/tokens"
          helpLabel="Get a token (role: Inference)"
          hasKey={hasHfApiKey}
          saveKey={saveHfApiKey}
          deleteKey={deleteHfApiKey}
        />
      </section>

      <ConnectorsSection styles={styles} />

      <RoutineTriggerSection styles={styles} />
    </div>
  );
}
