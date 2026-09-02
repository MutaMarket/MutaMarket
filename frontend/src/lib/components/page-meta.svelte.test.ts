import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

// The component reads the request origin off SvelteKit's page state.
vi.mock('$app/state', () => ({
  page: { url: new URL('https://mutamarket.com/characters/nicolas-kion-42') },
}));

const PageMeta = (await import('./page-meta.svelte')).default;

function content(attr: 'name' | 'property', key: string): string | null | undefined {
  return document.head.querySelector(`meta[${attr}="${key}"]`)?.getAttribute('content');
}

describe('page-meta.svelte', () => {
  it('renders the title and the absolute canonical URL', () => {
    render(PageMeta, { title: 'Nicolas Kion', description: 'Browse the modules.' });

    expect(document.title).toBe('Nicolas Kion | MutaMarket');
    expect(content('property', 'og:url')).toBe('https://mutamarket.com/characters/nicolas-kion-42');
    expect(content('property', 'twitter:url')).toBe(
      'https://mutamarket.com/characters/nicolas-kion-42',
    );
  });

  it('absolutizes the default card and omits its dimensions', () => {
    render(PageMeta, { title: 'Nicolas Kion', description: 'Browse the modules.' });

    expect(content('property', 'og:image')).toBe('https://mutamarket.com/img/mutamarket-og.png');
    expect(content('property', 'twitter:image')).toBe(
      'https://mutamarket.com/img/mutamarket-og.png',
    );
    expect(document.head.querySelector('meta[property="og:image:width"]')).toBeNull();
  });

  it('absolutizes a page image and declares its dimensions', () => {
    render(PageMeta, {
      title: 'Nicolas Kion',
      description: 'Browse the modules.',
      image: { url: '/og/character/42', width: 600, height: 315 },
    });

    expect(content('property', 'og:image')).toBe('https://mutamarket.com/og/character/42');
    expect(content('property', 'og:image:width')).toBe('600');
    expect(content('property', 'og:image:height')).toBe('315');
  });

  it('splits the tags between name and property as the legacy component did', () => {
    render(PageMeta, {
      title: 'Nicolas Kion',
      description: 'Browse the modules.',
      keywords: ['Nicolas Kion', 'character'],
    });

    expect(content('name', 'description')).toBe('Browse the modules.');
    expect(content('name', 'keywords')).toBe(
      'mutamarket, mutaplasmid, modules, contracts, deals, offers, eve, online, gaming, appraise, abyssal, Nicolas Kion,character',
    );
    expect(content('name', 'twitter:card')).toBe('summary_large_image');
    expect(content('name', 'og:site_name')).toBe('mutamarket.com');
    expect(content('name', 'theme-color')).toBe('#f59f0a');
    expect(content('name', 'twitter:site')).toBe('mutamarket.com');
    expect(content('name', 'og:locale')).toBe('en_US');
    expect(content('name', 'og:type')).toBe('website');
    expect(content('property', 'og:title')).toBe('Nicolas Kion | MutaMarket');
    expect(content('property', 'og:description')).toBe('Browse the modules.');
    expect(content('property', 'og:image:type')).toBe('image/png');
  });

  it('emits exactly one title element', () => {
    render(PageMeta, { title: 'Nicolas Kion', description: 'Browse the modules.' });

    expect(document.head.querySelectorAll('title')).toHaveLength(1);
  });
});
