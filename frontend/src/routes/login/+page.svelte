<!-- The login page, the legacy Pages/Auth/LoginPage.vue: the welcome
     column with its feature cards on the left, the sign-in card on the
     right. Redesigned on top of that layout: the card explains what the
     scopes are for, and it stays in view while the features scroll. -->
<script lang="ts">
  import {
    Calculator,
    ChartColumn,
    Handshake,
    Library,
    MapPin,
    MessagesSquare,
    PackageSearch,
  } from '@lucide/svelte';
  import Logo from '$lib/components/logo.svelte';
  import PageMeta from '$lib/components/page-meta.svelte';
  import { Button } from '$lib/components/ui/button';
  import { t } from '$lib/i18n.svelte';

  // The legacy feature cards, keyed under auth.login.features.*.
  const features = [
    { icon: PackageSearch, key: 'assetImport' },
    { icon: Calculator, key: 'appraisal' },
    { icon: Library, key: 'collections' },
    { icon: MapPin, key: 'locationTracking' },
    { icon: MessagesSquare, key: 'offers' },
    { icon: Handshake, key: 'contracts' },
    { icon: ChartColumn, key: 'statistics' },
  ];
</script>

<PageMeta title={t('meta.login.title')} description={t('meta.login.description')} />

<div class="grid gap-10 2xl:grid-cols-[1fr_minmax(0,26rem)]">
  <div>
    <p class="text-xs font-medium tracking-[0.2em] text-primary uppercase">
      {t('auth.login.eyebrow')}
    </p>
    <h1 class="mt-2 text-4xl font-bold">{t('auth.login.welcomeTitle')}</h1>
    <p class="mt-4 max-w-2xl text-lg text-muted-foreground">
      {t('auth.login.welcomeDescription')}
    </p>

    <div class="mt-8 grid gap-4 md:grid-cols-2">
      {#each features as feature (feature.key)}
        <div class="flex items-start gap-4 border border-border bg-card-1 p-5">
          <div class="shrink-0 bg-primary/10 p-2">
            <feature.icon class="size-6 text-primary" />
          </div>
          <div>
            <h2 class="font-semibold">{t(`auth.login.features.${feature.key}.title`)}</h2>
            <p class="mt-1 text-sm text-muted-foreground">
              {t(`auth.login.features.${feature.key}.description`)}
            </p>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <aside class="self-start 2xl:sticky 2xl:top-6">
    <div class="border border-border bg-card-1 p-8 text-center">
      <Logo class="mx-auto size-12 text-primary" />
      <h2 class="mt-4 text-2xl font-bold">{t('auth.login.card.title')}</h2>
      <p class="mt-2 text-muted-foreground">{t('auth.login.card.description')}</p>

      <Button href="/eve" rel="external" size="lg" class="mt-6 w-full text-base">
        {t('auth.login.card.signInWithEve')}
      </Button>
      <a
        href="/eve?without_scopes=true"
        rel="external"
        class="mt-3 block text-sm text-muted-foreground transition hover:text-foreground hover:underline"
      >
        {t('auth.login.card.loginWithoutScopes')}
      </a>

      <p class="mt-6 text-left text-xs text-muted-foreground">
        {t('auth.login.card.scopesExplanation')}
      </p>
    </div>
  </aside>
</div>
