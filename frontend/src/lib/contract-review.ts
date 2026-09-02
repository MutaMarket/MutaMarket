// The moderator contract review page logic, ported from the legacy
// Moderator/ContractReviewPage.vue: the response shape, the three review
// actions and the magic-key shortcuts.

import { t } from './i18n.svelte';
import type { CharacterRef, ModuleDetail } from './types';

/** The reviewable historic contract of /api/moderator/contracts. */
export interface ReviewContract {
  id: number;
  type: string;
  price: number | null;
  asking_for_items: boolean;
  plex_count: number;
  non_abyssal_modules_count: number;
  abyssal_modules_count: number;
  issuer: CharacterRef;
  status: string;
  modules: ModuleDetail[];
  date_issued: string | null;
  date_expired: string | null;
  /** Present for admins only. */
  ignore_for_training?: boolean;
}

export interface ReviewPageData {
  contract: ReviewContract | null;
  search: {
    type: { id: number; name: string } | null;
    needs_training: number | null;
  };
}

export type ReviewStatus = 'completed' | 'failed' | 'unknown';

/** The footer's review buttons, in legacy order with their variants. */
export const REVIEW_ACTIONS: {
  status: ReviewStatus;
  labelKey: string;
  variant: 'default' | 'destructive' | 'outline';
}[] = [
  { status: 'completed', labelKey: 'admin.contractReview.markAsCompleted', variant: 'default' },
  { status: 'failed', labelKey: 'admin.contractReview.markAsFailed', variant: 'destructive' },
  { status: 'unknown', labelKey: 'admin.contractReview.markAsUnknown', variant: 'outline' },
];

export function statusLabel(status: string): string {
  switch (status) {
    case 'completed':
    case 'failed':
    case 'unknown':
      return t(`admin.contractReview.statuses.${status}`);
    default:
      return status;
  }
}

/**
 * The legacy useMagicKeys watchers: Shift+C completes, Shift+X fails
 * (both bypassing the confirm dialog, a legacy quirk), Shift+L copies
 * the contract link.
 */
export function shortcutAction(event: {
  shiftKey: boolean;
  key: string;
}): 'completed' | 'failed' | 'copy-link' | null {
  if (!event.shiftKey) {
    return null;
  }
  switch (event.key) {
    case 'C':
      return 'completed';
    case 'X':
      return 'failed';
    case 'L':
      return 'copy-link';
    default:
      return null;
  }
}
