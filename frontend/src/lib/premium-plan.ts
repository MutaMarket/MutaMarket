import { refreshAll } from '$app/navigation';
import { t } from '#lib/i18n.svelte.ts';
import { notifyError, notifySuccess } from '#lib/toast.ts';

/** Puts the account on the ad-free or the plain plan (`POST
 * /premium/plan`; every linked character converts its time), toasts the
 * outcome and reloads the page data so the nav, the sidebar and the ads
 * follow. True on success. */
export async function switchPremiumPlan(adFree: boolean): Promise<boolean> {
  const response = await fetch('/premium/plan', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ ad_free: adFree }),
  });
  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    notifyError(
      t('premium.show.adFree.failedTitle'),
      body.message ?? t('errors.internalServerError.name'),
    );
    return false;
  }
  notifySuccess(
    t('premium.show.adFree.switchedTitle'),
    t(body.ad_free ? 'premium.show.adFree.switchedAdFree' : 'premium.show.adFree.switchedPremium'),
  );
  await refreshAll();
  return true;
}
