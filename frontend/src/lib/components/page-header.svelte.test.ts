import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';

import PageHeader from './page-header.svelte';

// `context` is also a render option, so every render passes props
// under the explicit key.
const context = createRawSnippet(() => ({
  render: () => '<a href="/locations/jita-4-4-60003760">Jita IV - Moon 4</a>',
}));

/** The muted line under the title, whitespace collapsed. */
function subtitleLine(container: HTMLElement): HTMLElement | null {
  return container.querySelector('h1 + div');
}

describe('page-header.svelte', () => {
  it('renders the context beside the subtitle behind a hairline', async () => {
    const { container } = await render(PageHeader, {
      props: { title: 'Ammo Box', subtitle: 'Station Container', context },
    });

    const line = subtitleLine(container);
    expect(line?.textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'Station Container Jita IV - Moon 4',
    );
    const holder = line?.querySelector('a')?.parentElement;
    expect(holder?.classList.contains('border-l')).toBe(true);
    expect(holder?.previousElementSibling?.tagName).toBe('P');
  });

  it('shows the context alone when there is no subtitle', async () => {
    const { container } = await render(PageHeader, { props: { title: 'Ammo Box', context } });

    const line = subtitleLine(container);
    expect(line?.querySelector('p')).toBeNull();
    expect(line?.querySelector('a')?.parentElement?.previousElementSibling).toBeNull();
  });

  it('omits the line entirely without subtitle or context', async () => {
    const { container } = await render(PageHeader, { title: 'Ammo Box' });

    expect(subtitleLine(container)).toBeNull();
  });
});
