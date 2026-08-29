<script lang="ts">
	import { page } from '$app/state';
	import PageMeta from '$lib/components/page-meta.svelte';

	// SvelteKit routes every failure through this one component, where
	// legacy had a page per status (Errors/NotFound.vue and friends);
	// their meta copy is keyed by status here.
	const ERRORS: Record<number, { title: string; description: string }> = {
		400: {
			title: '400',
			description:
				'Invalid request. The server could not understand the request due to invalid syntax.'
		},
		403: { title: '403', description: 'You are not authorized to access this page.' },
		404: { title: '404', description: 'The page you are looking for does not exist.' },
		500: { title: '500', description: 'We encountered an internal server error.' },
		503: {
			title: 'Maintenance in progress',
			description: 'The service is temporarily unavailable.'
		}
	};

	const meta = $derived(
		ERRORS[page.status] ?? {
			title: String(page.status),
			description: page.error?.message ?? 'Something went wrong'
		}
	);
</script>

<PageMeta title={meta.title} description={meta.description} />

{#if page.status === 404}
	<h1>Page not found</h1>
{:else}
	<h1>{page.error?.message ?? 'Something went wrong'}</h1>
{/if}
