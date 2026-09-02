<script lang="ts">
  import { page } from '$app/state';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { t } from '$lib/i18n.svelte';

  // SvelteKit routes every failure through this one component, where
  // legacy had a page per status (Errors/NotFound.vue and friends);
  // their meta and heading keys are looked up by status here.
  const ERRORS: Record<number, { meta: string; name: string }> = {
    400: { meta: 'meta.errorBadRequest', name: 'errors.badRequest.name' },
    403: { meta: 'meta.errorUnauthorized', name: 'errors.unauthorized.name' },
    404: { meta: 'meta.errorNotFound', name: 'errors.notFound.name' },
    500: { meta: 'meta.errorInternalServerError', name: 'errors.internalServerError.name' },
    503: { meta: 'meta.errorServiceUnavailable', name: 'errors.serviceUnavailable.heading' },
  };

  const known = $derived(ERRORS[page.status]);
  const fallback = $derived(page.error?.message ?? t('errors.internalServerError.name'));
  const meta = $derived(
    known
      ? { title: t(`${known.meta}.title`), description: t(`${known.meta}.description`) }
      : { title: String(page.status), description: fallback },
  );
</script>

<PageMeta title={meta.title} description={meta.description} />

{#if known}
  <h1>{t(known.name)}</h1>
{:else}
  <h1>{fallback}</h1>
{/if}
