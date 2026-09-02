// Opening a contract in the EVE client: POST /ui/contract answers JSON
// failures (403 missing scope with a grant_scope_url, 422 validation,
// 502 ESI failure), so the shared helper reads the response instead of
// firing and forgetting, and only toasts success when the window
// actually opened.

import { toast } from 'svelte-sonner';
import { t } from './i18n.svelte';
import { notifyError, notifySuccess } from './toast';

export interface OpenContractFailure {
  message: string;
  grantScopeUrl: string | null;
}

/** The failure toast content for a /ui/contract error response body. */
export function openContractFailure(body: unknown): OpenContractFailure {
  const record = typeof body === 'object' && body !== null ? (body as Record<string, unknown>) : {};
  return {
    message:
      typeof record.message === 'string' && record.message !== ''
        ? record.message
        : t('contracts.openInGame.failedBody'),
    grantScopeUrl: typeof record.grant_scope_url === 'string' ? record.grant_scope_url : null,
  };
}

export async function openContractInGame(contractId: number | undefined | null): Promise<void> {
  if (!contractId) return;
  let response: Response;
  try {
    response = await fetch('/ui/contract', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ contract_id: contractId }),
      redirect: 'manual',
    });
  } catch {
    notifyError(t('contracts.openInGame.failedTitle'), t('contracts.openInGame.requestFailedBody'));
    return;
  }
  if (response.ok) {
    notifySuccess(
      t('contracts.actionsDropdown.openingTitle'),
      t('contracts.actionsDropdown.openingBody', { id: contractId }),
    );
    return;
  }
  const failure = openContractFailure(await response.json().catch(() => null));
  if (failure.grantScopeUrl !== null) {
    const url = failure.grantScopeUrl;
    toast.error(t('contracts.openInGame.failedTitle'), {
      description: failure.message,
      action: {
        label: t('contracts.openInGame.grantScope'),
        onClick: () => window.location.assign(url),
      },
    });
    return;
  }
  notifyError(t('contracts.openInGame.failedTitle'), failure.message);
}
