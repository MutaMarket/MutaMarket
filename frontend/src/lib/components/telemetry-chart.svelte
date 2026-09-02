<script lang="ts">
  // A stacked-column minute chart for the admin telemetry: one column
  // per minute over the window, stacked by series, with a grouped hover
  // tooltip. The props API is unchanged from the LayerChart version it
  // replaced; only the rendering moved. LayerChart mounted a component
  // per rect, which cost ~15ms per band and made a 60-minute window
  // block the main thread for most of a second on every redraw.
  import { barY, defineChart, stack } from '@tanstack/charts';
  import { scaleBand } from '@tanstack/charts/scales/band';
  import { scaleLinear } from '@tanstack/charts/scales/linear';
  import { Chart } from '@tanstack/charts/svelte';
  import { tooltip } from '@tanstack/charts/tooltip';
  import { t } from '$lib/i18n.svelte';

  export interface ChartSeries {
    key: string;
    label: string;
    color: string;
  }

  export interface ChartMinute {
    /** Unix seconds of the minute's start. */
    minuteStart: number;
    /** Value per series key. */
    values: Record<string, number>;
    /** Extra tooltip line (e.g. average latency), if any. */
    detail?: string;
  }

  let {
    title,
    series,
    minutes,
    emptyText,
    headline,
    headlineClass = 'text-foreground',
    sub,
    onSelect,
    formatLabel,
    tickAt,
  }: {
    title: string;
    series: ChartSeries[];
    minutes: ChartMinute[];
    emptyText: string;
    /** The hour total shown as the card's stat. */
    headline?: string;
    headlineClass?: string;
    sub?: string;
    /** Clicking a column reports its minute, for a drill-down. */
    onSelect?: (minuteStart: number) => void;
    /** Axis label; defaults to the HH:MM the ESI charts want. */
    formatLabel?: (bucketStart: number) => string;
    /** Which buckets get a tick. An index predicate rather than a
     * seconds modulus, because months are not evenly spaced. */
    tickAt?: (bucketStart: number, index: number) => boolean;
  } = $props();

  /** Plot height, matching the vitals cards' rhythm. */
  const HEIGHT = 180;
  /** Axis ticks every 15 minutes, aligned to the wall clock. */
  const TICK_SECONDS = 900;

  const hasData = $derived(
    minutes.some((minute) => series.some((s) => (minute.values[s.key] ?? 0) > 0)),
  );

  // One row per series per minute: the grammar takes long data and
  // derives the stack, where the predecessor took one wide row.
  const rows = $derived(
    minutes.flatMap((minute) =>
      series.map((s) => ({
        minuteStart: minute.minuteStart,
        series: s.key,
        label: s.label,
        value: minute.values[s.key] ?? 0,
        detail: minute.detail ?? null,
      })),
    ),
  );

  function defaultLabel(minuteStart: number): string {
    const date = new Date(minuteStart * 1000);
    const hh = String(date.getUTCHours()).padStart(2, '0');
    const mm = String(date.getUTCMinutes()).padStart(2, '0');
    return `${hh}:${mm}`;
  }

  const timeLabel = $derived(formatLabel ?? defaultLabel);
  const ticked = $derived(tickAt ?? ((start: number) => start % TICK_SECONDS === 0));

  const definition = $derived(
    defineChart({
      marks: [
        barY(rows, {
          x: 'minuteStart',
          y: 'value',
          color: 'series',
          // Explicit order: the series arrive in the stack order the
          // page chose, and the color adjacency depends on it.
          layout: stack({ order: series.map((s) => s.key) }),
        }),
      ],
      scales: {
        x: {
          scale: () =>
            scaleBand<number>()
              .domain(minutes.map((minute) => minute.minuteStart))
              .padding(0.25),
          axis: {
            ticks: {
              values: minutes
                .map((minute) => minute.minuteStart)
                .filter((start, index) => ticked(start, index)),
              format: timeLabel,
            },
          },
        },
        y: {
          scale: scaleLinear,
          nice: true,
          grid: true,
          axis: { ticks: { format: (value: number) => value.toLocaleString('en-US') } },
        },
      },
      color: {
        domain: series.map((s) => s.key),
        range: series.map((s) => s.color),
      },
      focus: 'group-x',
      tooltip: {
        use: tooltip,
        formatGroup(points) {
          const first = points[0];
          const heading = t('admin.console.eveTime', {
            time: timeLabel(Number(first?.xValue ?? 0)),
          });
          const detail = first?.datum.detail;
          return [
            detail ? `${heading} · ${detail}` : heading,
            ...points
              .filter((point) => point.datum.value > 0)
              .map((point) => `${point.datum.label}: ${point.datum.value.toLocaleString('en-US')}`),
          ].join('\n');
        },
      },
    }),
  );
</script>

<div class="hud-frame p-4">
  <div class="mb-2 flex flex-wrap items-start justify-between gap-x-3 gap-y-1">
    <div class="min-w-0">
      <h2 class="hud-label whitespace-nowrap">{title}</h2>
      {#if headline}
        <div class="mt-1 truncate text-lg font-semibold tabular-nums {headlineClass}">
          {headline}
        </div>
      {/if}
      {#if sub}
        <div class="truncate text-xs text-muted-foreground">{sub}</div>
      {/if}
    </div>
  </div>

  {#if !hasData}
    <div class="grid h-[180px] place-items-center text-xs text-muted-foreground">
      {emptyText}
    </div>
  {:else}
    <Chart
      {definition}
      ariaLabel={title}
      height={HEIGHT}
      onSelect={onSelect
        ? (point) => {
            if (point) onSelect(Number(point.xValue));
          }
        : undefined}
    />
  {/if}
</div>
