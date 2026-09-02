import { describe, expect, it } from 'vitest';

import { toContractLink, toItemLink, toPyfa } from './export';
import type { ModuleDetail } from './types';

const module = {
  id: 1046163215321,
  type: { id: 47408, name: '50MN Abyssal Microwarpdrive' },
  source_type: { id: 12076, name: '50MN Quad LiF Restrained Microwarpdrive' },
  mutaplasmid: { id: 47757, name: 'Decayed 50MN Microwarpdrive Mutaplasmid' },
  contract: { id: 987654, price: 150000000 },
  mutated_attributes: [
    { name: 'speedFactor', value: 512.5 },
    { name: 'capacitorNeed', value: 231.25 },
  ],
} as unknown as ModuleDetail;

describe('export formats', () => {
  it('builds the exact legacy Pyfa block', () => {
    expect(toPyfa(module)).toBe(
      '50MN Quad LiF Restrained Microwarpdrive\n' +
        'Decayed 50MN Microwarpdrive Mutaplasmid\n' +
        'speedFactor 512.5, capacitorNeed 231.25',
    );
  });

  it('builds the exact in-game link formats', () => {
    expect(toItemLink(module)).toBe(
      '<url=showinfo:47408//1046163215321>50MN Abyssal Microwarpdrive (1046163215321)</url>',
    );
    // Intl's currency form separates "ISK" with a non-breaking space,
    // exactly like the legacy links did.
    expect(toContractLink(module)).toBe(
      '<url=contract:30000142//987654>Contract 987654 (50MN Abyssal Microwarpdrive) ISK 150,000,000</url>',
    );
  });
});
