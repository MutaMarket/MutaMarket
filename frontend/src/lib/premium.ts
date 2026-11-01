// Helpers of the premium sales page (the legacy
// Premium/ShowPremiumPage.vue computed props) and the shared premium
// config the legacy AppData middleware exposed globally, served here
// through /api/sidebar.

import type { ModuleDetail, NavState } from './types';

/** The legacy AppData shared props: config app.premium_character /
 * app.premium_cost / app.premium_yearly_cost (env-overridable on the
 * backend). */
export interface PremiumConfig {
  premium_character: string;
  premium_cost: number;
  premium_yearly_cost: number;
  /** The ad-free plan's price points (three times both, by default). */
  premium_ad_free_cost: number;
  premium_ad_free_yearly_cost: number;
}

/** The backend config defaults, used only when /api/sidebar is
 * unreachable (the pages degrade like the donations lists). */
export const DEFAULT_PREMIUM: PremiumConfig = {
  premium_character: 'MutaMate',
  premium_cost: 100_000_000,
  premium_yearly_cost: 1_000_000_000,
  premium_ad_free_cost: 300_000_000,
  premium_ad_free_yearly_cost: 3_000_000_000,
};

/** The shared premium config out of a sidebar payload. */
export function premiumFromSidebar(
  payload: Partial<PremiumConfig> | null | undefined,
): PremiumConfig {
  return {
    premium_character: payload?.premium_character ?? DEFAULT_PREMIUM.premium_character,
    premium_cost: payload?.premium_cost ?? DEFAULT_PREMIUM.premium_cost,
    premium_yearly_cost: payload?.premium_yearly_cost ?? DEFAULT_PREMIUM.premium_yearly_cost,
    premium_ad_free_cost: payload?.premium_ad_free_cost ?? DEFAULT_PREMIUM.premium_ad_free_cost,
    premium_ad_free_yearly_cost:
      payload?.premium_ad_free_yearly_cost ?? DEFAULT_PREMIUM.premium_ad_free_yearly_cost,
  };
}

/** Columns of the falling hero backdrop. */
export const HERO_COLUMNS = 3;

/** The legacy hero_columns computed: the sample modules dealt
 * round-robin into three columns, empty columns dropped. */
export function heroColumns(modules: ModuleDetail[]): ModuleDetail[][] {
  const columns: ModuleDetail[][] = Array.from({ length: HERO_COLUMNS }, () => []);
  modules.forEach((module, index) => {
    columns[index % HERO_COLUMNS].push(module);
  });
  return columns.filter((column) => column.length > 0);
}

export type PremiumPlan = 'monthly' | 'yearly';

/** The monthly and yearly price of plain or ad-free premium. */
export function tierCosts(
  premium: PremiumConfig,
  adFree: boolean,
): { monthly: number; yearly: number } {
  return adFree
    ? {
        monthly: premium.premium_ad_free_cost,
        yearly: premium.premium_ad_free_yearly_cost,
      }
    : { monthly: premium.premium_cost, yearly: premium.premium_yearly_cost };
}

/** What the transfer ticket asks for under the chosen plan and tier. */
export function planAmount(premium: PremiumConfig, plan: PremiumPlan, adFree = false): number {
  const costs = tierCosts(premium, adFree);
  return plan === 'yearly' ? costs.yearly : costs.monthly;
}

/** The legacy yearly_savings computed: two free months. */
export function yearlySavings(premium: PremiumConfig, adFree = false): number {
  const costs = tierCosts(premium, adFree);
  return costs.monthly * 12 - costs.yearly;
}

/** How many times plain premium the ad-free plan costs; also the rate a
 * plan switch converts the remaining time at. */
export function adFreeFactor(premium: PremiumConfig): number {
  return premium.premium_ad_free_cost / premium.premium_cost;
}

/** How long an account stays ad-free before it may switch back, the
 * API's AD_FREE_MIN_DAYS (it keeps ad-free from being toggled on just
 * for a visit). */
export const AD_FREE_MIN_DAYS = 30;

/** A stretch of premium time as the plan-switch warning prints it:
 * whole days from one day on, whole hours below (never "0 days"). */
export function premiumSpan(ms: number): { unit: 'days' | 'hours'; count: number } {
  const hours = Math.floor(ms / 3_600_000);
  return hours >= 24
    ? { unit: 'days', count: Math.floor(hours / 24) }
    : { unit: 'hours', count: hours };
}

/** The character the account's premium status shows: the one whose
 * premium runs longest, else the active one, else the first. */
export function premiumHolder<T extends { active: boolean; premium_paid_until: string | null }>(
  characters: T[],
): T | null {
  let holder: T | null = null;
  for (const character of characters) {
    const until = character.premium_paid_until;
    if (until !== null && (holder?.premium_paid_until ?? '') < until) {
      holder = character;
    }
  }
  return holder ?? characters.find((character) => character.active) ?? characters[0] ?? null;
}

/** Whole days of premium left until `paidUntil`, none when it lapsed. */
export function daysLeft(paidUntil: string | null, now: number): number {
  if (paidUntil === null) {
    return 0;
  }
  return Math.max(0, Math.floor((Date.parse(paidUntil) - now) / 86_400_000));
}

/** The days a plan switch leaves: a third going ad-free, three times
 * coming back (whole days, as the page shows them). */
export function switchedDays(days: number, toAdFree: boolean, factor: number): number {
  return Math.floor(toAdFree ? days / factor : days * factor);
}

/** Whole days a gift form may ask for: at least one, at most the donor's
 * balance, never a fraction. */
export function clampGiftDays(value: number, remaining: number): number {
  if (!Number.isFinite(value) || remaining < 1) {
    return 1;
  }
  return Math.min(Math.max(Math.trunc(value), 1), remaining);
}

/** Who the premium card demo shows: the visitor's active character
 * (`own`), or failing that the creator of the first sample module. */
export function demoCharacter(
  nav: NavState | null,
  modules: ModuleDetail[],
): { id: number; name: string; own: boolean } | null {
  const active = nav?.characters.find((character) => character.id === nav.user.active_character_id);
  if (active) {
    return { id: active.id, name: active.name, own: true };
  }
  const creator = modules.find((module) => module.creator !== null)?.creator;
  return creator ? { id: creator.id, name: creator.name, own: false } : null;
}
