<script lang="ts">
  // The legacy Tables/Contracts/ContractStatus.vue badge.
  import { Check, CircleQuestionMark, Play, X, type Icon as IconType } from '@lucide/svelte';
  import { Badge, type BadgeVariant } from '$lib/components/ui/badge';
  import { t } from '$lib/i18n.svelte';

  let { status }: { status: string } = $props();

  const config: { label: string; icon: typeof IconType; variant: BadgeVariant } = $derived.by(
    () => {
      switch (status) {
        case 'outstanding':
          return { label: t('contracts.status.outstanding'), icon: Play, variant: 'info' };
        case 'completed':
          return { label: t('contracts.status.completed'), icon: Check, variant: 'positive' };
        case 'failed':
          return { label: t('contracts.status.failed'), icon: X, variant: 'negative' };
        default:
          return {
            label: status || t('common.labels.unknown'),
            icon: CircleQuestionMark,
            variant: 'muted',
          };
      }
    },
  );
</script>

<div class="text-center">
  <Badge variant={config.variant}>
    <config.icon class="h-3" />
    {config.label}
  </Badge>
</div>
