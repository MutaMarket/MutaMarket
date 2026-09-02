<script lang="ts">
  // One captured ESI failure, in the order someone debugging reads it:
  // what happened, what ESI said, which call it was, who made it, and
  // only then the raw bodies. The detail is fetched on open because the
  // bodies are capped at 8 KB each and must not ride the console poll.
  import * as Dialog from '$lib/components/ui/dialog';
  import {
    callerLabel,
    failureAt,
    failureLabel,
    formatBody,
    jobName,
    truncationNote,
  } from '$lib/admin-failures';
  import { relativeTime } from '$lib/duration';
  import { t } from '$lib/i18n.svelte';
  import type { EsiFailureDetail, EsiFailureSummary } from '$lib/admin-types';

  let {
    failure = $bindable(),
    now,
  }: {
    /** The summary to open; null closes the dialog. */
    failure: EsiFailureSummary | null;
    now: number;
  } = $props();

  let detail = $state<EsiFailureDetail | null>(null);
  let loading = $state(false);

  $effect(() => {
    const summary = failure;
    if (summary === null) {
      detail = null;
      return;
    }
    loading = true;
    void (async () => {
      const response = await fetch(`/api/admin/esi-failures/${summary.id}`);
      detail = response.ok ? await response.json() : null;
      loading = false;
    })();
  });

  const shown = $derived(detail ?? failure);
  const responseBody = $derived(formatBody(detail?.response_body ?? null));
  const requestBody = $derived(formatBody(detail?.request_body ?? null));
  const responseNote = $derived(
    truncationNote(detail?.response_body ?? null, detail?.response_bytes ?? null),
  );
  const headers = $derived(Object.entries(detail?.response_headers ?? {}));
  /** Promoted out of the headers: this is what explains a 420 storm. */
  const errorBudget = $derived(detail?.response_headers?.['x-esi-error-limit-remain'] ?? null);
  const budgetResets = $derived(detail?.response_headers?.['x-esi-error-limit-reset'] ?? null);
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
          <span>{shown.endpoint}</span>
          <span
            class="rounded-full border border-border px-2 py-0.5 text-xs {shown.status === null
              ? 'text-[#fab219]'
              : shown.status >= 500
                ? 'text-negative'
                : 'text-[#ec835a]'}"
          >
            {failureLabel(shown)}
          </span>
        </Dialog.Title>
        <Dialog.Description>
          {relativeTime(failureAt(shown) - now)} · {shown.occurred_at} · {t(
            'admin.esiFailures.took',
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
          <span class="hud-label">{t('admin.esiFailures.request')}</span>
          <code class="rounded bg-card-2 px-2 py-1.5 font-mono text-xs break-all">
            {shown.url}
          </code>
        </div>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="hud-panel px-3 py-2">
            <div class="hud-label">{t('admin.esiFailures.calledBy')}</div>
            <div class="truncate text-sm">
              {#if jobName(shown)}
                <a class="text-primary hover:underline" href="/admin/jobs">
                  {callerLabel(shown)}
                </a>
              {:else}
                {callerLabel(shown) ?? '—'}
              {/if}
            </div>
          </div>
          <div class="hud-panel px-3 py-2">
            <div class="hud-label">{t('admin.esiFailures.schedulerRun')}</div>
            <div class="text-sm tabular-nums">{detail?.scheduler_run_id ?? '—'}</div>
          </div>
          <div class="hud-panel px-3 py-2">
            <div class="hud-label">{t('admin.esiFailures.tokenSent')}</div>
            <div class="text-sm">
              {shown.authenticated ? t('common.actions.yes') : t('common.actions.no')}
            </div>
          </div>
          <div class="hud-panel px-3 py-2">
            <div class="hud-label">{t('admin.esiFailures.errorBudget')}</div>
            <div class="text-sm tabular-nums">
              {errorBudget ?? '—'}{budgetResets
                ? ` · ${t('admin.esiFailures.budgetResets', { seconds: budgetResets })}`
                : ''}
            </div>
          </div>
        </div>

        {#if loading}
          <p class="text-sm text-muted-foreground">{t('admin.esiFailures.loading')}</p>
        {:else if detail}
          <div class="flex flex-col gap-1">
            <span class="hud-label">
              {t('admin.esiFailures.responseBody')}
              {#if responseNote}
                <span class="ml-2 normal-case">({responseNote})</span>
              {/if}
            </span>
            {#if responseBody}
              <pre
                class="max-h-64 overflow-auto rounded bg-card-2 p-3 font-mono text-xs">{responseBody}</pre>
            {:else}
              <p class="text-sm text-muted-foreground">{t('admin.esiFailures.noBody')}</p>
            {/if}
          </div>

          {#if headers.length > 0}
            <div class="flex flex-col gap-1">
              <span class="hud-label">{t('admin.esiFailures.responseHeaders')}</span>
              <table class="w-full text-xs">
                <tbody>
                  {#each headers as [name, value] (name)}
                    <tr class="border-b border-border/60 last:border-0">
                      <td class="py-1 pr-3 font-mono text-muted-foreground">{name}</td>
                      <td class="py-1 font-mono break-all">{value}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}

          <div class="flex flex-col gap-1">
            <span class="hud-label">{t('admin.esiFailures.requestBody')}</span>
            {#if requestBody}
              <pre
                class="max-h-48 overflow-auto rounded bg-card-2 p-3 font-mono text-xs">{requestBody}</pre>
            {:else}
              <p class="text-sm text-muted-foreground">{t('admin.esiFailures.notCaptured')}</p>
            {/if}
          </div>
        {:else}
          <p class="text-sm text-muted-foreground">{t('admin.esiFailures.pruned')}</p>
        {/if}
      </div>
    {/if}
  </Dialog.Content>
</Dialog.Root>
