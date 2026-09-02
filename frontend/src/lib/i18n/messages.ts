// The translation catalogue: the legacy locale files, one JSON file per
// namespace and locale, flattened to "namespace.path.key". English is the
// source of truth; de and zh mirror its key set (pinned by a test).
export const LOCALES = [
  { value: 'en', label: 'English' },
  { value: 'de', label: 'Deutsch' },
  { value: 'zh', label: '中文' },
] as const;

export type Locale = (typeof LOCALES)[number]['value'];

export const DEFAULT_LOCALE: Locale = 'en';

export function isLocale(value: unknown): value is Locale {
  return LOCALES.some((locale) => locale.value === value);
}

type Tree = { [key: string]: string | Tree };

function flatten(tree: Tree, prefix: string, into: Record<string, string>): void {
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'string') {
      into[path] = value;
    } else {
      flatten(value, path, into);
    }
  }
}

const files = import.meta.glob('./locales/*/*.json', { eager: true, import: 'default' }) as Record<
  string,
  Tree
>;

function catalogue(): Record<Locale, Record<string, string>> {
  const result = { en: {}, de: {}, zh: {} } as Record<Locale, Record<string, string>>;
  for (const [path, tree] of Object.entries(files)) {
    const match = /\.\/locales\/(\w+)\/(\w+)\.json$/.exec(path);
    if (!match || !isLocale(match[1])) {
      continue;
    }
    flatten(tree, match[2], result[match[1]]);
  }
  return result;
}

export const MESSAGES = catalogue();
