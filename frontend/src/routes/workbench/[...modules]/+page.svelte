<script lang="ts">
  // The shared-workbench invitation, the legacy
  // ShowWorkbenchInvitationPage: the shared modules as cards, with the
  // accept action importing them into the visitor's own workbench.
  import { FlaskConical } from '@lucide/svelte';
  import { page } from '$app/state';
  import ModuleCard from '$lib/components/module-card.svelte';
  import PageHeader from '$lib/components/page-header.svelte';
  import { Button } from '$lib/components/ui/button';
  import { defaultDisplaySettings } from '$lib/display';
  import { t } from '$lib/i18n.svelte';
  import { notifySuccess } from '$lib/toast';
  import { refreshWorkbench, workbenchOpen } from '$lib/workbench';
  import type { PageProps } from './$types';
  import PageMeta from '$lib/components/page-meta.svelte';

  let { data }: PageProps = $props();

  const signedIn = $derived(Boolean(page.data.nav?.user));
  let accepting = $state(false);

  async function accept() {
    accepting = true;
    try {
      await fetch(`/workbench/${data.shared}`, { method: 'POST', redirect: 'manual' });
      notifySuccess(
        t('admin.workbench.modulesAddedTitle'),
        t('admin.workbench.modulesAddedBody', { count: data.modules.length }),
      );
      await refreshWorkbench();
      workbenchOpen.set(true);
    } finally {
      accepting = false;
    }
  }
</script>

<PageMeta
  title={t('meta.workbenchInvitation.title')}
  description={t('meta.workbenchInvitation.description', { count: data.modules.length })}
/>

<PageHeader
  title={t('admin.workbench.invitationTitle')}
  subtitle={t('admin.workbench.invitationSubtitle', { count: data.modules.length })}
>
  {#snippet icon()}
    <div class="grid size-10 place-items-center rounded-lg border border-border bg-card-1">
      <FlaskConical class="size-5 text-primary" stroke-width={1.5} />
    </div>
  {/snippet}
  {#snippet actions()}
    {#if signedIn}
      <Button class="h-8" disabled={accepting || data.modules.length === 0} onclick={accept}>
        {t('admin.workbench.addModulesButton')}
      </Button>
    {:else}
      <Button class="h-8" href="/login">{t('admin.workbench.loginToAdd')}</Button>
    {/if}
  {/snippet}
</PageHeader>

{#if data.modules.length === 0}
  <div class="hud-frame p-6">
    <span class="block text-lg font-medium">{t('admin.workbench.emptyInvitationTitle')}</span>
    <p class="text-muted-foreground">{t('admin.workbench.emptyInvitationBody')}</p>
  </div>
{:else}
  <div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
    {#each data.modules as module (module.id)}
      <ModuleCard {module} settings={defaultDisplaySettings()} />
    {/each}
  </div>
{/if}
