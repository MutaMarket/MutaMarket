import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import EsiFailureDialog from './esi-failure-dialog.svelte';
import type { EsiFailureDetail, EsiFailureSummary } from '$lib/admin-types';
import { t } from '$lib/i18n.svelte';

function summary(overrides: Partial<EsiFailureSummary> = {}): EsiFailureSummary {
  return {
    id: 7,
    occurred_at: '2026-08-30 14:04:10.757061+00',
    endpoint: 'contracts/public',
    method: 'GET',
    url: 'https://esi.evetech.net/latest/contracts/public/10000002/?page=3',
    status: 500,
    error_kind: null,
    error_message: 'Internal error',
    duration_ms: 120,
    authenticated: false,
    caller: 'job:region-contracts',
    ...overrides,
  };
}

function detail(overrides: Partial<EsiFailureDetail> = {}): EsiFailureDetail {
  return {
    ...summary(),
    scheduler_run_id: 42,
    response_headers: {
      'content-type': 'application/json',
      'x-esi-error-limit-remain': '17',
      'x-esi-error-limit-reset': '38',
    },
    response_body: '{"error":"Internal error"}',
    response_bytes: 26,
    request_body: null,
    request_bytes: null,
    ...overrides,
  };
}

function respond(body: EsiFailureDetail) {
  vi.stubGlobal(
    'fetch',
    vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve(body) } as Response)),
  );
}

/** The dialog's rendered text, whitespace collapsed. */
function text(): string {
  const dialog = document.querySelector('[role="dialog"]');
  return (dialog?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

async function settle() {
  await new Promise((resolve) => setTimeout(resolve, 250));
}

describe('esi-failure-dialog', () => {
  it('leads with the verdict, ESI message and the exact request', async () => {
    respond(detail());
    await render(EsiFailureDialog, { failure: summary(), now: 1_787_000_000 });
    await settle();

    const rendered = text();
    expect(rendered).toContain('GET');
    expect(rendered).toContain('contracts/public');
    expect(rendered).toContain('500');
    expect(rendered).toContain('Internal error');
    // The query params are the second thing anyone asks about.
    expect(rendered).toContain('?page=3');
  });

  it('promotes the error budget out of the headers', async () => {
    respond(detail());
    await render(EsiFailureDialog, { failure: summary(), now: 1_787_000_000 });
    await settle();

    // The number that explains a 420 storm should not be buried.
    expect(text()).toContain('17');
    expect(text()).toContain(t('admin.esiFailures.budgetResets', { seconds: 38 }));
  });

  it('says a request body was not captured, and why', async () => {
    respond(detail());
    await render(EsiFailureDialog, { failure: summary(), now: 1_787_000_000 });
    await settle();

    expect(text()).toContain(t('admin.esiFailures.notCaptured'));
  });

  it('reports whether a token was sent, never which', async () => {
    respond(detail({ authenticated: true }));
    await render(EsiFailureDialog, {
      failure: summary({ authenticated: true }),
      now: 1_787_000_000,
    });
    await settle();

    expect(text()).toContain(t('admin.esiFailures.tokenSent'));
    expect(text()).toContain('Yes');
  });

  it('explains a transport failure that has no body at all', async () => {
    const transport = summary({ status: null, error_kind: 'timeout', error_message: 'timed out' });
    respond(
      detail({
        ...transport,
        response_headers: null,
        response_body: null,
        response_bytes: null,
      }),
    );
    await render(EsiFailureDialog, { failure: transport, now: 1_787_000_000 });
    await settle();

    expect(text()).toContain(`${t('admin.telemetry.series.noResponse')} · timeout`);
    expect(text()).toContain(t('admin.esiFailures.noBody'));
  });

  it('renders nothing while no failure is selected', async () => {
    respond(detail());
    await render(EsiFailureDialog, { failure: null, now: 1_787_000_000 });

    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });
});
