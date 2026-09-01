<script lang="ts">
	// The appraise page, the legacy Modules/AddModulePage.vue: a hero
	// input for an in-game item link with live validation, the quick
	// paste / import-from-assets / EVE-mail info cards, and the AI
	// estimate notice. Ctrl+V anywhere submits straight from the
	// clipboard, like the legacy global paste handler.
	import { ClipboardPaste, Copy, Cpu, Info, Mail, Search } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { notifySuccess } from '$lib/toast';
	import PageMeta from '$lib/components/page-meta.svelte';

	let message = $state('');
	let submitting = $state(false);
	let serverError = $state<string | null>(null);
	let copied = $state(false);

	const loggedIn = $derived(page.data.nav !== null);

	/** The legacy in-game link shape. */
	const LINK_PATTERN = /showinfo:\d+\/\/\d+/;
	const valid = $derived(LINK_PATTERN.test(message));

	/** The MutaMate appraisal character (mail your links to it). */
	const MUTAMATE_CHARACTER_ID = 2121690232;

	async function submit(text: string) {
		if (!LINK_PATTERN.test(text) || submitting) {
			return;
		}
		submitting = true;
		serverError = null;
		try {
			const response = await fetch('/modules', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ message: text }),
				redirect: 'follow',
			});
			if (response.redirected) {
				await goto(new URL(response.url).pathname);
				return;
			}
			const body: { message?: string } = await response.json().catch(() => ({}));
			serverError = body.message ?? 'Failed to add module!';
		} finally {
			submitting = false;
		}
	}

	function onGlobalPaste(event: ClipboardEvent) {
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea')) {
			return;
		}
		const text = event.clipboardData?.getData('text') ?? '';
		if (LINK_PATTERN.test(text)) {
			message = text;
			void submit(text);
		}
	}

	function copyName() {
		navigator.clipboard.writeText('MutaMate');
		copied = true;
		setTimeout(() => (copied = false), 2000);
		notifySuccess('Copied!', 'Paste into the recipient field of an EVE Mail.');
	}
</script>

<PageMeta
	title="Add and appraise modules"
	description="Add and appraise modules on MutaMarket, the best place to buy and sell abyssal modules!"
/>
<svelte:window onpaste={onGlobalPaste} />

<div class="grid gap-8">
	<!-- Hero section with the main input -->
	<div class="hud-frame p-6">
		<div class="grid gap-4">
			<div class="flex items-center gap-3">
				<div class="rounded-lg bg-primary/10 p-2">
					<ClipboardPaste class="size-5 text-primary" />
				</div>
				<div>
					<h1 class="text-lg font-semibold">Paste an item link</h1>
					<p class="text-sm text-muted-foreground">
						Copy a module link from EVE chat and paste it here.
					</p>
				</div>
			</div>

			<form
				class="flex items-start gap-3"
				onsubmit={(event) => {
					event.preventDefault();
					void submit(message);
				}}
			>
				<div class="grid grow gap-1">
					<Input
						bind:value={message}
						data-valid={valid}
						class="h-12 text-base data-[valid=true]:border-positive"
						placeholder="<url=showinfo:typeID//itemID>Module Name</url>"
					/>
					{#if serverError}
						<p class="text-xs text-negative">{serverError}</p>
					{:else if message && !valid}
						<p class="text-xs text-negative">
							Could not detect an item link. Expected format: showinfo:typeID//itemID
						</p>
					{/if}
				</div>
				<Button type="submit" disabled={!valid || submitting} class="h-12 px-6">Appraise</Button>
			</form>
		</div>
	</div>

	<!-- Info cards row -->
	<div class="grid gap-4 lg:grid-cols-3">
		<div class="hud-frame flex items-start gap-4 p-5">
			<div class="shrink-0 rounded-lg bg-primary/10 p-2">
				<Info class="size-5 text-primary" />
			</div>
			<div>
				<p class="text-sm font-medium">Quick paste</p>
				<p class="mt-1 text-sm text-muted-foreground">
					Press
					<kbd class="rounded border border-border bg-card-1 px-1.5 py-0.5 font-mono text-xs">
						Ctrl+V
					</kbd>
					anywhere on MutaMarket to appraise directly from your clipboard.
				</p>
			</div>
		</div>

		<div class="hud-frame flex items-start gap-4 p-5">
			<div class="shrink-0 rounded-lg bg-primary/10 p-2">
				<Search class="size-5 text-primary" />
			</div>
			<div>
				<p class="text-sm font-medium">Import from assets</p>
				<p class="mt-1 text-sm text-muted-foreground">
					{#if loggedIn}
						Import modules from your in-game assets via
						<a href="/personal/modules" class="text-primary hover:underline">My Modules</a>.
					{:else}
						<a href="/login" class="text-primary hover:underline">Log in with EVE SSO</a>
						to import from your assets.
					{/if}
				</p>
			</div>
		</div>

		<div class="hud-frame flex items-start gap-4 p-5">
			<div class="shrink-0 rounded-lg bg-primary/10 p-2">
				<Mail class="size-5 text-primary" />
			</div>
			<div>
				<p class="text-sm font-medium">Send via EVE Mail</p>
				<p class="mt-1 text-sm text-muted-foreground">Mail your module links to:</p>
				<Button variant="outline" size="sm" class="mt-2 gap-2 self-start" onclick={copyName}>
					<img
						alt=""
						class="size-5 rounded"
						src="https://images.evetech.net/characters/{MUTAMATE_CHARACTER_ID}/portrait?size=64"
					/>
					<span>MutaMate</span>
					<span
						class="flex items-center gap-1 text-xs transition-colors {copied
							? 'text-green-500'
							: 'text-muted-foreground'}"
					>
						<Copy class="size-3" />
						{copied ? 'Copied!' : 'Copy'}
					</span>
				</Button>
			</div>
		</div>
	</div>

	<!-- AI estimate notice -->
	<div class="hud-frame flex items-start gap-4 p-5">
		<div class="shrink-0 rounded-lg bg-primary/10 p-2">
			<Cpu class="size-5 text-primary" />
		</div>
		<div>
			<p class="text-sm font-medium">About AI estimates</p>
			<p class="mt-1 text-sm text-muted-foreground">
				AI estimates are a helpful starting point but should not be treated as definitive prices.
				Always cross-reference with recent contracts. Rare or exceptional rolls may be mispriced.
			</p>
		</div>
	</div>
</div>
