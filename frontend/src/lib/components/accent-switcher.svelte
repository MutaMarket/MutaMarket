<!-- The theme-color button beside the language switcher (no legacy
     counterpart): a swatch in the current accent that opens a card of
     colors. Hovering a swatch previews it on the whole page, clicking
     saves it to the account. The default lime and the legacy orange are
     free, every other color is a premium pick and shows a lock until
     the account has premium. -->
<script lang="ts">
  import { Check, Lock } from '@lucide/svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import {
    DEFAULT_ACCENT_SWATCH,
    FREE_ACCENTS,
    PREMIUM_ACCENTS,
    accentThemeCss,
    normalizeAccent,
  } from '$lib/accent';
  import { t } from '$lib/i18n.svelte';
  import { notifyError } from '$lib/toast';

  let { accent, hasPremium }: { accent: string | null; hasPremium: boolean } = $props();

  const current = $derived(normalizeAccent(accent));

  let open = $state(false);
  let hovered = $state<string | null | undefined>(undefined);
  // The hover preview is a second accent style after the layout's, so
  // it wins while the pointer rests on a swatch; leaving restores the
  // saved color. A null hover previews the default lime by cancelling
  // the saved override, which the layout's style would otherwise keep.
  const previewStyle = $derived(
    hovered === undefined
      ? null
      : (accentThemeCss(hovered) ?? accentThemeCss(DEFAULT_ACCENT_SWATCH)),
  );

  interface Swatch {
    /** The saved value: null is the default theme. */
    value: string | null;
    color: string;
    label: string;
    locked: boolean;
  }

  const swatches = $derived<Swatch[]>([
    {
      value: null,
      color: DEFAULT_ACCENT_SWATCH,
      label: t('nav.accentSwitcher.default'),
      locked: false,
    },
    ...FREE_ACCENTS.map((color) => ({ value: color, color, label: color, locked: false })),
    ...PREMIUM_ACCENTS.map((color) => ({ value: color, color, label: color, locked: !hasPremium })),
  ]);

  async function pick(swatch: Swatch) {
    hovered = undefined;
    open = false;
    if (swatch.locked) {
      await goto('/premium');
      return;
    }
    if (swatch.value === current) {
      return;
    }
    const response = await fetch('/settings/accent', {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ accent_color: swatch.value }),
    });
    if (response.ok) {
      await invalidateAll();
    } else {
      const body = await response.json().catch(() => ({ message: undefined }));
      notifyError(
        t('settings.theme.notUpdated'),
        body.message ?? t('errors.internalServerError.name'),
      );
    }
  }
</script>

<svelte:head>
  {#if previewStyle}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- previewStyle is a strict hex-only string -->
    {@html `<style>${previewStyle}</style>`}
  {/if}
</svelte:head>

<DropdownMenu.Root bind:open onOpenChange={(value) => !value && (hovered = undefined)}>
  <DropdownMenu.Trigger
    class="flex size-10 cursor-pointer items-center justify-center bg-white/[0.04] shadow-none transition hover:bg-white/[0.07] focus:outline-none"
  >
    <span class="size-4 rounded-full bg-primary ring-2 ring-white/20"></span>
    <span class="sr-only">{t('nav.accentSwitcher.label')}</span>
  </DropdownMenu.Trigger>
  <DropdownMenu.Content sideOffset={8} align="end" class="w-56 p-3">
    <p class="hud-label mb-2">{t('nav.accentSwitcher.title')}</p>
    <div
      class="grid grid-cols-4 gap-2"
      role="group"
      aria-label={t('nav.accentSwitcher.label')}
      onmouseleave={() => (hovered = undefined)}
    >
      {#each swatches as swatch (swatch.color)}
        <button
          type="button"
          aria-label={swatch.label}
          aria-pressed={swatch.value === current}
          data-locked={swatch.locked || undefined}
          class="relative flex size-9 cursor-pointer items-center justify-center rounded-full ring-2 ring-offset-2 ring-offset-popover transition-transform hover:scale-110 {swatch.value ===
          current
            ? 'ring-foreground'
            : 'ring-transparent hover:ring-border'}"
          style="background-color: {swatch.color}"
          onmouseenter={() => (hovered = swatch.value)}
          onfocus={() => (hovered = swatch.value)}
          onblur={() => (hovered = undefined)}
          onclick={() => pick(swatch)}
        >
          {#if swatch.locked}
            <span class="flex size-5 items-center justify-center rounded-full bg-black/60">
              <Lock class="size-3 text-white" />
            </span>
          {:else if swatch.value === current}
            <Check class="size-4 text-black/70" />
          {/if}
        </button>
      {/each}
    </div>
    {#if !hasPremium}
      <a
        href="/premium"
        class="mt-3 block text-xs text-muted-foreground hover:text-foreground hover:underline"
      >
        {t('nav.accentSwitcher.premiumHint')}
      </a>
    {/if}
  </DropdownMenu.Content>
</DropdownMenu.Root>
