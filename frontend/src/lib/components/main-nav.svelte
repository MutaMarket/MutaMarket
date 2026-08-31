<script lang="ts">
	// The main navigation, the legacy Navigation/DesktopNavbar.vue +
	// useNavigationData.ts: the accent-colored logo, the primary link row
	// with icons, the hover-opened "More" menu with its grouped entries,
	// and the square character button on the right. Only ported pages
	// appear: Sell, Offers, My contracts/locations/statistics, Settings,
	// Historic sales, the API docs link and the legacy admin tools arrive
	// with their features; the Admin group instead carries our scheduler
	// page. The locale switcher waits on i18n.
	import { ChevronDown } from '@lucide/svelte';
	import { page } from '$app/state';
	import CharacterMenu from './character-menu.svelte';
	import Logo from './logo.svelte';
	import NavIcon, { type NavigationIcon } from './nav-icon.svelte';
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
			{ title: 'Buy', href: '/', icon: 'shop', active: path === '/' },
			{ title: 'Appraise', href: '/modules/add', icon: 'plus', active: path === '/modules/add' },
			{ title: 'Characters', href: '/characters', icon: 'users', active: path === '/characters' },
			{
				title: 'Collections',
				href: '/collections',
				icon: 'collection',
				active: path === '/collections'
			}
		];
		if (nav) {
			list.splice(2, 0, {
				title: 'Sell',
				href: '/sell/modules',
				icon: 'contract',
				active: path.startsWith('/sell/modules')
			});
			list.push({
				title: 'Offers',
				href: '/offers',
				icon: 'offer',
				active: path.startsWith('/offers')
			});
			list.push({
				title: 'My modules',
				href: '/personal/modules',
				icon: 'cubes',
				active: path.startsWith('/personal/modules')
			});
			list.push({
				title: 'My locations',
				href: '/locations',
				icon: 'location',
				active: path.startsWith('/locations')
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
					{ title: 'My profile', href: profile, icon: 'users', active: path === profile },
					{
						title: 'My contracts',
						href: '/personal/contracts',
						icon: 'contract',
						active: path.startsWith('/personal/contracts')
					}
				]
			});
		}
		groups.push({
			label: 'Resources',
			items: [
				{
					title: 'All modules',
					href: '/all-modules',
					icon: 'cubes',
					active: path.startsWith('/all-modules')
				},
				...(nav?.user?.has_premium
					? [
							{
								title: 'Historic Sales',
								href: '/historic-sales',
								icon: 'contract' as const,
								active: path.startsWith('/historic-sales')
							}
						]
					: []),
				{
					title: 'Calculator',
					href: '/calculator',
					icon: 'calculator',
					active: path.startsWith('/calculator')
				},
				{
					title: 'Statistics',
					href: '/statistics',
					icon: 'chart',
					active: path.startsWith('/statistics')
				}
			]
		});
		groups.push({
			items: [
				{
					title: 'Contract review',
					href: '/moderator/contracts',
					icon: 'contract',
					active: path.startsWith('/moderator/contracts')
				},
				{
					title: 'Documentation',
					href: '/documentation',
					icon: 'info',
					active: path.startsWith('/documentation')
				}
			]
		});
		if (nav?.user.is_admin) {
			groups.push({
				label: 'Admin',
				items: [
					{
						title: 'Console',
						href: '/admin',
						icon: 'cog',
						active: path.startsWith('/admin')
					}
				]
			});
		}
		return groups;
	});

	let moreOpen = $state(false);
</script>

<div class="z-40">
	<!-- Matches the page container, which grows by the sidebar width. -->
	<div class="mx-auto w-full max-w-7xl xl:max-w-[calc(var(--container-7xl)+250px+--spacing(6))] px-4">
		<div class="flex items-center gap-2 border-b border-border py-3">
			<a href="/" class="flex shrink-0 items-center py-1 transition hover:opacity-80">
				<Logo class="size-8 text-primary" />
			</a>

			<nav class="ml-1 flex items-center gap-0.5">
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

				<div
					class="relative"
					role="presentation"
					onmouseenter={() => (moreOpen = true)}
					onmouseleave={() => (moreOpen = false)}
					onfocusin={() => (moreOpen = true)}
					onfocusout={() => (moreOpen = false)}
				>
					<button
						type="button"
						class="flex items-center gap-1 px-3 py-2 text-[0.82rem] font-medium transition focus:outline-none {moreOpen
							? 'bg-white/[0.07] text-white'
							: 'text-white/60 hover:bg-white/[0.04] hover:text-white'}"
					>
						More
						<ChevronDown class="size-3 text-white/45 transition {moreOpen ? 'rotate-180' : ''}" />
					</button>

					{#if moreOpen}
						<div class="absolute top-full left-0 z-50 w-52 border border-border bg-card p-1">
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
										onclick={() => (moreOpen = false)}
									>
										<NavIcon icon={item.icon} class="text-white/40" />
										{item.title}
									</a>
								{/each}
							{/each}
						</div>
					{/if}
				</div>
			</nav>

			<div class="flex-1"></div>

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
					<span class="sr-only">Log in</span>
				</a>
			{/if}
		</div>
	</div>
</div>
