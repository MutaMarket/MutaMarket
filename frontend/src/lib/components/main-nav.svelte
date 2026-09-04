<script lang="ts">
  // The main navigation, the legacy Navigation/DesktopNavbar.vue +
  // useNavigationData.ts: the accent-colored logo, the primary link row
  // with icons, the hover-opened "More" menu with its grouped entries,
  // and the square character button on the right. Only ported pages
  // appear: Sell, Offers, My contracts/locations/statistics, Settings,
  // Historic sales, the API docs link and the legacy admin tools arrive
  // with their features; the Admin group instead carries our scheduler
  // page. Below the xl breakpoint the
  // bar collapses the way MobileNavbar.vue did: logo, character button
  // and a hamburger that slides the whole link list in from the left.
  import { ChevronDown, Menu } from '@lucide/svelte';
  import { page } from '$app/state';
  import CharacterMenu from './character-menu.svelte';
  import LocaleSwitcher from './locale-switcher.svelte';
  import Logo from './logo.svelte';
  import * as Sheet from './ui/sheet/index.js';
  import NavIcon, { type NavigationIcon } from './nav-icon.svelte';
  import { t } from '$lib/i18n.svelte';
  import { moduleSlug } from '$lib/query';
  import type { NavState } from '$lib/types';

  let { nav }: { nav: NavState | null } = $props();

  interface NavLink {
    title: string;
    href: string;
    icon: NavigationIcon;
    active: boolean;
  }

  interface MenuGroup {
    label?: string;
    items: NavLink[];
  }

  // Widened: the generated pathname union omits bare rest-param routes
  // like /all-modules.
  const path = $derived(page.url.pathname as string);
  const active = $derived(nav?.characters.find((character) => character.active) ?? null);

  const links = $derived.by(() => {
    const list: NavLink[] = [
      // The legacy Buy link lights only on the home route.
      { title: t('nav.links.buy'), href: '/', icon: 'shop', active: path === '/' },
      {
        title: t('nav.links.appraise'),
        href: '/modules/add',
        icon: 'plus',
        active: path === '/modules/add',
      },
      {
        title: t('nav.links.characters'),
        href: '/characters',
        icon: 'users',
        active: path === '/characters',
      },
      {
        title: t('nav.links.collections'),
        href: '/collections',
        icon: 'collection',
        active: path === '/collections',
      },
    ];
    if (nav) {
      list.splice(2, 0, {
        title: t('nav.links.sell'),
        href: '/sell/modules',
        icon: 'contract',
        active: path.startsWith('/sell/modules'),
      });
      list.push({
        title: t('nav.links.offers'),
        href: '/offers',
        icon: 'offer',
        active: path.startsWith('/offers'),
      });
      list.push({
        title: t('nav.links.myModules'),
        href: '/personal/modules',
        icon: 'cubes',
        active: path.startsWith('/personal/modules'),
      });
      list.push({
        title: t('nav.menu.myLocations'),
        href: '/locations',
        icon: 'location',
        active: path.startsWith('/locations'),
      });
    }
    return list;
  });

  const menuGroups = $derived.by(() => {
    const groups: MenuGroup[] = [];
    if (nav && active) {
      const profile = `/characters/${moduleSlug(active.name, active.id)}`;
      groups.push({
        items: [
          {
            title: t('nav.menu.myProfile'),
            href: profile,
            icon: 'users',
            active: path === profile,
          },
          {
            title: t('nav.menu.myContracts'),
            href: '/personal/contracts',
            icon: 'contract',
            active: path.startsWith('/personal/contracts'),
          },
        ],
      });
    }
    groups.push({
      label: t('nav.groups.resources'),
      items: [
        {
          title: t('nav.menu.allModules'),
          href: '/all-modules',
          icon: 'cubes',
          active: path.startsWith('/all-modules'),
        },
        ...(nav?.user?.has_premium
          ? [
              {
                title: t('nav.menu.historicSales'),
                href: '/historic-sales',
                icon: 'contract' as const,
                active: path.startsWith('/historic-sales'),
              },
            ]
          : []),
        {
          title: t('nav.menu.calculator'),
          href: '/calculator',
          icon: 'calculator',
          active: path.startsWith('/calculator'),
        },
        {
          title: t('nav.menu.statistics'),
          href: '/statistics',
          icon: 'chart',
          active: path.startsWith('/statistics'),
        },
      ],
    });
    groups.push({
      items: [
        {
          title: t('nav.menu.contractReview'),
          href: '/moderator/contracts',
          icon: 'contract',
          active: path.startsWith('/moderator/contracts'),
        },
        {
          title: t('nav.menu.documentation'),
          href: '/documentation',
          icon: 'info',
          active: path.startsWith('/documentation'),
        },
        {
          // Legacy linked out to its separate Scribe site; the
          // reference lives in our own docs, so this is an
          // ordinary in-app link.
          title: t('nav.menu.api'),
          href: '/documentation/api-overview',
          icon: 'api',
          active: path.startsWith('/documentation/api-'),
        },
      ],
    });
    if (nav?.user.is_admin) {
      groups.push({
        label: t('nav.groups.admin'),
        items: [
          {
            title: t('nav.menu.console'),
            href: '/admin',
            icon: 'cog',
            active: path.startsWith('/admin'),
          },
        ],
      });
    }
    return groups;
  });

  let drawerOpen = $state(false);
</script>

<div class="z-40">
  <!-- Matches the page container, which grows by the sidebar width. -->
  <div
    class="mx-auto w-full max-w-7xl xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))] px-4"
  >
    <div class="flex items-center gap-2 border-b border-border py-3">
      <a href="/" class="flex shrink-0 items-center py-1 transition hover:opacity-80">
        <Logo class="size-8 text-primary" />
        <span class="sr-only">{t('nav.logo.home')}</span>
      </a>

      <nav class="ml-1 hidden items-center gap-0.5 xl:flex" aria-label={t('nav.ariaLabel')}>
        {#each links as link (link.title)}
          <a
            href={link.href}
            class="flex items-center gap-1.5 px-3 py-2 text-[0.82rem] font-medium transition {link.active
              ? 'bg-primary/10 text-white'
              : 'text-white/60 hover:bg-white/[0.04] hover:text-white'}"
          >
            <NavIcon icon={link.icon} />
            {link.title}
          </a>
        {/each}

        <div class="group relative">
          <button
            type="button"
            aria-haspopup="true"
            class="flex items-center gap-1 px-3 py-2 text-[0.82rem] font-medium text-white/60 outline-none transition group-hover:bg-white/[0.07] group-hover:text-white group-focus-within:bg-white/[0.07] group-focus-within:text-white"
          >
            {t('nav.desktop.more')}
            <ChevronDown
              class="size-3 text-white/45 transition group-hover:rotate-180 group-focus-within:rotate-180"
            />
          </button>

          <div
            class="invisible absolute top-full left-0 z-50 w-52 border border-border bg-card p-1 opacity-0 transition group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100"
          >
            {#each menuGroups as group, index (index)}
              {#if index > 0}
                <div class="my-1 border-t border-white/8"></div>
              {/if}
              {#if group.label}
                <p class="px-2 py-1.5 text-[0.62rem] tracking-[0.2em] text-white/40 uppercase">
                  {group.label}
                </p>
              {/if}
              {#each group.items as item (item.title)}
                <a
                  href={item.href}
                  class="flex items-center gap-2 px-2 py-1.5 text-[0.82rem] font-medium transition hover:bg-white/[0.06] {item.active
                    ? 'text-primary'
                    : 'text-white/70'}"
                >
                  <NavIcon icon={item.icon} class="text-white/40" />
                  {item.title}
                </a>
              {/each}
            {/each}
          </div>
        </div>
      </nav>

      <div class="flex-1"></div>

      <LocaleSwitcher />

      {#if nav}
        <CharacterMenu characters={nav.characters} scopeCatalogue={nav.scope_catalogue} />
      {:else}
        <a
          href="/login"
          class="flex size-10 items-center justify-center bg-white/[0.04] text-white shadow-none transition hover:bg-white/[0.07]"
        >
          <img
            alt=""
            class="size-6"
            src="https://images.evetech.net/characters/1/portrait?size=64"
          />
          <span class="sr-only">{t('nav.auth.login')}</span>
        </a>
      {/if}

      <Sheet.Root bind:open={drawerOpen}>
        <Sheet.Trigger
          class="flex size-10 items-center justify-center bg-white/[0.04] text-white transition hover:bg-white/[0.07] xl:hidden"
        >
          <Menu class="size-5" />
          <span class="sr-only">{t('nav.mobile.openMenu')}</span>
        </Sheet.Trigger>
        <Sheet.Content side="left" class="w-72 gap-0 overflow-y-auto p-0">
          <Sheet.Header class="border-b border-border px-4 py-3">
            <Sheet.Title class="flex items-center gap-2">
              <Logo class="size-6 text-primary" />
              MutaMarket
            </Sheet.Title>
          </Sheet.Header>
          <nav class="flex flex-col p-2" aria-label={t('nav.ariaLabel')}>
            {#each links as link (link.title)}
              <a
                href={link.href}
                class="flex items-center gap-2 px-2 py-2 text-[0.82rem] font-medium transition hover:bg-white/[0.06] {link.active
                  ? 'text-primary'
                  : 'text-white/70'}"
                onclick={() => (drawerOpen = false)}
              >
                <NavIcon icon={link.icon} class="text-white/40" />
                {link.title}
              </a>
            {/each}
            {#each menuGroups as group, index (index)}
              <div class="my-2 border-t border-white/8"></div>
              {#if group.label}
                <p class="px-2 py-1.5 text-[0.62rem] tracking-[0.2em] text-white/40 uppercase">
                  {group.label}
                </p>
              {/if}
              {#each group.items as item (item.title)}
                <a
                  href={item.href}
                  class="flex items-center gap-2 px-2 py-2 text-[0.82rem] font-medium transition hover:bg-white/[0.06] {item.active
                    ? 'text-primary'
                    : 'text-white/70'}"
                  onclick={() => (drawerOpen = false)}
                >
                  <NavIcon icon={item.icon} class="text-white/40" />
                  {item.title}
                </a>
              {/each}
            {/each}
          </nav>
        </Sheet.Content>
      </Sheet.Root>
    </div>
  </div>
</div>
