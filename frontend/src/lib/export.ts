// Module export formats and clipboard actions, ported from the legacy
// Helper/Export.ts + Composables/useExport.ts (toast copy included).

import { toIsk } from './format-number';
import { t } from './i18n.svelte';
import { notifyError, notifySuccess } from './toast';
import type { ModuleDetail } from './types';

/** The Jita solar system id EVE contract links are scoped to. */
const CONTRACT_LINK_SYSTEM = 30000142;

export function toPyfa(module: ModuleDetail): string {
  const attributes = module.mutated_attributes
    .map((attribute) => `${attribute.name} ${attribute.value}`)
    .join(', ');

  return `${module.source_type?.name ?? module.type.name}\n${module.mutaplasmid?.name ?? ''}\n${attributes}`;
}

export function toItemLink(module: ModuleDetail): string {
  return `<url=showinfo:${module.type.id}//${module.id}>${module.type.name} (${module.id})</url>`;
}

export function toContractLink(module: ModuleDetail): string {
  return `<url=contract:${CONTRACT_LINK_SYSTEM}//${module.contract?.id}>Contract ${module.contract?.id} (${module.type.name}) ${toIsk(module.contract?.price ?? 0)}</url>`;
}

/** The legacy toContractLink branch for a bare contract resource (the
 * moderator review page): the contract's own id and price. */
export function toHistoricContractLink(contract: { id: number; price: number | null }): string {
  return `<url=contract:${CONTRACT_LINK_SYSTEM}//${contract.id}>Contract ${contract.id} ${toIsk(contract.price ?? 0)}</url>`;
}

function capitalise(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

/** `what` is the translated noun in its mid-sentence form ("item link");
 * the title capitalises it, the body reads it as is. */
export async function copyWithToasts(text: string, what: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    notifySuccess(
      t('modules.export.copiedTitle', { what: capitalise(what) }),
      t('modules.export.copiedBody', { what }),
    );
  } catch {
    notifyError(
      t('modules.export.copyFailedTitle', { what }),
      t('modules.export.copyFailedBody', { what }),
    );
  }
}

export const copyPyfa = (module: ModuleDetail) =>
  copyWithToasts(toPyfa(module), t('modules.export.subjects.module'));
export const copyItemLink = (module: ModuleDetail) =>
  copyWithToasts(toItemLink(module), t('modules.export.subjects.itemLink'));
export const copyContractLink = (module: ModuleDetail) =>
  copyWithToasts(toContractLink(module), t('modules.export.subjects.contractLink'));
export const copyPageLink = (module: ModuleDetail) =>
  copyWithToasts(
    `${location.origin}/modules/${module.slug}`,
    t('modules.export.subjects.pageLink'),
  );
export const copyImageLink = (module: ModuleDetail) =>
  copyWithToasts(
    `${location.origin}/og/module/${module.id}.png`,
    t('modules.export.subjects.imageLink'),
  );

/** Share via the native sheet, falling back to the page-link copy. */
export async function shareModule(module: ModuleDetail): Promise<void> {
  if (!navigator.share) {
    return copyPageLink(module);
  }
  try {
    await navigator.share({
      title: t('meta.modulesShow.title', {
        creator: module.creator?.name ?? t('common.labels.unknown'),
        type: module.type.name,
      }),
      url: `${location.origin}/modules/${module.slug}`,
    });
  } catch (error) {
    if ((error as DOMException).name !== 'AbortError') {
      await copyPageLink(module);
    }
  }
}
