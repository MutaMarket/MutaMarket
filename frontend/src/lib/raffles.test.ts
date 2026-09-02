import { describe, expect, it } from 'vitest';
import {
  hasWinner,
  maskCode,
  poolCounts,
  statusColor,
  statusLabel,
  type AdminRaffleItem,
} from './raffles';
import { STATUS_ACTIVE, STATUS_CLAIMED, STATUS_PENDING } from './raffle-status';

function item(status: number, winner: AdminRaffleItem['winner'] = null): AdminRaffleItem {
  return {
    id: status,
    name: 'Prize',
    description: null,
    code: 'CODE',
    status,
    type: null,
    winner,
    expires_at: null,
    created_at: null,
  };
}

describe('statusLabel', () => {
  it('names every RaffleStatus case', () => {
    expect(statusLabel(STATUS_PENDING)).toBe('Pending');
    expect(statusLabel(STATUS_ACTIVE)).toBe('Active');
    expect(statusLabel(STATUS_CLAIMED)).toBe('Claimed');
    expect(statusLabel(99)).toBe('Unknown');
  });
});

describe('statusColor', () => {
  it('colours claimed green, active amber, the rest muted', () => {
    expect(statusColor(STATUS_CLAIMED)).toBe('text-green-500');
    expect(statusColor(STATUS_ACTIVE)).toBe('text-yellow-500');
    expect(statusColor(STATUS_PENDING)).toBe('text-muted-foreground');
  });
});

describe('maskCode', () => {
  it('hides every character behind a bullet', () => {
    expect(maskCode('ABC-123')).toBe('•••••••');
  });
});

describe('hasWinner', () => {
  const winner = { id: 1, name: 'Winner', character_id: null };

  it('shows the winner of drawn and claimed prizes', () => {
    expect(hasWinner(item(STATUS_ACTIVE, winner))).toBe(true);
    expect(hasWinner(item(STATUS_CLAIMED, winner))).toBe(true);
  });

  it('hides a stale winner on a prize back in the pool', () => {
    expect(hasWinner(item(STATUS_PENDING, winner))).toBe(false);
    expect(hasWinner(item(STATUS_ACTIVE))).toBe(false);
  });
});

describe('poolCounts', () => {
  it('counts the pool by status', () => {
    const counts = poolCounts([
      item(STATUS_PENDING),
      item(STATUS_PENDING),
      item(STATUS_ACTIVE),
      item(STATUS_CLAIMED),
      item(STATUS_CLAIMED),
    ]);
    expect(counts).toEqual({ pending: 2, active: 1, claimed: 2 });
  });
});
