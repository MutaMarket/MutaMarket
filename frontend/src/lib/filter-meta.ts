// The legacy meta-group and meta-level statics (Static/MetaGroups.ts,
// Static/MetaLevels.ts), plus the id-to-slug mapping our query grammar
// uses for meta-group segments.
export interface MetaGroupOption {
  id: number;
  slug: string;
  name: string;
  dotClass: string;
}

export const META_GROUPS: MetaGroupOption[] = [
  { id: 1, slug: 't1', name: 'Tech I', dotClass: 'bg-gray-500' },
  { id: 2, slug: 't2', name: 'Tech II', dotClass: 'bg-orange-500' },
  { id: 3, slug: 'storyline', name: 'Storyline', dotClass: 'bg-green-300' },
  { id: 4, slug: 'faction', name: 'Faction', dotClass: 'bg-green-500' },
  { id: 5, slug: 'officer', name: 'Officer', dotClass: 'bg-purple-500' },
  { id: 6, slug: 'deadspace', name: 'Deadspace', dotClass: 'bg-blue-500' },
];

export function metaGroupDotClass(metaGroupId: number | null): string {
  return META_GROUPS.find((group) => group.id === metaGroupId)?.dotClass ?? 'bg-gray-500';
}

export interface MetaLevelOption {
  id: number;
  name: string;
  /** The meta groups whose types can sit at this level. */
  groups: number[];
}

export const META_LEVELS: MetaLevelOption[] = [
  { id: 0, name: 'Level 0', groups: [1] },
  { id: 1, name: 'Level 1', groups: [1] },
  { id: 2, name: 'Level 2', groups: [1] },
  { id: 3, name: 'Level 3', groups: [1] },
  { id: 4, name: 'Level 4', groups: [1] },
  { id: 5, name: 'Level 5', groups: [2] },
  { id: 6, name: 'Level 6', groups: [3, 4] },
  { id: 7, name: 'Level 7', groups: [3, 4, 5, 6] },
  { id: 8, name: 'Level 8', groups: [4, 5, 6] },
  { id: 9, name: 'Level 9', groups: [4, 5, 6] },
  { id: 10, name: 'Level 10', groups: [4, 5, 6] },
  { id: 11, name: 'Level 11', groups: [4, 5, 6] },
  { id: 12, name: 'Level 12', groups: [4, 5, 6] },
  { id: 13, name: 'Level 13', groups: [5, 6] },
  { id: 14, name: 'Level 14', groups: [5, 6] },
  { id: 15, name: 'Level 15', groups: [5, 6] },
  { id: 16, name: 'Level 16', groups: [5, 6] },
  { id: 17, name: 'Level 17', groups: [5] },
];

/** The meta-rank-then-name order every legacy type list uses. */
export function sortByMetaAndName<T extends { meta_group_id: number | null; name: string }>(
  a: T,
  b: T,
): number {
  const rank = (id: number | null) => {
    switch (id) {
      case 1:
        return 1;
      case 2:
        return 2;
      case 3:
        return 3;
      case 4:
        return 4;
      case 6:
        return 5;
      case 5:
        return 6;
      default:
        return id ?? Number.MAX_SAFE_INTEGER;
    }
  };
  return rank(a.meta_group_id) - rank(b.meta_group_id) || a.name.localeCompare(b.name);
}
