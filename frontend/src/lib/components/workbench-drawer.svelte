<script lang="ts">
	// The workbench drawer, feature-matching the legacy Workbench.vue
	// (resizable right panel, share link, collection conversion, clear,
	// combined price and set DPS) with a redesigned body: a List view of
	// compact rows and a Compare view — an attributes × modules matrix
	// highlighting the best roll per attribute, which is what a bench of
	// same-type modules is for.
	import {
		ArrowDown,
		ArrowUp,
		ArrowUpDown,
		EllipsisVertical,
		ExternalLink,
		FlaskConical,
		HandCoins,
		Link2,
		Layers,
		Trash2,
		X
	} from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import GameImage from './game-image.svelte';
	import ModuleCard from './module-card.svelte';
	import ModuleMenuItems from './module-menu-items.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { defaultDisplaySettings } from '$lib/display';
	import { openMakeOffer, sentOffers } from '$lib/make-offer';
	import { openContractInGame } from '$lib/open-contract';
	import { attributeFormattedValue, isVisual } from '$lib/attributes';
	import { toIskCompact } from '$lib/format-number';
	import { setEvaluation } from '$lib/set-evaluation';
	import { notifySuccess } from '$lib/toast';
	import {
		clearWorkbench,
		removeFromWorkbench,
		workbenchEntries,
		workbenchOpen,
		workbenchShareLink,
		type WorkbenchEntry
	} from '$lib/workbench';

	const entries = $derived($workbenchEntries);
	const open = $derived($workbenchOpen);

	let view = $state<'list' | 'compare'>('list');
	// A bottom sheet (a side panel covered the page content), resizable
	// by dragging its top edge; height persisted like the legacy width.
	let height = $state(540);
	let dragging = $state(false);
	let dragStartY = 0;
	let dragStartHeight = 0;

	$effect(() => {
		const stored = Number(localStorage.getItem('workbench_height'));
		if (Number.isFinite(stored) && stored >= 300) {
			height = stored;
		}
	});

	function onDragStart(event: PointerEvent) {
		dragging = true;
		dragStartY = event.clientY;
		dragStartHeight = height;
	}

	function onPointerMove(event: PointerEvent) {
		if (dragging) {
			height = Math.min(
				Math.max(dragStartHeight - (event.clientY - dragStartY), 300),
				Math.round(window.innerHeight * 0.85)
			);
		}
	}

	function onPointerUp() {
		if (dragging) {
			dragging = false;
			localStorage.setItem('workbench_height', String(height));
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			workbenchOpen.set(false);
		}
	}

	const combinedPrice = $derived(
		entries.reduce((sum, entry) => sum + (entry.module.contract?.price ?? 0), 0)
	);
	const dpsIncrease = $derived(setEvaluation(entries.map((entry) => entry.module)));

	// The bench's comparable types (two or more rolls), for the type
	// toggle when several kinds of module are benched at once.
	const compareTypes = $derived.by(() => {
		const byType = new Map<number, { id: number; name: string; count: number }>();
		for (const entry of entries) {
			const existing = byType.get(entry.module.type.id);
			if (existing) {
				existing.count += 1;
			} else {
				byType.set(entry.module.type.id, {
					id: entry.module.type.id,
					name: entry.module.type.name,
					count: 1
				});
			}
		}
		return [...byType.values()].filter((type) => type.count >= 2).sort((a, b) => b.count - a.count);
	});

	let compareTypeId = $state<number | null>(null);

	/** Column sort: an attribute id, or 'module' for the estimate. */
	let compareSort = $state<{ key: number | 'module'; desc: boolean } | null>(null);

	function sortCompareBy(key: number | 'module') {
		compareSort =
			compareSort?.key === key ? { key, desc: !compareSort.desc } : { key, desc: true };
	}

	// The compare matrix of the chosen (or dominant) type: modules as
	// rows, attributes as columns, best/worst per attribute.
	const compare = $derived.by(() => {
		const typeId =
			compareTypeId !== null && compareTypes.some((type) => type.id === compareTypeId)
				? compareTypeId
				: (compareTypes.at(0)?.id ?? null);
		if (typeId === null) return null;
		let columns = entries.filter((entry) => entry.module.type.id === typeId);
		if (columns.length < 2) return null;

		if (compareSort !== null) {
			const sort = compareSort;
			const key = (entry: WorkbenchEntry) =>
				sort.key === 'module'
					? (entry.module.contract?.price ?? entry.module.estimated_value ?? -Infinity)
					: (entry.module.mutated_attributes.find(
							(attribute) => attribute.id === sort.key
						)?.fraction_absolute ?? -Infinity);
			columns = [...columns].sort((a, b) => (sort.desc ? key(b) - key(a) : key(a) - key(b)));
		}

		const template = columns[0].module.mutated_attributes.filter((attribute) =>
			isVisual(attribute)
		);
		const attributes = template.map((attribute) => {
			const cells = columns.map((entry) => {
				const match = entry.module.mutated_attributes.find(
					(candidate) => candidate.id === attribute.id
				);
				return match ?? null;
			});
			// Best/worst by the absolute roll fraction (the -10..+10
			// score's source), direction-aware by construction.
			let best = -1;
			let worst = -1;
			let bestFraction = -Infinity;
			let worstFraction = Infinity;
			cells.forEach((cell, index) => {
				if (cell === null) return;
				if (cell.fraction_absolute > bestFraction) {
					bestFraction = cell.fraction_absolute;
					best = index;
				}
				if (cell.fraction_absolute < worstFraction) {
					worstFraction = cell.fraction_absolute;
					worst = index;
				}
			});
			return { attribute, cells, best, worst };
		});
		return { typeName: columns[0].module.type.name, columns, attributes };
	});

	function share() {
		void navigator.clipboard.writeText(workbenchShareLink(entries));
		notifySuccess('Link copied!', 'Share it to show this workbench to anyone.');
	}

	async function toCollection() {
		const response = await fetch('/workbench-collections', {
			method: 'POST',
			redirect: 'follow'
		});
		if (response.ok) {
			notifySuccess('Collection created!', 'Your workbench is now a collection.');
			workbenchOpen.set(false);
			await goto(new URL(response.url).pathname);
		}
	}

	let strip = $state<HTMLDivElement | null>(null);
	// The legacy desktop strip: the wheel scrolls the cards sideways.
	function onWheel(event: WheelEvent) {
		if (strip !== null && view === 'list') {
			event.preventDefault();
			strip.scrollLeft += event.deltaY;
		}
	}

	const SEGMENT =
		'flex h-7 items-center gap-1.5 rounded-[5px] px-2.5 text-xs transition-colors';
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} onkeydown={onKeydown} />

{#if entries.length > 0 && !open}
	<!-- The collapsed pill, the legacy WorkbenchCollapsed. -->
	<button
		type="button"
		class="fixed right-4 bottom-4 z-40 flex cursor-pointer items-center gap-2 rounded-full border border-border bg-card-1 py-2 pr-4 pl-3 shadow-lg transition hover:brightness-125"
		onclick={() => workbenchOpen.set(true)}
	>
		<FlaskConical class="size-4 text-primary" />
		<span class="text-sm">Workbench</span>
		<span class="rounded-full bg-primary px-1.5 text-xs font-semibold text-primary-foreground">
			{entries.length}
		</span>
	</button>
{/if}

{#if open}
	<aside
		class="fixed right-0 bottom-0 left-0 z-50 flex flex-col border-t border-border bg-card shadow-2xl"
		style="height: min({height}px, 85vh)"
	>
		<Tooltip.Provider delayDuration={300}>
		<!-- The whole top edge is the resize handle. -->
		<div
			role="separator"
			aria-label="Resize workbench"
			class="group/resize absolute -top-1.5 right-0 left-0 z-10 h-3 cursor-ns-resize touch-none"
			onpointerdown={(event) => {
				event.preventDefault();
				onDragStart(event);
			}}
		>
			<div
				class="absolute top-1 right-0 left-0 h-0.5 transition-colors group-hover/resize:bg-primary/60 {dragging
					? 'bg-primary'
					: ''}"
			></div>
		</div>
		<header class="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
			<FlaskConical class="size-4 text-primary" />
			<h2 class="text-sm font-semibold">Workbench</h2>
			<span class="rounded-full bg-card-2 px-2 py-0.5 text-xs text-muted-foreground">
				{entries.length}
			</span>

			<div class="ml-3 flex rounded-[7px] border border-border bg-card-2 p-0.5">
				<button
					type="button"
					class="{SEGMENT} {view === 'list'
						? 'bg-primary text-primary-foreground'
						: 'text-muted-foreground hover:text-foreground'}"
					onclick={() => (view = 'list')}
				>
					List
				</button>
				<button
					type="button"
					class="{SEGMENT} {view === 'compare'
						? 'bg-primary text-primary-foreground'
						: 'text-muted-foreground hover:text-foreground'}"
					onclick={() => (view = 'compare')}
				>
					Compare
				</button>
			</div>

			<div class="ml-auto flex items-center gap-1">
				<Button variant="ghost" size="sm" class="gap-1.5" onclick={share}>
					<Link2 class="size-3.5" />
					Share
				</Button>
				<Button variant="ghost" size="sm" class="gap-1.5" onclick={toCollection}>
					<Layers class="size-3.5" />
					To collection
				</Button>
				<Button
					variant="ghost"
					size="sm"
					class="gap-1.5 text-muted-foreground"
					onclick={() => clearWorkbench()}
				>
					<Trash2 class="size-3.5" />
					Clear
				</Button>
				<Button
					variant="ghost"
					size="icon"
					class="size-8"
					aria-label="Close workbench"
					onclick={() => workbenchOpen.set(false)}
				>
					<X class="size-4" />
				</Button>
			</div>
		</header>

		<div class="min-h-0 grow overflow-y-auto p-3">
			{#if entries.length === 0}
				<p class="p-4 text-sm text-muted-foreground">
					Your workbench is empty. Add modules from any module's menu to compare and share
					them.
				</p>
			{:else if view === 'compare' && compare !== null}
				<div class="px-1">
					{#if compareTypes.length > 1}
						<div class="mb-2 flex rounded-[7px] border border-border bg-card-2 p-0.5 self-start w-max">
							{#each compareTypes as type (type.id)}
								<button
									type="button"
									class="{SEGMENT} {compare.columns[0].module.type.id === type.id
										? 'bg-primary text-primary-foreground'
										: 'text-muted-foreground hover:text-foreground'}"
									onclick={() => (compareTypeId = type.id)}
								>
									<GameImage
										src="https://images.evetech.net/types/{type.id}/icon?size=64"
										alt=""
										class="size-4 rounded-sm"
									/>
									{type.name.replace('Abyssal', '').trim()}
									<span class="opacity-70">{type.count}</span>
								</button>
							{/each}
						</div>
					{/if}
					<div class="overflow-x-auto">
						<table class="w-full border-separate border-spacing-0 text-sm">
							<thead>
								<tr>
									<th class="w-52 min-w-52 pb-1 align-bottom">
										<button
											type="button"
											class="flex cursor-pointer items-center gap-1 text-xs font-normal text-muted-foreground hover:text-foreground"
											onclick={() => sortCompareBy('module')}
										>
											Price
											{#if compareSort?.key === 'module'}
												{#if compareSort.desc}
													<ArrowDown class="size-3" />
												{:else}
													<ArrowUp class="size-3" />
												{/if}
											{:else}
												<ArrowUpDown class="size-3 opacity-50" />
											{/if}
										</button>
									</th>
									{#each compare.attributes as column (column.attribute.id)}
										<th class="max-w-40 min-w-24 px-3 pb-1 align-bottom">
											<button
												type="button"
												class="flex w-full cursor-pointer items-center justify-end gap-1.5 hover:text-foreground"
												title={column.attribute.display_name}
												onclick={() => sortCompareBy(column.attribute.id)}
											>
												<GameImage
													src="/img/icons/{column.attribute.id}.png"
													alt=""
													class="size-5 shrink-0"
												/>
												<span class="truncate text-xs font-normal text-muted-foreground">
													{column.attribute.display_name}
												</span>
												{#if compareSort?.key === column.attribute.id}
													{#if compareSort.desc}
														<ArrowDown class="size-3 shrink-0 text-foreground" />
													{:else}
														<ArrowUp class="size-3 shrink-0 text-foreground" />
													{/if}
												{:else}
													<ArrowUpDown class="size-3 shrink-0 text-muted-foreground opacity-50" />
												{/if}
											</button>
										</th>
									{/each}
									<th class="w-40 min-w-40"></th>
								</tr>
							</thead>
							<tbody>
								{#each compare.columns as entry, moduleIndex (entry.id)}
									<tr class="group">
										<td class="border-t border-border py-2 pr-4 group-hover:bg-card-2/40">
											<a
												class="flex items-center gap-2.5 hover:underline"
												href="/modules/{entry.module.slug}"
											>
												<GameImage
													src="https://images.evetech.net/types/{entry.module.type
														.id}/icon?size=64"
													alt=""
													class="size-9 rounded"
												/>
												<!-- The asked price leads: it is what the buyer
												     actually decides on; the estimate ranks second. -->
												<span class="flex flex-col leading-tight">
													{#if entry.module.contract?.price != null}
														<span>{toIskCompact(entry.module.contract.price)}</span>
														<span class="text-xs text-muted-foreground">
															{entry.module.estimated_value !== null
																? `Est. ${toIskCompact(entry.module.estimated_value)}`
																: 'No estimate'}
														</span>
													{:else}
														<span>
															{entry.module.estimated_value !== null
																? `Est. ${toIskCompact(entry.module.estimated_value)}`
																: 'No estimate'}
														</span>
														{#if entry.module.public_asset}
															<span class="text-xs text-muted-foreground">
																sold by {entry.module.public_asset.owner.name}
															</span>
														{/if}
													{/if}
												</span>
											</a>
										</td>
										{#each compare.attributes as column (column.attribute.id)}
											{@const cell = column.cells[moduleIndex]}
											<td
												class="border-t border-border px-3 py-2 text-right tabular-nums group-hover:bg-card-2/40 {column.best ===
													moduleIndex && column.best !== column.worst
													? 'font-semibold text-primary'
													: column.worst === moduleIndex && column.best !== column.worst
														? 'text-red-500'
														: ''}"
											>
												{cell !== null ? attributeFormattedValue(cell) : '—'}
											</td>
										{/each}
										<td class="border-t border-border py-2 pl-3 group-hover:bg-card-2/40">
											<div class="flex items-center justify-end gap-0.5">
												{#if entry.module.public_asset}
													{@const myOffer = $sentOffers.get(entry.module.id)}
													<Tooltip.Root>
														<Tooltip.Trigger>
															{#snippet child({ props })}
																{#if myOffer !== undefined}
																	<Button
																		{...props}
																		variant="ghost"
																		size="icon"
																		class="size-7"
																		href="/offers/{myOffer}"
																	>
																		<HandCoins class="size-4 text-primary" />
																	</Button>
																{:else}
																	<Button
																		{...props}
																		variant="ghost"
																		size="icon"
																		class="size-7"
																		onclick={() => openMakeOffer(entry.module)}
																	>
																		<HandCoins class="size-4" />
																	</Button>
																{/if}
															{/snippet}
														</Tooltip.Trigger>
														<Tooltip.Content>
															{myOffer !== undefined ? 'Go to offer' : 'Make offer'}
														</Tooltip.Content>
													</Tooltip.Root>
												{/if}
												{#if entry.module.contract}
													<Tooltip.Root>
														<Tooltip.Trigger>
															{#snippet child({ props })}
																<Button
																	{...props}
																	variant="ghost"
																	size="icon"
																	class="size-7"
																	onclick={() => openContractInGame(entry.module.contract?.id)}
																>
																	<ExternalLink class="size-4" />
																</Button>
															{/snippet}
														</Tooltip.Trigger>
														<Tooltip.Content>Open contract in game</Tooltip.Content>
													</Tooltip.Root>
												{/if}
												<DropdownMenu.Root>
													<DropdownMenu.Trigger>
														{#snippet child({ props })}
															<Button
																{...props}
																variant="ghost"
																size="icon"
																class="size-7"
																title="Module menu"
															>
																<EllipsisVertical class="size-4" />
															</Button>
														{/snippet}
													</DropdownMenu.Trigger>
													<!-- align end: the trigger sits at the page's right
													     edge, a start-aligned menu runs off screen. -->
													<DropdownMenu.Content align="end" collisionPadding={8}>
														<ModuleMenuItems module={entry.module} kind="dropdown" />
													</DropdownMenu.Content>
												</DropdownMenu.Root>
												<Tooltip.Root>
													<Tooltip.Trigger>
														{#snippet child({ props })}
															<Button
																{...props}
																variant="ghost"
																size="icon"
																class="size-7 text-muted-foreground hover:text-red-500"
																onclick={() => removeFromWorkbench(entry.id)}
															>
																<X class="size-4" />
															</Button>
														{/snippet}
													</Tooltip.Trigger>
													<Tooltip.Content>Remove from workbench</Tooltip.Content>
												</Tooltip.Root>
											</div>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{:else}
				{#if view === 'compare'}
					<p class="mb-2 text-xs text-muted-foreground">
						Add at least two modules of the same type to compare them side by side.
					</p>
				{/if}
				<!-- Full module cards packed like the main module grid: same
				     container, and the wrapper mirrors the card's masonry
				     row span (the card is not a direct grid child here). -->
				<div class="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4 pb-2">
					{#each entries as entry (entry.id)}
						<div
							class="relative"
							style="grid-row: span {2 +
								entry.module.mutated_attributes.filter(isVisual).length}"
						>
							<button
								type="button"
								class="absolute -top-1.5 -left-1.5 z-10 grid size-6 cursor-pointer place-items-center rounded-full border border-border bg-card-2 text-muted-foreground shadow hover:text-red-500"
								aria-label="Remove from workbench"
								onclick={() => removeFromWorkbench(entry.id)}
							>
								<X class="size-3.5" />
							</button>
							<ModuleCard module={entry.module} settings={defaultDisplaySettings()} />
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<footer
			class="flex shrink-0 items-center gap-6 border-t border-border bg-card-1 px-4 py-2 text-xs"
		>
			<span>
				<span class="hud-label mr-1.5">Modules</span>
				{entries.length}
			</span>
			<span>
				<span class="hud-label mr-1.5">Combined price</span>
				{combinedPrice > 0 ? toIskCompact(combinedPrice) : '—'}
			</span>
			{#if dpsIncrease !== null}
				<span>
					<span class="hud-label mr-1.5">Set DPS</span>
					<span class="text-primary">+{dpsIncrease.toFixed(2)}%</span>
				</span>
			{/if}
		</footer>
		</Tooltip.Provider>
	</aside>
{/if}
