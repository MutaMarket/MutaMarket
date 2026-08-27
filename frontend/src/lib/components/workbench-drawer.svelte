<script lang="ts">
	// The workbench drawer, feature-matching the legacy Workbench.vue
	// (resizable right panel, share link, collection conversion, clear,
	// combined price and set DPS) with a redesigned body: a List view of
	// compact rows and a Compare view — an attributes × modules matrix
	// highlighting the best roll per attribute, which is what a bench of
	// same-type modules is for.
	import { FlaskConical, GripHorizontal, Link2, Layers, Trash2, X } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import GameImage from './game-image.svelte';
	import ModuleCard from './module-card.svelte';
	import { Button } from '$lib/components/ui/button';
	import { defaultDisplaySettings } from '$lib/display';
	import { isVisual } from '$lib/attributes';
	import { toIskCompact } from '$lib/format-number';
	import { setEvaluation } from '$lib/set-evaluation';
	import { notifySuccess } from '$lib/toast';
	import {
		clearWorkbench,
		removeFromWorkbench,
		workbenchEntries,
		workbenchOpen,
		workbenchShareLink
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

	// The compare matrix: one row per attribute of the dominant type,
	// one column per module of that type; the best cell per row wins.
	const compare = $derived.by(() => {
		if (entries.length < 2) return null;
		const counts = new Map<number, number>();
		for (const entry of entries) {
			counts.set(entry.module.type.id, (counts.get(entry.module.type.id) ?? 0) + 1);
		}
		const [typeId] =
			[...counts.entries()].sort((a, b) => b[1] - a[1]).at(0) ?? [];
		if (typeId === undefined) return null;
		const columns = entries.filter((entry) => entry.module.type.id === typeId);
		if (columns.length < 2) return null;

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
		<header class="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
			<button
				type="button"
				class="cursor-ns-resize text-muted-foreground hover:text-foreground"
				aria-label="Resize workbench"
				onpointerdown={(event) => {
					event.preventDefault();
					onDragStart(event);
				}}
			>
				<GripHorizontal class="size-4" />
			</button>
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
				<div class="mx-auto max-w-5xl">
					<p class="mb-1 text-xs text-muted-foreground">
						Comparing {compare.columns.length}
						{compare.typeName} rolls — the best roll per attribute in lime, the worst in
						red.
					</p>
					<div class="overflow-x-auto">
						<table class="border-separate border-spacing-0 text-xs">
							<thead>
								<tr>
									<th class="min-w-56 align-bottom"></th>
									{#each compare.attributes as column (column.attribute.id)}
										<th class="relative h-28 min-w-16 align-bottom">
											<span
												class="absolute bottom-1 left-1/2 origin-bottom-left -rotate-45 font-normal whitespace-nowrap text-muted-foreground"
											>
												{column.attribute.display_name}
											</span>
										</th>
									{/each}
								</tr>
							</thead>
							<tbody>
								{#each compare.columns as entry, moduleIndex (entry.id)}
									<tr class="group">
										<td class="border-t border-border py-1.5 pr-4">
											<a
												class="flex items-center gap-2 hover:underline"
												href="/modules/{entry.module.slug}"
											>
												<GameImage
													src="https://images.evetech.net/types/{entry.module.type
														.id}/icon?size=64"
													alt=""
													class="size-7 rounded"
												/>
												<span class="flex flex-col">
													<span>
														{entry.module.estimated_value !== null
															? `Est. ${toIskCompact(entry.module.estimated_value)}`
															: 'No estimate'}
													</span>
													{#if entry.module.contract?.price != null}
														<span class="text-muted-foreground">
															{toIskCompact(entry.module.contract.price)} asked
														</span>
													{/if}
												</span>
											</a>
										</td>
										{#each compare.attributes as column (column.attribute.id)}
											{@const cell = column.cells[moduleIndex]}
											<td
												class="border-t border-border px-2 py-1.5 text-right group-hover:bg-card-2/40 {column.best ===
													moduleIndex && column.best !== column.worst
													? 'bg-primary/10 font-semibold text-primary'
													: column.worst === moduleIndex && column.best !== column.worst
														? 'text-red-500'
														: ''}"
											>
												{cell !== null
													? cell.value.toLocaleString('en-US', {
															maximumSignificantDigits: 4
														})
													: '—'}
											</td>
										{/each}
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
				<!-- The legacy horizontal card strip: full module cards. -->
				<div bind:this={strip} class="h-full overflow-x-auto overscroll-x-contain" onwheel={onWheel}>
					<div class="flex w-max gap-3 pb-2">
						{#each entries as entry (entry.id)}
							<div class="relative w-[280px] shrink-0">
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
	</aside>
{/if}
