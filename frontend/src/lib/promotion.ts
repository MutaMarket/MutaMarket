// Promoted modules: a donation with `promote <item id>` as its in-game
// reason buys the module days at the top of the browser's default order
// (see src/promotions.rs). One dialog mounted in the layout, opened from
// any module menu, shows what to send.
import { writable } from 'svelte/store';
import type { ModuleDetail } from './types';

/** The backend's DAILY_COST, used only when /api/sidebar is unreachable
 * (like the premium defaults). */
export const DEFAULT_DAILY_COST = 25_000_000;

/** The longest stretch the dialog's day field offers; the backend takes
 * any amount, this only keeps the field sane. */
export const MAX_DIALOG_DAYS = 365;

/** The module the dialog is open for; null keeps it closed. */
export const promotionModule = writable<ModuleDetail | null>(null);

export function openPromote(module: ModuleDetail) {
  promotionModule.set(module);
}

export function closePromote() {
  promotionModule.set(null);
}

/** Only modules the browser lists can be promoted: a live contract or a
 * MutaMarket sell listing (the backend's rule). */
export function canPromote(module: ModuleDetail): boolean {
  return module.contract !== null || module.public_asset !== null;
}

/** The donation reason that promotes the module. */
export function promotionReason(module: ModuleDetail): string {
  return `promote ${module.id}`;
}

/** What a day costs, out of a sidebar payload. */
export function dailyCostFromSidebar(
  payload: { promotion_daily_cost?: number } | null | undefined,
): number {
  return payload?.promotion_daily_cost ?? DEFAULT_DAILY_COST;
}

/** The day field's text as whole days within the dialog's range. */
export function clampDays(input: string): number {
  const days = Math.floor(Number(input));
  if (!Number.isFinite(days) || days < 1) return 1;
  return Math.min(days, MAX_DIALOG_DAYS);
}

/** When the module's running promotion ends, or null without one. */
export function promotedUntil(module: ModuleDetail, now: number = Date.now()): Date | null {
  if (!module.promoted_until) return null;
  const until = new Date(module.promoted_until);
  return until.getTime() > now ? until : null;
}
