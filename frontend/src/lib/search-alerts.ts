// Saved search alerts (GET /api/search-alerts): the shape the API serves
// and the client-side mirror of the server's query normalization, so the
// bell can tell whether the current browser query is already saved.

import { META_GROUPS } from './filter-meta';
import { toIsk } from './format-number';
import { buildQueryPath, parseQueryUi, type UiSearch } from './query';

export interface SearchAlert {
  id: number;
  /** The normalized query path without prefix (`type/47408/goldbar`). */
  query: string;
  type: { id: number; name: string } | null;
  created_at: string;
  last_notified_at: string | null;
  notified_count: number;
}

/**
 * The saved form of a browser query, mirroring `search_alerts::normalize_query`:
 * no page, no sort, none of the account-relative options, the rest in the
 * builder's canonical order.
 */
export function alertQuery(search: UiSearch): string {
  return buildQueryPath('', {
    ...search,
    page: 1,
    sort: null,
    withPersonalModules: false,
    created: false,
    withoutFitted: false,
    withoutAssets: false,
    withoutContracts: false,
  }).replace(/^\/+/, '');
}

/** The sort a saved alert opens its search with, like the notification
 * link: newest listing first, so the fresh matches sit on top. */
const ALERT_SORT: [string, boolean] = ['date-added', true];

/** The browser page a saved alert links back to, newest listing first. */
export function alertSearchPath(alert: SearchAlert): string {
  return buildQueryPath('modules', { ...parseQueryUi(alert.query), sort: ALERT_SORT });
}

/** The query's options after the type, for the settings list. */
export function alertOptions(alert: SearchAlert): string[] {
  const parts = alert.query.split('/').filter((part) => part !== '');
  if (parts[0] === 'type') {
    parts.splice(0, 2);
  }
  return parts;
}

/** One line of an alert's plain-language breakdown: a translation key
 * under `modules.alerts.criteria` plus its parameters. */
export interface AlertCriterion {
  key: string;
  params?: Record<string, string | number>;
}

/**
 * Spells out what an alert watches for, one criterion per saved option,
 * in the query's canonical order. Bound semantics follow the server: a
 * lone attribute or estimated-value number is a minimum, a lone contract
 * price a maximum.
 */
export function alertCriteria(alert: SearchAlert): AlertCriterion[] {
  const search = parseQueryUi(alert.query);
  const criteria: AlertCriterion[] = [];
  const group = META_GROUPS.find((option) => option.slug === search.metaGroup);
  if (group) {
    criteria.push({ key: 'metaGroup', params: { name: group.name } });
  }
  if (search.metaLevel !== null) {
    criteria.push({ key: 'metaLevel', params: { level: search.metaLevel } });
  }
  for (const attribute of search.attributes) {
    criteria.push(
      attribute.upper === null
        ? { key: 'attributeMin', params: { name: attribute.name, min: attribute.lower } }
        : {
            key: 'attributeRange',
            params: { name: attribute.name, min: attribute.lower, max: attribute.upper },
          },
    );
  }
  if (search.contractType === 'item_exchange') {
    criteria.push({ key: 'itemExchange' });
  } else if (search.contractType === 'auction') {
    criteria.push({ key: 'auction' });
  }
  if (search.price !== null) {
    const [lower, upper] = search.price;
    criteria.push(
      upper === null
        ? { key: 'priceMax', params: { max: toIsk(lower) } }
        : { key: 'priceRange', params: { min: toIsk(lower), max: toIsk(upper) } },
    );
  }
  if (search.value !== null) {
    const [lower, upper] = search.value;
    criteria.push(
      upper === null
        ? { key: 'valueMin', params: { min: toIsk(lower) } }
        : { key: 'valueRange', params: { min: toIsk(lower), max: toIsk(upper) } },
    );
  }
  if (search.noMultiItemContracts) {
    criteria.push({ key: 'singleItem' });
  }
  if (search.onlyContracts) {
    criteria.push({ key: 'onlyContracts' });
  }
  if (search.goldbar) {
    criteria.push({ key: 'goldbar' });
  }
  if (search.brownbar) {
    criteria.push({ key: 'brownbar' });
  }
  if (search.diamondbar) {
    criteria.push({ key: 'diamondbar' });
  }
  if (search.withoutOtherItems) {
    criteria.push({ key: 'withoutOtherItems' });
  }
  if (search.inJita) {
    criteria.push({ key: 'inJita' });
  }
  if (search.search !== null) {
    criteria.push({ key: 'searchTerm', params: { term: search.search } });
  }
  return criteria;
}
