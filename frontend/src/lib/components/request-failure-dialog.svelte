<script lang="ts">
  // One captured request failure, in the order someone debugging reads
  // it: what we answered, which request it was, who made it, and then
  // the body we sent. The body is fetched on open because it is capped
  // at 8 KB and does not belong in the list.
  import * as Dialog from '$lib/components/ui/dialog';
  import { formatBody } from '$lib/admin-failures';
  import {
    accountLabel,
    failureAt,
    statusClass,
    truncationNote,
  } from '$lib/admin-request-failures';
  import { relativeTime } from '$lib/duration';
  import { t } from '$lib/i18n.svelte';
  import type { RequestFailureDetail, RequestFailureSummary } from '$lib/admin-types';

  let {
    failure = $bindable(),
    now,
  }: {
    /** The summary to open; null closes the dialog. */
    failure: RequestFailureSummary | null;
    now: number;
  } = $props();

  let detail = $state<RequestFailureDetail | null>(null);
  let loading = $state(false);

  $effect(() => {
    const summary = failure;
    if (summary === null) {
      detail = null;
      return;
    }
    loading = true;
    void (async () => {
      const response = await fetch(`/api/admin/request-failures/${summary.id}`);
      detail = response.ok ? await response.json() : null;
      loading = false;
    })();
  });

  const shown = $derived(detail ?? failure);
  const body = $derived(formatBody(detail?.response_body ?? null));
  const note = $derived(
    truncationNote(detail?.response_body ?? null, detail?.response_bytes ?? null),
  );
</script>

<Dialog.Root
  open={failure !== null}
  onOpenChange={(open) => {
    if (!open) failure = null;
  }}
>
  <Dialog.Content class="sm:max-w-3xl">
    {#if shown}
      <Dialog.Header>
        <Dialog.Title class="flex flex-wrap items-center gap-2">
          <span class="font-mono">{shown.method}</span>
          <span class="min-w-0 break-all">{shown.route.split(' ').slice(1).join(' ')}</span>
          <span
            class="rounded-full border border-border px-2 py-0.5 text-xs {statusClass(
              shown.status,
            ) === 'server'
              ? 'text-negative'
              : 'text-[#ec835a]'}"
          >
            {shown.status}
          </span>
        </Dialog.Title>
        <Dialog.Description>
          {relativeTime(failureAt(shown) - now)} · {shown.occurred_at} · {t(
            'admin.requestFailures.took',
            { ms: shown.duration_ms },
          )}
        </Dialog.Description>
      </Dialog.Header>

      <div class="flex max-h-[65vh] flex-col gap-4 overflow-y-auto">
        {#if shown.error_message}
          <!-- Nine times out of ten this is the whole answer. -->
          <p class="text-base text-foreground">{shown.error_message}</p>
        {/if}

        <div class="flex flex-col gap-1">
          <span class="hud-label">{t('admin.requestFailures.path')}</span>
          <code class="rounded bg-card-2 px-2 py-1.5 font-mono text-xs break-all">
            {shown.path}
          </code>
        </div>

        <div class="hud-panel px-3 py-2">
          <div class="hud-label">{t('admin.requestFailures.account')}</div>
          <div class="truncate text-sm">{accountLabel(shown)}</div>
        </div>

        {#if loading}
          <p class="text-sm text-muted-foreground">{t('admin.requestFailures.loading')}</p>
        {:else if detail}
          <div class="flex flex-col gap-1">
            <span class="hud-label">
              {t('admin.requestFailures.responseBody')}
              {#if note}
                <span class="ml-2 normal-case">({note})</span>
              {/if}
            </span>
            {#if body}
              <pre
                class="max-h-64 overflow-auto rounded bg-card-2 p-3 font-mono text-xs">{body}</pre>
            {:else}
              <p class="text-sm text-muted-foreground">{t('admin.requestFailures.noBody')}</p>
            {/if}
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">{t('admin.requestFailures.pruned')}</p>
        {/if}
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
