<script lang="ts">
	// The workbench drawer, feature-matching the legacy Workbench.vue
	// (resizable right panel, share link, collection conversion, clear,
	// combined price and set DPS) with a redesigned body: a List view of
	// compact rows and a Compare view — an attributes × modules matrix
	// highlighting the best roll per attribute, which is what a bench of
	// same-type modules is for.
	import { FlaskConical, GripVertical, Link2, Layers, Trash2, X } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import GameImage from './game-image.svelte';
	import { Button } from '$lib/components/ui/button';
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
	let width = $state(700);
	let dragging = $state(false);
	let dragStartX = 0;
	let dragStartWidth = 0;

	// The legacy workbench_width localStorage persistence.
	$effect(() => {
		const stored = Number(localStorage.getItem('workbench_width'));
		if (Number.isFinite(stored) && stored >= 380) {
			width = stored;
		}
	});

	function onDragStart(event: PointerEvent) {
		dragging = true;
		dragStartX = event.clientX;
		dragStartWidth = width;
	}

	function onPointerMove(event: PointerEvent) {
		if (dragging) {
			width = Math.min(Math.max(dragStartWidth - (event.clientX - dragStartX), 380), 1100);
		}
	}

	function onPointerUp() {
		if (dragging) {
			dragging = false;
			localStorage.setItem('workbench_width', String(width));
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
		const rows = template.map((attribute) => {
			const cells = columns.map((entry) => {
				const match = entry.module.mutated_attributes.find(
					(candidate) => candidate.id === attribute.id
				);
				return match ?? null;
			});
			// "Best" follows the bar direction: the highest bar fraction
			// is the best roll of the row.
			let best = -1;
			let bestBar = -Infinity;
			cells.forEach((cell, index) => {
				if (cell !== null && cell.bar > bestBar) {
					bestBar = cell.bar;
					best = index;
				}
			});
			return { attribute, cells, best };
		});
		return { typeName: columns[0].module.type.name, columns, rows };
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
		class="fixed top-0 right-0 z-50 flex h-full flex-col border-l border-border bg-card shadow-2xl"
		style="width: min({width}px, 100vw)"
	>
		<header class="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
			<button
				type="button"
				class="hidden cursor-ew-resize text-muted-foreground hover:text-foreground sm:block"
				aria-label="Resize workbench"
				onpointerdown={(event) => {
					event.preventDefault();
					onDragStart(event);
				}}
			>
				<GripVertical class="size-4" />
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
				<p class="mb-2 text-xs text-muted-foreground">
					Comparing {compare.columns.length}
					{compare.typeName} rolls — the best roll per attribute glows.
				</p>
				<div class="overflow-x-auto">
					<table class="w-full border-separate border-spacing-0 text-xs">
						<thead>
							<tr>
								<th class="sticky left-0 bg-card p-2 text-left font-normal text-muted-foreground"
									>Attribute</th
								>
								{#each compare.columns as entry (entry.id)}
									<th class="p-2 text-right font-normal">
										<a class="hover:underline" href="/modules/{entry.module.slug}">
											<GameImage
												src="https://images.evetech.net/types/{entry.module.type
													.id}/icon?size=64"
												alt=""
												class="mx-auto mb-1 size-7 rounded"
											/>
											<span class="block text-[10px] text-muted-foreground">
												{entry.module.estimated_value !== null
													? toIskCompact(entry.module.estimated_value)
													: '—'}
											</span>
										</a>
									</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each compare.rows as row (row.attribute.id)}
								<tr>
									<td
										class="sticky left-0 border-t border-border bg-card p-2 text-muted-foreground"
									>
										{row.attribute.display_name}
									</td>
									{#each row.cells as cell, index (index)}
										<td class="border-t border-border p-2 text-right">
											{#if cell !== null}
												<span
													class={row.best === index
														? 'rounded bg-primary/15 px-1.5 py-0.5 font-semibold text-primary'
														: ''}
												>
													{cell.value.toLocaleString('en-US', {
														maximumSignificantDigits: 4
													})}
												</span>
											{:else}
												<span class="text-muted-foreground">—</span>
											{/if}
										</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				{#if view === 'compare'}
					<p class="mb-2 text-xs text-muted-foreground">
						Add at least two modules of the same type to compare them side by side.
					</p>
				{/if}
				<ul class="flex flex-col gap-1">
					{#each entries as entry (entry.id)}
						<li
							class="flex items-center gap-3 rounded-md border border-border bg-card-1 p-2"
						>
							<GameImage
								src="https://images.evetech.net/types/{entry.module.type.id}/icon?size=64"
								alt=""
								class="size-9 rounded"
							/>
							<div class="min-w-0 grow">
								<a
									class="block truncate text-sm hover:underline"
									href="/modules/{entry.module.slug}"
								>
									{entry.module.type.name}
								</a>
								<span class="text-xs text-muted-foreground">
									{entry.module.estimated_value !== null
										? `Est. ${toIskCompact(entry.module.estimated_value)}`
										: 'No estimate'}
									{#if entry.module.contract?.price != null}
										· {toIskCompact(entry.module.contract.price)} asked
									{/if}
								</span>
							</div>
							<Button
								variant="ghost"
								size="icon"
								class="size-7 text-muted-foreground hover:text-red-500"
								aria-label="Remove from workbench"
								onclick={() => removeFromWorkbench(entry.id)}
							>
								<X class="size-4" />
							</Button>
						</li>
					{/each}
				</ul>
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
