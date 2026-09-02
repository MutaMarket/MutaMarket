<script lang="ts">
  // The raffle prize pool, the legacy Admin/RafflePage: a create card
  // that loads one prize per redemption code above the pool list with
  // its status and winner columns. Prizes are only created and drawn,
  // never edited or deleted, like the legacy page.
  import { goto, invalidateAll } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { hasWinner, poolCounts, statusLabel } from '$lib/raffles';
  import { notifyError, notifySuccess } from '$lib/toast';
  import PageMeta from '$lib/components/page-meta.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  let submitting = $state(false);
  // Mirrors the applied search from the URL; typing edits it until the
  // next navigation re-seeds it.
  let typeSearch = $state('');
  $effect(() => {
    typeSearch = data.raffles.type_search;
  });

  let form = $state({ name: '', description: '', type_id: null as number | null, codes: '' });

  const counts = $derived(poolCounts(data.raffles.raffle_items));

  // A plain navigation, not a nested form: the type search shares the
  // create card with the create form.
  async function searchTypes() {
    const query = typeSearch ? `?type_search=${encodeURIComponent(typeSearch)}` : '';
    await goto(`/admin/raffles${query}`, { keepFocus: true, noScroll: true });
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    submitting = true;
    try {
      const response = await fetch('/raffles', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(form),
      });
      if (response.ok || response.redirected) {
        notifySuccess('Prizes created', 'The codes were added to the raffle pool.');
        form = { name: '', description: '', type_id: null, codes: '' };
        await invalidateAll();
        return;
      }
      const body = await response.json().catch(() => null);
      notifyError('Could not create prizes', body?.message ?? 'Please check the form and retry.');
    } finally {
      submitting = false;
    }
  }
</script>

<PageMeta title="Admin - Raffles" description="Manage raffle items" />

<div class="space-y-6">
  <section class="hud-frame p-4">
    <h2 class="text-lg font-medium">Add prizes</h2>
    <p class="text-muted-foreground mb-4 text-sm">
      One prize is created per redemption code, all sharing the name, description and type.
    </p>

    <form class="grid gap-4 sm:grid-cols-2" onsubmit={create}>
      <div class="space-y-1">
        <Label for="raffle-name">Name</Label>
        <Input id="raffle-name" bind:value={form.name} required />
      </div>
      <div class="space-y-1">
        <Label for="raffle-description">Description</Label>
        <Input id="raffle-description" bind:value={form.description} />
      </div>

      <div class="space-y-1 sm:col-span-2">
        <Label for="raffle-codes">Redemption codes (one per line)</Label>
        <textarea
          id="raffle-codes"
          bind:value={form.codes}
          class="border-input bg-background min-h-24 w-full rounded-md border px-3 py-2 font-mono text-sm"
          required></textarea>
      </div>

      <div class="space-y-1 sm:col-span-2">
        <Label>Prize type (optional)</Label>
        <div class="flex gap-2">
          <Input bind:value={typeSearch} placeholder="Search a type by name" />
          <Button type="button" variant="secondary" onclick={searchTypes}>Search</Button>
        </div>
        {#if data.raffles.types.length > 0}
          <div class="mt-2 flex flex-wrap gap-2">
            {#each data.raffles.types as type (type.id)}
              <Button
                size="sm"
                type="button"
                variant={form.type_id === type.id ? 'default' : 'secondary'}
                onclick={() => (form.type_id = form.type_id === type.id ? null : type.id)}
              >
                {type.name}
              </Button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="sm:col-span-2">
        <Button disabled={submitting} type="submit">Create prizes</Button>
      </div>
    </form>
  </section>

  <section class="hud-frame p-4">
    <div class="mb-4 flex flex-wrap items-baseline gap-x-4 gap-y-1">
      <h2 class="text-lg font-medium">Prize pool</h2>
      <span class="text-muted-foreground text-sm">
        {counts.pending} pending, {counts.active} awaiting claim, {counts.claimed} claimed
      </span>
    </div>

    {#if data.raffles.raffle_items.length > 0}
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead class="text-muted-foreground text-left">
            <tr>
              <th class="py-2 pr-4 font-medium">Prize</th>
              <th class="py-2 pr-4 font-medium">Status</th>
              <th class="py-2 pr-4 font-medium">Winner</th>
              <th class="py-2 font-medium">Expires</th>
            </tr>
          </thead>
          <tbody>
            {#each data.raffles.raffle_items as item (item.id)}
              <tr class="border-border border-t">
                <td class="py-2 pr-4">
                  <div class="flex items-center gap-2">
                    {#if item.type}
                      <img
                        alt={item.type.name ?? ''}
                        class="size-8 rounded"
                        src="https://images.evetech.net/types/{item.type.id}/icon?size=64"
                      />
                    {/if}
                    <div>
                      <div>{item.name}</div>
                      {#if item.description}
                        <div class="text-muted-foreground text-xs">{item.description}</div>
                      {/if}
                    </div>
                  </div>
                </td>
                <td class="py-2 pr-4">{statusLabel(item.status)}</td>
                <td class="py-2 pr-4">
                  {#if hasWinner(item)}
                    {item.winner?.name ?? '-'}
                  {:else}
                    <span class="text-muted-foreground">-</span>
                  {/if}
                </td>
                <td class="text-muted-foreground py-2">{item.expires_at ?? '-'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <p class="text-muted-foreground text-sm">No prizes in the pool yet.</p>
    {/if}
  </section>
</div>
