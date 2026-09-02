// The source-type comparison cells, ported from the legacy
// Tables/SourceTypes/TypesTable.vue getTypeComparisons: each input type
// of the mutaplasmid gets, per mutated attribute, the input type's own
// value plus the signed difference from that type to this roll, colored
// by whether the roll beats it.
import { formatDifference, formatValue } from './attributes';
import type { ModuleAttributeView, SourceTypeComparison } from './types';

export interface ComparisonCell {
  attribute_id: number;
  /** The input type's own value, formatted with the attribute's unit. */
  value: string;
  /** Signed difference from the input type to this roll. */
  difference: string;
  is_positive: boolean;
}

export function comparisonCells(
  attributes: ModuleAttributeView[],
  comparison: SourceTypeComparison,
): ComparisonCell[] {
  return attributes.map((attribute) => {
    const mutatedValue = attribute.value ?? 0;
    const inputValue = comparison.attributes.find((value) => value.id === attribute.id)?.value ?? 0;
    const baseValue = attribute.base_value ?? 0;

    // Whether "high" is the good direction for this roll: a positive
    // fraction means the roll improved by going up, a negative one by
    // going down (legacy quirk: derived from this module's own roll).
    const highIsGood =
      attribute.fraction >= 0 ? mutatedValue >= baseValue : mutatedValue < baseValue;

    return {
      attribute_id: attribute.id,
      value: formatValue(
        inputValue,
        attribute.unit?.name ?? null,
        attribute.unit?.display_name ?? null,
      ),
      difference: formatDifference(
        mutatedValue,
        inputValue,
        attribute.unit?.name ?? null,
        attribute.unit?.display_name ?? null,
      ),
      is_positive: highIsGood ? inputValue <= mutatedValue : inputValue > mutatedValue,
    };
  });
}

/**
 * The legacy META_GROUP_SORT_ORDER: T1, T2, Storyline, Faction,
 * Deadspace, Officer; unknown groups sort by their own id.
 */
export function metaGroupRank(metaGroupId: number | null): number {
  switch (metaGroupId) {
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
      return metaGroupId ?? Number.MAX_SAFE_INTEGER;
  }
}

/** The default table order: meta-group rank, meta level, name. */
export function compareTypes(a: SourceTypeComparison, b: SourceTypeComparison): number {
  const rank = metaGroupRank(a.type.meta_group_id) - metaGroupRank(b.type.meta_group_id);
  if (rank !== 0) {
    return rank;
  }
  const level = (a.type.meta_level ?? 0) - (b.type.meta_level ?? 0);
  if (level !== 0) {
    return level;
  }
  return a.type.name.localeCompare(b.type.name);
}
