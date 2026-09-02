// Sort-control decisions of the module browser, ported from the Rust
// filter controls (themselves the legacy `SortFunctions` behavior).

/**
 * Whether a sort field is active, and if so its direction, for a given
 * sort state. The boolean in the pair is `descending`.
 */
export function sortDirection(sort: [string, boolean] | null, field: string): boolean | null {
  if (sort === null || sort[0] !== field) {
    return null;
  }
  return sort[1];
}

/**
 * The legacy sort cycle for one field: off, then ascending, then
 * descending, then off again.
 */
export function cycleSort(current: boolean | null): boolean | null {
  if (current === null) {
    return false;
  }
  return current === false ? true : null;
}
