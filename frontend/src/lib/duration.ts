// Small time helpers of the admin scheduler page.
import { t } from './i18n.svelte';

/** A job cadence as prose: "every 5 min", "hourly", "daily". */
export function humanizeInterval(seconds: number): string {
  if (seconds === 7 * 24 * 3600) return t('misc.duration.weekly');
  if (seconds % (24 * 3600) === 0 && seconds > 24 * 3600) {
    return t('misc.duration.every', { time: t('misc.duration.days', { count: seconds / 86_400 }) });
  }
  if (seconds === 24 * 3600) return t('misc.duration.daily');
  if (seconds === 3600) return t('misc.duration.hourly');
  if (seconds % 3600 === 0) {
    return t('misc.duration.every', { time: t('misc.duration.hours', { count: seconds / 3600 }) });
  }
  if (seconds >= 60) {
    return t('misc.duration.every', {
      time: t('misc.duration.minutes', { count: Math.round(seconds / 60) }),
    });
  }
  return t('misc.duration.every', { time: t('misc.duration.seconds', { count: seconds }) });
}

/** A span of seconds in its coarsest unit: "30 s", "3 min", "2 h", "5 d". */
export function durationLabel(seconds: number): string {
  const magnitude = Math.abs(seconds);
  if (magnitude < 60) return t('misc.duration.seconds', { count: Math.round(magnitude) });
  if (magnitude < 3600) return t('misc.duration.minutes', { count: Math.round(magnitude / 60) });
  if (magnitude < 24 * 3600) {
    return t('misc.duration.hours', { count: Math.round(magnitude / 3600) });
  }
  return t('misc.duration.days', { count: Math.round(magnitude / (24 * 3600)) });
}

/** Seconds relative to now as prose: "3 min ago", "in 12 min", "just now". */
export function relativeTime(deltaSeconds: number): string {
  if (Math.abs(deltaSeconds) < 5) return t('misc.duration.justNow');
  const time = durationLabel(deltaSeconds);
  return deltaSeconds < 0 ? t('misc.duration.ago', { time }) : t('misc.duration.in', { time });
}

/** Parses the API's `timestamptz::text` format into unix seconds. */
export function parseDbTimestamp(text: string): number {
  // "2026-08-25 14:30:00.123+00" - make it ISO for Date.parse: T
  // separator, and the bare Postgres offset needs its minutes.
  const iso = text.replace(' ', 'T').replace(/([+-]\d{2})$/, '$1:00');
  const parsed = Date.parse(iso);
  return Number.isNaN(parsed) ? 0 : parsed / 1000;
}
