<script lang="ts">
  // The legacy Tables/Contracts/ContractStatus.vue badge.
  import { Check, CircleQuestionMark, Play, X, type Icon as IconType } from '@lucide/svelte';
  import { Badge, type BadgeVariant } from '$lib/components/ui/badge';

  let { status }: { status: string } = $props();

  const config: { label: string; icon: typeof IconType; variant: BadgeVariant } = $derived.by(
    () => {
      switch (status) {
        case 'outstanding':
          return { label: 'Outstanding', icon: Play, variant: 'info' };
        case 'completed':
          return { label: 'Completed', icon: Check, variant: 'positive' };
        case 'failed':
          return { label: 'Failed', icon: X, variant: 'negative' };
        default:
          return { label: status || 'Unknown', icon: CircleQuestionMark, variant: 'muted' };
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
