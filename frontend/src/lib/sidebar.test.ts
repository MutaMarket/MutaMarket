import { describe, expect, it } from 'vitest';

import { visibleDiscordInvites, type DiscordInvite } from './sidebar';

describe('partner Discord invites', () => {
  it('hides unconfigured invites and keeps the payload order', () => {
    const invites: DiscordInvite[] = [
      {
        name: 'Abyssal Trading',
        url: 'https://discord.gg/abc',
        image: '/img/at.webp',
        member_count: 12543,
      },
      { name: 'MutaMarket', url: null, image: null, member_count: null },
      {
        name: 'EC Trade',
        url: 'https://discord.gg/def',
        image: '/img/ectrade.png',
        member_count: null,
      },
    ];
    expect(visibleDiscordInvites(invites).map((invite) => invite.name)).toEqual([
      'Abyssal Trading',
      'EC Trade',
    ]);
  });

  it('an all-unconfigured payload hides the section', () => {
    expect(
      visibleDiscordInvites([{ name: 'MutaMarket', url: null, image: null, member_count: null }]),
    ).toEqual([]);
  });
});
