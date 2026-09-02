// The display settings a module page renders with: one reactive object
// per browser session, shared by every page and the options bar that
// mutates it, so a toggle carries across client-side navigation. The
// root layout's cookie read (page.data.displaySettings) seeds it once;
// on the server each request gets its own copy.
import { browser } from '$app/environment';
import { page } from '$app/state';
import { type DisplaySettings, defaultDisplaySettings } from './display';

let shared: DisplaySettings = $state(defaultDisplaySettings());
let seeded = false;

export function useDisplaySettings(): DisplaySettings {
  const fromData =
    (page.data.displaySettings as DisplaySettings | undefined) ?? defaultDisplaySettings();
  if (!browser) {
    const perRequest: DisplaySettings = $state({ ...fromData });
    return perRequest;
  }
  if (!seeded) {
    Object.assign(shared, fromData);
    seeded = true;
  }
  return shared;
}
