<script lang="ts">
  // One schema's fields, as a table. Nested objects are named rather than
  // inlined, so a reader follows a link instead of scrolling a tree.
  import { refName, typeLabel, type Schema } from '$lib/openapi';
  import { t } from '$lib/i18n.svelte';

  let { name, schemas }: { name: string; schemas: Record<string, Schema> } = $props();

  const schema = $derived(schemas[name]);
  const required = $derived(new Set(schema?.required ?? []));
  const fields = $derived(Object.entries(schema?.properties ?? {}));
  /** A oneOf has no fields of its own, only the shapes it may take. */
  const alternatives = $derived(
    (schema?.oneOf ?? []).map((entry) => refName(entry) ?? typeLabel(entry)),
  );
</script>

{#if alternatives.length > 0}
  <p class="text-sm">
    {t('docs.schema.oneOf')}
    {#each alternatives as alternative, index (alternative)}<a
        class="font-mono"
        href="#schema-{alternative}">{alternative}</a
      >{#if index < alternatives.length - 1},
      {/if}{/each}
  </p>
{:else if fields.length > 0}
  <table>
    <thead>
      <tr>
        <th>{t('docs.schema.field')}</th>
        <th>{t('common.labels.type')}</th>
        <th>{t('docs.schema.description')}</th>
      </tr>
    </thead>
    <tbody>
      {#each fields as [field, property] (field)}
        <tr>
          <td class="font-mono text-xs whitespace-nowrap">
            {field}
            {#if required.has(field)}
              <span class="text-negative" title={t('docs.schema.alwaysPresent')}>*</span>
            {/if}
          </td>
          <td class="font-mono text-xs">
            {#if refName(property) || refName(property.items)}
              <a href="#schema-{refName(property) ?? refName(property.items)}">
                {typeLabel(property)}
              </a>
            {:else}
              {typeLabel(property)}
            {/if}
          </td>
          <td class="text-sm">{property.description ?? ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p class="text-sm text-muted-foreground">{t('docs.schema.noFields')}</p>
{/if}
