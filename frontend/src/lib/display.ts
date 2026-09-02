// The legacy display-setting cookies, mirroring the server's
// `settings_from_headers` defaults and allowed values
// (src/server/display.rs). The cookies are not HttpOnly, so the client
// persists changes through PUT /display and reads them here during SSR.

import { browser } from '$app/environment';

export const DISPLAY_VALUES = ['grid', 'list', 'table'] as const;
export const ATTRIBUTE_BAR_MODES = ['default', 'type', 'absolute', 'none'] as const;

export type DisplayValue = (typeof DISPLAY_VALUES)[number];
export type AttributeBarMode = (typeof ATTRIBUTE_BAR_MODES)[number];

export interface DisplaySettings {
  display: DisplayValue;
  attribute_bar_mode: AttributeBarMode;
  show_attribute_scores: boolean;
}

export function defaultDisplaySettings(): DisplaySettings {
  return { display: 'grid', attribute_bar_mode: 'default', show_attribute_scores: false };
}

/** The settings from cookie values, falling back per field like the server. */
export function settingsFromCookies(cookie: (name: string) => string | undefined): DisplaySettings {
  const defaults = defaultDisplaySettings();

  const display = cookie('display');
  const barMode = cookie('attribute_bar_mode');
  const scores = cookie('show_attribute_scores');

  return {
    display: DISPLAY_VALUES.includes(display as DisplayValue)
      ? (display as DisplayValue)
      : defaults.display,
    attribute_bar_mode: ATTRIBUTE_BAR_MODES.includes(barMode as AttributeBarMode)
      ? (barMode as AttributeBarMode)
      : defaults.attribute_bar_mode,
    show_attribute_scores:
      scores === undefined ? defaults.show_attribute_scores : scores === '1' || scores === 'true',
  };
}

// The last settings saved in this browser session. The root layout's
// server load supplies the cookie values, and that load does not rerun
// on client-side navigation, so without this a toggle showed on the
// next module page only after a hard refresh. Browser-only: on the
// server the module is shared between requests.
let remembered: DisplaySettings | null = null;

/** The settings a page should start from: the last saved ones this session, else the server's cookie read. */
export function currentDisplaySettings(fromData: DisplaySettings): DisplaySettings {
  return browser && remembered ? { ...remembered } : { ...fromData };
}

export function rememberDisplaySettings(settings: DisplaySettings): void {
  if (browser) {
    remembered = { ...settings };
  }
}

/** Persists the settings through the guest-accessible endpoint (204). */
export async function saveDisplaySettings(settings: DisplaySettings): Promise<void> {
  rememberDisplaySettings(settings);
  await fetch('/display', {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(settings),
  });
}
