// Variance-bound search paths, ported from the legacy
// Helper/ModuleFinder.ts: each enabled attribute gets a window of
// `(best − worst) · variance / 100` around the roll's value, rendered
// into the attribute-bounds URL grammar by the shared query builder.
import { buildQueryPath, defaultUiSearch, type UiAttributeFilter } from './query';
import type { AbyssalTypeStatistic, ModuleDetail } from './types';

function statisticFor(
  statistics: AbyssalTypeStatistic[],
  attributeId: number,
): AbyssalTypeStatistic | null {
  return statistics.find((statistic) => statistic.attribute_id === attributeId) ?? null;
}

function boundedAttributes(
  module: ModuleDetail,
  statistics: AbyssalTypeStatistic[],
  enabledIds: number[],
  variance: number,
  cheapest: boolean,
): UiAttributeFilter[] {
  return module.mutated_attributes
    .filter((attribute) => enabledIds.includes(attribute.id))
    .flatMap((attribute): UiAttributeFilter[] => {
      const statistic = statisticFor(statistics, attribute.id);
      if (!statistic) {
        return [];
      }
      const range = Math.abs(statistic.best - statistic.worst);
      const window = (range * variance) / 100;
      const current = attribute.value;

      if (cheapest) {
        // A single bound: "at least this good", flipped for
        // low-is-good attributes (the legacy getCheapestModules).
        const lower = statistic.high_is_good ? current - window : current + window;
        return [{ name: attribute.name, lower, upper: null }];
      }
      return [{ name: attribute.name, lower: current - window, upper: current + window }];
    });
}

/** `/{prefix}/type/{id}/attributes/...` for rolls like this one. */
export function similarSearchPath(
  module: ModuleDetail,
  statistics: AbyssalTypeStatistic[],
  enabledIds: number[],
  variance: number,
  prefix = 'modules',
): string {
  const search = defaultUiSearch();
  search.typeSlug = String(module.type.id);
  search.attributes = boundedAttributes(module, statistics, enabledIds, variance, false);
  return buildQueryPath(prefix, search);
}

/** The similar window plus for-sale-only and price-ascending sort. */
export function cheapestSearchPath(
  module: ModuleDetail,
  statistics: AbyssalTypeStatistic[],
  enabledIds: number[],
  variance: number,
  prefix = 'modules',
): string {
  const search = defaultUiSearch();
  search.typeSlug = String(module.type.id);
  search.attributes = boundedAttributes(module, statistics, enabledIds, variance, true);
  search.onlyContracts = true;
  search.sort = ['price', false];
  return buildQueryPath(prefix, search);
}

/** The cheapest window over the sold archive. */
export function historicSearchPath(
  module: ModuleDetail,
  statistics: AbyssalTypeStatistic[],
  enabledIds: number[],
  variance: number,
): string {
  return cheapestSearchPath(module, statistics, enabledIds, variance, 'historic-sales');
}
