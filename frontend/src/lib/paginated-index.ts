// The index pages' URL state, the legacy paginator query strings: a
// search term plus one page number per paginated section, visited by
// updating the current URL so the server load reruns.
import { goto } from '$app/navigation';

export interface VisitOptions {
  /** A search change replaces history and keeps the input focused. */
  search?: boolean;
}

/** The page number a query string names; anything unparsable is page 1. */
export function pageParam(params: URLSearchParams, name: string): number {
  const value = Number.parseInt(params.get(name) ?? '', 10);
  return Number.isFinite(value) && value >= 1 ? value : 1;
}

/** Builds the API query for an index page: the legacy paginators omit
 * their param on page 1 and the search when empty. */
export function indexQuery(entries: Record<string, string | number | null>): string {
  const params = new URLSearchParams();
  for (const [name, value] of Object.entries(entries)) {
    if (value === null || value === '' || value === 1) continue;
    params.set(name, String(value));
  }
  const query = params.toString();
  return query ? `?${query}` : '';
}

/** Navigates the current page to the given query entries (null clears
 * one), so the server load reruns with the new page or search. */
export function visitIndex(
  current: URL,
  entries: Record<string, string | number | null>,
  options: VisitOptions = {},
): Promise<void> {
  const url = new URL(current);
  for (const [name, value] of Object.entries(entries)) {
    if (value === null || value === '' || value === 1) {
      url.searchParams.delete(name);
    } else {
      url.searchParams.set(name, String(value));
    }
  }
  return goto(url, {
    replaceState: options.search ?? false,
    keepFocus: options.search ?? false,
    noScroll: options.search ?? false,
  });
}
