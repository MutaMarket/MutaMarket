// The settings page data (GET /api/settings) and its raffle-code
// masking helper.

export interface LinkedAccount {
  name: string;
  avatar: string | null;
  is_public: boolean;
}

export interface RaffleWin {
  id: number;
  code: string;
  name: string;
  description: string | null;
  type: { id: number; name: string } | null;
}

/** An account the user blocked, under its owner's notify character. */
export interface BlockedUser {
  user_id: number;
  name: string;
  character_id: number | null;
  blocked_at: string;
}

export interface SettingsData {
  characters: { id: number; name: string }[];
  character_to_notify: { id: number; name: string } | null;
  discord: LinkedAccount | null;
  twitch: LinkedAccount | null;
  patreon: LinkedAccount | null;
  raffle_wins: RaffleWin[];
  blocked_users: BlockedUser[];
}

/** The legacy dot mask over an unrevealed prize code. */
export function maskCode(code: string): string {
  return '•'.repeat(code.length);
}
