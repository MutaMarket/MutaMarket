<script lang="ts">
	// The endpoint reference, rendered from the generated OpenAPI document.
	// Every path, parameter, status and schema on this page comes from the
	// server's own route definitions.
	import DocsNav from '$lib/components/docs-nav.svelte';
	import PageMeta from '$lib/components/page-meta.svelte';
	import SchemaTable from '$lib/components/schema-table.svelte';
	import { groupOperations, reachableSchemas } from '$lib/openapi';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const groups = $derived(groupOperations(data.spec));
	const schemas = $derived(data.spec.components?.schemas ?? {});
	const server = $derived(data.spec.servers?.[0]?.url ?? '');

	/** Only the schemas the endpoints actually reach. */
	const shown = $derived(
		reachableSchemas(
			data.spec,
			groups
				.flatMap((group) => group.operations)
				.flatMap((operation) => [
					operation.requestBody,
					...operation.responses.map((response) => response.schemaRef)
				])
				.filter((name): name is string => name !== null && name in schemas)
		).sort()
	);

	const METHOD_COLOR: Record<string, string> = {
		GET: 'text-primary border-primary/40 bg-primary/10',
		POST: 'text-[#22d3ee] border-[#22d3ee]/40 bg-[#22d3ee]/10',
		PUT: 'text-[#fab219] border-[#fab219]/40 bg-[#fab219]/10',
		DELETE: 'text-negative border-negative/40 bg-negative/10'
	};

	function statusClass(status: string): string {
		if (status.startsWith('2')) return 'text-positive';
		if (status.startsWith('4')) return 'text-[#ec835a]';
		return 'text-negative';
	}
</script>

<PageMeta title="API reference" description="Every endpoint of the MutaMarket public API." />

<div class="lg:grid lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-6">
	<DocsNav sections={data.sections} current="api" />

	<div class="hud-frame min-w-0">
		<div class="border-b border-border px-6 py-4">
			<span class="hud-label">Documentation // API</span>
			<h1 class="mt-1 text-2xl font-bold">Endpoint reference</h1>
			{#if server}
				<p class="mt-2 text-sm text-muted-foreground">
					Base URL <code class="bg-card-2 px-1.5 py-0.5 font-mono text-xs">{server}</code>
				</p>
			{/if}
		</div>

		<!-- The index: every endpoint at a glance before any detail. -->
		<div class="border-b border-border px-6 py-4">
			<ul class="flex flex-col gap-1">
				{#each groups as group (group.name)}
					{#each group.operations as operation (operation.id)}
						<li>
							<a
								href="#{operation.id}"
								class="flex flex-wrap items-baseline gap-2 py-0.5 text-sm hover:bg-white/[0.03]"
							>
								<span
									class="w-14 shrink-0 border px-1.5 text-center font-mono text-2xs {METHOD_COLOR[
										operation.method
									] ?? 'text-muted-foreground border-border'}"
								>
									{operation.method}
								</span>
								<code class="font-mono text-xs">{operation.path}</code>
								<span class="min-w-0 truncate text-xs text-muted-foreground">
									{operation.summary}
								</span>
							</a>
						</li>
					{/each}
				{/each}
			</ul>
		</div>

		<div class="docs-prose px-6 py-6">
			{#each groups as group (group.name)}
				<h2 id={group.name.toLowerCase()}>{group.name}</h2>
				{#if group.description}
					<p>{group.description}</p>
				{/if}

				{#each group.operations as operation (operation.id)}
					<section class="mt-8 scroll-mt-20" id={operation.id}>
						<div class="flex flex-wrap items-center gap-2">
							<span
								class="border px-2 py-0.5 font-mono text-xs {METHOD_COLOR[operation.method] ??
									'text-muted-foreground border-border'}"
							>
								{operation.method}
							</span>
							<code class="bg-card-2 px-2 py-0.5 font-mono text-sm">{operation.path}</code>
						</div>

						{#if operation.summary}
							<h3 class="!mt-3 !mb-1">{operation.summary}</h3>
						{/if}
						{#each operation.description.split('\n\n') as paragraph (paragraph)}
							{#if paragraph.trim()}
								<p class="text-sm">{paragraph}</p>
							{/if}
						{/each}

						{#if operation.parameters.length > 0}
							<h4>Parameters</h4>
							<table>
								<thead>
									<tr><th>Name</th><th>In</th><th>Type</th><th>Description</th></tr>
								</thead>
								<tbody>
									{#each operation.parameters as parameter (parameter.name)}
										<tr>
											<td class="font-mono text-xs whitespace-nowrap">
												{parameter.name}
												{#if parameter.required}
													<span class="text-negative" title="required">*</span>
												{/if}
											</td>
											<td class="text-xs text-muted-foreground">{parameter.location}</td>
											<td class="font-mono text-xs">{parameter.type}</td>
											<td class="text-sm">{parameter.description}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						{/if}

						{#if operation.requestBody}
							<h4>Request body</h4>
							<SchemaTable name={operation.requestBody} {schemas} />
						{/if}

						<h4>Responses</h4>
						<div class="flex flex-col gap-3">
							{#each operation.responses as response (response.status)}
								<div class="border border-border">
									<div
										class="flex flex-wrap items-baseline gap-3 border-b border-border bg-card-2 px-3 py-1.5"
									>
										<span class="font-mono text-sm font-semibold {statusClass(response.status)}">
											{response.status}
										</span>
										<span class="text-sm">{response.description}</span>
										{#if response.schema}
											<span class="ml-auto font-mono text-2xs text-muted-foreground">
												{#if response.schemaRef}
													<a href="#schema-{response.schemaRef}">{response.schema}</a>
												{:else}
													{response.schema}
												{/if}
											</span>
										{/if}
									</div>
									{#if response.example}
										<pre
											class="overflow-x-auto p-3 font-mono text-xs">{JSON.stringify(
												response.example,
												null,
												2
											)}</pre>
									{/if}
								</div>
							{/each}
						</div>
					</section>
				{/each}
			{/each}

			<h2 id="schemas" class="mt-12">Schemas</h2>
			{#each shown as name (name)}
				<section class="mt-6 scroll-mt-20" id="schema-{name}">
					<h3 class="!mb-1 font-mono">{name}</h3>
					{#if schemas[name]?.description}
						<p class="text-sm">{schemas[name].description}</p>
					{/if}
					<SchemaTable {name} {schemas} />
				</section>
			{/each}
		</div>
	</div>
</div>
