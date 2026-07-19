import { lazy } from 'react';
import type { ComponentType } from 'react';
import type { LucideIcon } from 'lucide-react';
import { Calendar, Compass, ListChecks, Settings as SettingsIcon, TrendingUp } from 'lucide-react';

const Now = lazy(() => import('./screens/Now'));
const Trajectory = lazy(() => import('./screens/Trajectory'));
const Semester = lazy(() => import('./screens/Semester'));
const DecisionLog = lazy(() => import('./screens/DecisionLog'));
const Settings = lazy(() => import('./screens/Settings'));

export type ScreenId = 'now' | 'trajectory' | 'semester' | 'decision-log' | 'settings';

export interface RouteConfig {
  id: ScreenId;
  label: string;
  icon: LucideIcon;
  shortcut: string;
  component: ComponentType;
}

/**
 * Flat, five items, no nesting (originally SPRINT2_SPEC.md §5's four;
 * Settings added as a fifth flat destination for the AI-provider key
 * management surface — see `screens/Settings`'s own doc comment for why
 * this is a route rather than a `ModalLayer` addition). Order here is
 * the single source of truth for nav-rail order and ⌘1–⌘5 keyboard
 * mapping. Now is first and is the default/landing screen (spec §5.2).
 */
export const routes: RouteConfig[] = [
  { id: 'now', label: 'Now', icon: Compass, shortcut: '1', component: Now },
  { id: 'trajectory', label: 'Trajectory', icon: TrendingUp, shortcut: '2', component: Trajectory },
  { id: 'semester', label: 'Semester', icon: Calendar, shortcut: '3', component: Semester },
  { id: 'decision-log', label: 'Decision Log', icon: ListChecks, shortcut: '4', component: DecisionLog },
  { id: 'settings', label: 'Settings', icon: SettingsIcon, shortcut: '5', component: Settings },
];

export const defaultScreenId: ScreenId = 'now';
