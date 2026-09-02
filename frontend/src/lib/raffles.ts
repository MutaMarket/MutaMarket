// The admin raffle page data (GET /api/admin/raffles) and the status
// labels the legacy RafflePage.vue renders.
import { t } from './i18n.svelte';
import { STATUS_ACTIVE, STATUS_CLAIMED, STATUS_PENDING } from '$lib/raffle-status';

export interface AdminRaffleItem {
  id: number;
  name: string | null;
  description: string | null;
  code: string;
  status: number;
  type: { id: number; name: string | null } | null;
  winner: { id: number; name: string | null; character_id: number | null } | null;
  expires_at: string | null;
  created_at: string | null;
}

export interface AdminRafflesData {
  raffle_items: AdminRaffleItem[];
  types: { id: number; name: string }[];
  type_search: string;
}

/** The legacy status labels of the admin list. */
export function statusLabel(status: number): string {
  switch (status) {
    case STATUS_PENDING:
      return t('admin.raffles.statusPending');
    case STATUS_ACTIVE:
      return t('admin.raffles.statusActive');
    case STATUS_CLAIMED:
      return t('admin.raffles.statusClaimed');
    default:
      return t('common.labels.unknown');
  }
}

/** The legacy getStatusColor: claimed green, active amber, pending muted. */
export function statusColor(status: number): string {
  switch (status) {
    case STATUS_CLAIMED:
      return 'text-green-500';
    case STATUS_ACTIVE:
      return 'text-yellow-500';
    default:
      return 'text-muted-foreground';
  }
}

/** The legacy maskCode: one bullet per character until revealed. */
export function maskCode(code: string): string {
  return '•'.repeat(code.length);
}

/** Drawn prizes and claimed ones are the ones with a winner to show. */
export function hasWinner(item: AdminRaffleItem): boolean {
  return item.winner !== null && item.status !== STATUS_PENDING;
}

/** The pool counts the legacy page shows above the list. */
export function poolCounts(items: AdminRaffleItem[]): {
  pending: number;
  active: number;
  claimed: number;
} {
  return {
    pending: items.filter((item) => item.status === STATUS_PENDING).length,
    active: items.filter((item) => item.status === STATUS_ACTIVE).length,
    claimed: items.filter((item) => item.status === STATUS_CLAIMED).length,
  };
}
