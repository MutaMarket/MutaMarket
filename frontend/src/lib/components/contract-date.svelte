<script lang="ts">
  // The legacy Tables/Contracts/ContractDate.vue: absolute timestamp
  // over a muted distance-to-now line.
  import { parseDbTimestamp, relativeTime } from '$lib/duration';

  let { date }: { date: string | null } = $props();

  const seconds = $derived(date !== null ? parseDbTimestamp(date) : null);

  function formatted(unixSeconds: number): string {
    const value = new Date(unixSeconds * 1000);
    const pad = (part: number) => String(part).padStart(2, '0');
    return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())} ${pad(value.getHours())}:${pad(value.getMinutes())}`;
  }
</script>

{#if seconds !== null}
  <div>
    <span class="text-sm">{formatted(seconds)}</span>
    <span class="block text-xs text-muted-foreground">
      {relativeTime(seconds - Date.now() / 1000)}
    </span>
  </div>
{:else}
  <span class="text-sm text-muted-foreground">N/A</span>
{/if}
