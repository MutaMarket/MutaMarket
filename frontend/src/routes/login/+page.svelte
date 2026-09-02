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

  const features = [
    {
      icon: PackageSearch,
      title: 'Easy asset import',
      description:
        'Import your modules straight from your EVE Online assets. No need to create contracts by hand.',
    },
    {
      icon: Calculator,
      title: 'Module appraisal',
      description: 'Get value estimates for your rolls from models trained on real contract sales.',
    },
    {
      icon: Library,
      title: 'Collections',
      description:
        'Create and share collections of modules, whether to organise your hangar or to show off your best finds.',
    },
    {
      icon: MapPin,
      title: 'Location tracking',
      description:
        'Know where every module sits across New Eden, down to the container and the slot inside it.',
    },
    {
      icon: MessagesSquare,
      title: 'Offers',
      description:
        'Negotiate prices through direct messages and get notified the moment an offer comes in.',
    },
    {
      icon: Handshake,
      title: 'Contract management',
      description:
        'Track your contracts in one place, with their history and how your sales are going.',
    },
    {
      icon: ChartColumn,
      title: 'Statistics',
      description: 'Detailed statistics about your modules, sales and trading performance.',
    },
  ];
</script>

<PageMeta
  title="Login"
  description="Login to MutaMarket and start trading abyssal modules today!"
/>

<div class="grid gap-10 2xl:grid-cols-[1fr_minmax(0,26rem)]">
  <div>
    <p class="text-xs font-medium tracking-[0.2em] text-primary uppercase">
      Abyssal module marketplace
    </p>
    <h1 class="mt-2 text-4xl font-bold">Welcome to MutaMarket</h1>
    <p class="mt-4 max-w-2xl text-lg text-muted-foreground">
      Your dedicated platform for trading abyssal modules in EVE Online. Import your assets, list
      them for sale and manage your module inventory in one place.
    </p>

    <div class="mt-8 grid gap-4 md:grid-cols-2">
      {#each features as feature (feature.title)}
        <div class="flex items-start gap-4 border border-border bg-card-1 p-5">
          <div class="shrink-0 bg-primary/10 p-2">
            <feature.icon class="size-6 text-primary" />
          </div>
          <div>
            <h3 class="font-semibold">{feature.title}</h3>
            <p class="mt-1 text-sm text-muted-foreground">{feature.description}</p>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <aside class="self-start 2xl:sticky 2xl:top-6">
    <div class="border border-border bg-card-1 p-8 text-center">
      <Logo class="mx-auto size-12 text-primary" />
      <h2 class="mt-4 text-2xl font-bold">Welcome, capsuleer</h2>
      <p class="mt-2 text-muted-foreground">Sign in with your EVE Online account to get started.</p>

      <Button href="/eve" rel="external" size="lg" class="mt-6 w-full text-base">
        Sign in with EVE Online
      </Button>
      <a
        href="/eve?without_scopes=true"
        rel="external"
        class="mt-3 block text-sm text-muted-foreground transition hover:text-foreground hover:underline"
      >
        Sign in without granting scopes
      </a>

      <p class="mt-6 text-left text-xs text-muted-foreground">
        The requested scopes let MutaMarket read your assets to import modules, follow your
        contracts, and answer in-game mails about your listings. Each can be added or removed per
        character in the settings later, and nothing is written to your EVE account.
      </p>
    </div>
  </aside>
</div>
