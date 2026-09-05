// Premium accent theming: a chosen color retints everything derived from
// the lime accent by overriding the theme's custom properties. The color
// is validated to a strict hex before it ever reaches an injected
// <style>, and a readable foreground is computed from its luminance.

/** The brand lime as one swatch color: the default theme's accent is a
 * light/dark pair of lime tokens, so the default is represented by a
 * null accent and this hex only paints its swatch. */
export const DEFAULT_ACCENT_SWATCH = '#a6e600';

/** The legacy site's primary, `hsl(38 92% 50%)` (Tailwind amber-500). */
export const LEGACY_ORANGE = '#f59e0b';

/** The colors every account may pick, premium or not: the legacy orange
 * beside the default lime. Mirrors `FREE_ACCENTS` on the API. */
export const FREE_ACCENTS = [LEGACY_ORANGE];

/** The premium-only picks, tasteful and distinct from the free ones. */
export const PREMIUM_ACCENTS = ['#22c55e', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899', '#ef4444'];

/** Quick picks for the settings page and the premium page's theme-color
 * demo; the first is the brand lime. */
export const ACCENT_PRESETS = [DEFAULT_ACCENT_SWATCH, ...PREMIUM_ACCENTS, ...FREE_ACCENTS];

export function isFreeAccent(color: string | null | undefined): boolean {
  const hex = normalizeAccent(color);
  return hex !== null && FREE_ACCENTS.includes(hex);
}

/** A strict `#rrggbb`, lowercased; anything else yields null so a bad
 * value can never break out of the injected style. */
export function normalizeAccent(color: string | null | undefined): string | null {
  if (!color) {
    return null;
  }
  const match = /^#([0-9a-fA-F]{6})$/.exec(color.trim());
  return match ? `#${match[1].toLowerCase()}` : null;
}

/** Readable text on the accent: near-black or near-white chosen by the
 * accent's WCAG relative luminance. */
export function accentForeground(hex: string): string {
  const channel = (start: number) => {
    const value = parseInt(hex.slice(start, start + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
  return luminance > 0.36 ? '#0a0a0a' : '#fafafa';
}

/** The `:root` custom-property overrides that retint everything derived
 * from the accent. `--glow` and `--sidebar-primary` reference `--primary`
 * and follow it automatically. Null when the color is absent or invalid,
 * so the default lime stands. */
export function accentThemeCss(color: string | null | undefined): string | null {
  const hex = normalizeAccent(color);
  if (!hex) {
    return null;
  }
  const foreground = accentForeground(hex);
  return (
    `:root{--primary:${hex}!important;--accent:${hex}!important;` +
    `--primary-foreground:${foreground}!important;--accent-foreground:${foreground}!important;` +
    `--ring:color-mix(in oklab,${hex} 60%,transparent)!important;}`
  );
}
