<script lang="ts">
  // An ISK amount field, the legacy Forms/CurrencyInput.vue: the number
  // itself is right-aligned and the short compact form of what is typed
  // floats over the free space on its left, inert so clicks anywhere
  // land in the field. The "ISK" tag rides along where nothing else
  // names the unit; the asking-price row has a coin icon instead, which
  // is how legacy's AskingPrice.vue leaves it off.
  //
  // `m` and `b` multiply the field by a million or a billion, the
  // legacy shortcut for typing large prices.
  import { Input } from '$lib/components/ui/input';
  import { toVeryCompact } from '$lib/format-number';
  import { t } from '$lib/i18n.svelte';

  let {
    value,
    label,
    onchange,
    onblur,
    onenter,
    empty = '',
    unit = true,
    max = null,
    class: className = '',
  }: {
    value: string;
    /** Accessible name; the field carries no visible label. */
    label: string;
    onchange: (value: string) => void;
    onblur?: () => void;
    onenter?: () => void;
    /** What the compact slot reads while the field is empty. */
    empty?: string;
    /** Show the "ISK" tag. Off where an icon already names the unit. */
    unit?: boolean;
    /** Ceiling the value may not pass; the field flags it instead. */
    max?: number | null;
    class?: string;
  } = $props();

  const amount = $derived(Number(value));
  const tooHigh = $derived(max !== null && Number.isFinite(amount) && amount > max);
  const compact = $derived.by(() => {
    if (tooHigh) {
      return t('forms.currencyInput.max', { value: toVeryCompact(max ?? 0) });
    }
    return value.trim() === '' || !Number.isFinite(amount) ? empty : toVeryCompact(amount);
  });

  function onkeydown(event: KeyboardEvent) {
    const multiplier = event.key === 'm' ? 1_000_000 : event.key === 'b' ? 1_000_000_000 : null;
    if (multiplier !== null) {
      event.preventDefault();
      onchange(String(Number(value) * multiplier));
      return;
    }
    if (event.key === 'Enter') {
      onenter?.();
    }
  }
</script>

<div class="isolate grid grid-cols-[auto_1fr] items-center focus-within:z-50">
  <Input
    type="number"
    aria-label={label}
    aria-invalid={tooHigh}
    max={max ?? undefined}
    {value}
    class="col-span-full col-start-1 row-start-1 h-8 w-full min-w-0 {unit
      ? 'pl-16'
      : 'pl-11'} text-right text-xs [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none {className}"
    oninput={(event) => onchange(event.currentTarget.value)}
    {onblur}
    {onkeydown}
  />
  <!-- Both readouts sit on top of the field and are inert, so a click
	     anywhere across the input still lands in it. -->
  <div
    class="pointer-events-none col-start-1 row-start-1 flex items-center gap-2 pl-2 text-xs {tooHigh
      ? 'text-destructive'
      : 'text-muted-foreground'}"
  >
    {#if unit}
      <span>ISK</span>
    {/if}
    <span class="truncate">{compact}</span>
  </div>
</div>
