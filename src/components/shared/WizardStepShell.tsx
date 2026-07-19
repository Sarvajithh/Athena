import { Fragment } from 'react';
import { Check } from 'lucide-react';
import { Icon } from './Icon';
import type { WizardStep } from '../../mock/types';
import styles from './WizardStepShell.module.css';

interface WizardStepShellProps {
  steps: WizardStep[];
}

/**
 * Generic multi-step wizard stepper (label + checkmark + connector per
 * step). Originally built for the Semester Setup wizard; that flow was
 * removed in the Semester screen reform, but Onboarding's ProfileWizard
 * depends on this same shell, so it lives here in components/shared
 * rather than under any one screen's folder.
 */
export function WizardStepShell({ steps }: WizardStepShellProps) {
  return (
    <nav className={styles.steps} aria-label="Wizard steps">
      {steps.map((step, index) => (
        <Fragment key={step.id}>
          <span className={`${styles.step} type-caption`} data-status={step.status}>
            {step.status === 'complete' ? <Icon icon={Check} size="inline" /> : null}
            {step.label}
          </span>
          {index < steps.length - 1 ? <span className={styles.connector} aria-hidden="true" /> : null}
        </Fragment>
      ))}
    </nav>
  );
}
