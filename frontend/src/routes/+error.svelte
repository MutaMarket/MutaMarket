<script lang="ts">
  // SvelteKit routes every failure through this one component, where
  // legacy had a page per status (Errors/NotFound.vue and friends):
  // the ErrorHeader status + name, an ErrorDescription, the ErrorLinks
  // row, and for 503 the standalone maintenance screen that polls until
  // the site is back. The API's message arrives as page.error.message
  // (apiGet turns non-OK responses into error(status, message)).
  import {
    ArrowLeft,
    FileWarning,
    RotateCw,
    SearchX,
    ServerCrash,
    ShieldBan,
    Wrench,
  } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import BrandIcon from '$lib/components/brand-icon.svelte';
  import Logo from '$lib/components/logo.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import Trans from '$lib/components/trans.svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';

  /** The legacy maintenance page's router.reload() cadence. */
  const MAINTENANCE_POLL_MS = 5000;

  interface KnownError {
    meta: string;
    name: string;
    body: string;
    icon: Component<{ class?: string }>;
    /** Offer a retry: the failure is on our side and may pass. */
    retry?: boolean;
  }

  const ERRORS: Record<number, KnownError> = {
    400: {
      meta: 'meta.errorBadRequest',
      name: 'errors.badRequest.name',
      body: 'errors.badRequest.body',
      icon: FileWarning,
    },
    403: {
      meta: 'meta.errorUnauthorized',
      name: 'errors.unauthorized.name',
      body: 'errors.unauthorized.body',
      icon: ShieldBan,
    },
    404: {
      meta: 'meta.errorNotFound',
      name: 'errors.notFound.name',
      body: 'errors.notFound.body',
      icon: SearchX,
    },
    500: {
      meta: 'meta.errorInternalServerError',
      name: 'errors.internalServerError.name',
      body: 'errors.internalServerError.body',
      icon: ServerCrash,
      retry: true,
    },
  };

  const status = $derived(page.status);
  const maintenance = $derived(status === 503);
  const known = $derived(ERRORS[status]);
  const message = $derived(page.error?.message ?? t('errors.internalServerError.name'));
  const meta = $derived(
    maintenance
      ? {
          title: t('meta.errorServiceUnavailable.title'),
          description: t('meta.errorServiceUnavailable.description'),
        }
      : known
        ? { title: t(`${known.meta}.title`), description: t(`${known.meta}.description`) }
        : { title: String(status), description: message },
  );
  const Icon = $derived(known?.icon ?? ServerCrash);

  // The legacy ErrorLinks: home (labelled Modules), characters,
  // collections, and settings for a signed-in user.
  const links = $derived([
    { href: '/', label: t('errors.links.modules') },
    { href: '/characters', label: t('nav.links.characters') },
    { href: '/collections', label: t('nav.links.collections') },
    ...(page.data.nav?.user ? [{ href: '/settings', label: t('nav.menu.settings') }] : []),
  ]);

  const discordInvite = $derived(
    page.data.sidebar?.discord_invites.find((invite) => invite.url !== null) ?? null,
  );

  let retrying = $state(false);
  async function retry() {
    retrying = true;
    try {
      await invalidateAll();
    } finally {
      retrying = false;
    }
  }

  // Maintenance polls the page until a load succeeds, which swaps this
  // component out for the page that was requested.
  $effect(() => {
    if (!maintenance) return;
    const timer = setInterval(() => void invalidateAll(), MAINTENANCE_POLL_MS);
    return () => clearInterval(timer);
  });
</script>

<PageMeta title={meta.title} description={meta.description} />

<div class="flex min-h-[60vh] items-center justify-center py-8" data-error-page={status}>
  {#if maintenance}
    <section class="hud-frame relative w-full max-w-2xl overflow-hidden p-8 text-center sm:p-12">
      <Wrench class="absolute -top-6 -right-6 size-48 text-white/[0.04]" />
      <div class="relative flex items-center justify-center gap-3">
        <Logo class="h-9 w-16 text-primary" />
        <span class="text-3xl font-semibold tracking-tight">MutaMarket</span>
      </div>
      <p class="hud-label relative mt-6">{t('errors.label')} // 503</p>
      <h1 class="relative mt-2 animate-pulse text-2xl font-semibold tracking-tight">
        {t('errors.serviceUnavailable.heading')}
      </h1>
      <p class="relative mx-auto mt-4 max-w-prose text-balance text-muted-foreground">
        {t('errors.serviceUnavailable.body')}
      </p>
      <p class="relative mt-6 flex items-center justify-center gap-2 text-xs text-muted-foreground">
        <RotateCw class="size-3.5 animate-spin [animation-duration:3s]" />
        {t('errors.serviceUnavailable.retrying')}
      </p>
      {#if discordInvite?.url}
        <Button
          href={discordInvite.url}
          target="_blank"
          rel="noopener"
          variant="outline"
          size="lg"
          class="relative mt-6"
        >
          <BrandIcon brand="discord" class="size-4" />
          {t('errors.serviceUnavailable.joinDiscord')}
        </Button>
      {/if}
    </section>
  {:else}
    <section class="hud-frame relative w-full max-w-2xl overflow-hidden p-8 sm:p-10">
      <Icon class="absolute -top-4 -right-4 size-44 text-white/[0.04]" />
      <p class="hud-label relative">{t('errors.label')} // {status}</p>
      <div class="relative mt-3 flex flex-wrap items-baseline gap-x-4 gap-y-1">
        <span class="font-mono text-6xl leading-none font-semibold text-primary tabular-nums">
          {status}
        </span>
        <h1 class="text-2xl font-semibold tracking-tight">
          {known ? t(known.name) : message}
        </h1>
      </div>
      <p class="relative mt-5 max-w-prose text-muted-foreground">
        {#if status === 404}
          <Trans key="errors.notFound.body">
            {#snippet query()}<strong class="break-all text-foreground">{page.url.pathname}</strong
              >{/snippet}
          </Trans>
        {:else if status === 400}
          <Trans key="errors.badRequest.body">
            {#snippet message()}<span class="text-foreground">{page.error?.message}</span>{/snippet}
          </Trans>
        {:else if known}
          {t(known.body)}
        {:else}
          {t('errors.generic.body')}
        {/if}
      </p>
      <div class="relative mt-6 flex flex-wrap items-center gap-2">
        {#if known?.retry || !known}
          <Button size="lg" onclick={retry} disabled={retrying}>
            <RotateCw class="size-4 {retrying ? 'animate-spin' : ''}" />
            {t('errors.actions.retry')}
          </Button>
        {/if}
        <Button variant="outline" size="lg" onclick={() => history.back()}>
          <ArrowLeft class="size-4" />
          {t('errors.actions.back')}
        </Button>
      </div>
      <nav
        class="relative mt-6 flex flex-wrap gap-x-5 gap-y-2 border-t border-border pt-4 text-sm"
        aria-label={t('errors.label')}
      >
        {#each links as link (link.href)}
          <a href={link.href} class="text-primary hover:underline">{link.label}</a>
        {/each}
      </nav>
    </section>
  {/if}
</div>
