<!-- The legacy Navigation/LocaleSwitcher.vue: a language button opening
     the three locales with their native names, which are never
     translated. -->
<script lang="ts">
  import { Check, Languages } from '@lucide/svelte';
  import { invalidateAll } from '$app/navigation';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import { LOCALES, type Locale, getLocale, setLocale, t } from '$lib/i18n.svelte';

  // The catalogue swaps in place; content the server renders per locale
  // (the documentation, API messages) reloads through the page loads,
  // which now send the new cookie.
  function pick(locale: Locale) {
    setLocale(locale);
    void invalidateAll();
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    class="flex size-10 cursor-pointer items-center justify-center bg-white/[0.04] text-white shadow-none transition hover:bg-white/[0.07] focus:outline-none"
  >
    <Languages class="size-5" />
    <span class="sr-only">{t('nav.localeSwitcher.label')}</span>
  </DropdownMenu.Trigger>
  <DropdownMenu.Content sideOffset={8} align="end">
    {#each LOCALES as option (option.value)}
      <DropdownMenu.Item
        class="flex cursor-pointer items-center justify-between gap-4"
        onSelect={() => pick(option.value)}
      >
        {option.label}
        {#if option.value === getLocale()}
          <Check class="size-4" />
        {/if}
      </DropdownMenu.Item>
    {/each}
  </DropdownMenu.Content>
</DropdownMenu.Root>
