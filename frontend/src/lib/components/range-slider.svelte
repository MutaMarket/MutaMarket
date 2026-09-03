<script lang="ts">
  // The filter range slider, a Svelte port of the legacy
  // vue-3-slider-component setup: 0-100
  // domain, two handles, regular text marks, source-type pips with a
  // hover popover, drag tooltips, and endpoint labels whose
  // endpoint label colors flip (the price slider).
  import type { Snippet } from 'svelte';
  import { clamp } from '$lib/slider-scale';

  export interface SliderMark {
    /** 0-100 track position. */
    position: number;
    kind: 'regular' | 'pip';
    /** Regular marks: the text under the track. */
    label?: string;
    /** Pips: the formatted value shown in the hover popover. */
    formatted?: string;
    /** Pips: the source types sitting at this position. */
    types?: { id: number; name: string; meta_group_id: number | null }[];
  }

  let {
    values = $bindable(),
    labels,
    valueText,
    marks = [],
    oninput,
    onchange,
    tooltip,
  }: {
    values: [number, number];
    /** The accessible names of the lower and upper handle. */
    labels: [string, string];
    /** Reads a handle position as the formatted value screen readers announce. */
    valueText?: (position: number) => string;
    marks?: SliderMark[];
    /** Fires on every handle move (the caller debounces the search). */
    oninput?: (values: [number, number]) => void;
    /** Fires once when a drag ends. */
    onchange?: (values: [number, number]) => void;
    /** Renders inside the drag tooltip; receives the handle position. */
    tooltip?: Snippet<[number]>;
  } = $props();

  /** The legacy slider interval. */
  const STEP = 0.01;

  /** Drag positions within this many slider units of a source-type pip
   * snap onto it, so pip clicks and near-pip drags land exactly. */
  const PIP_SNAP_RANGE = 1.5;

  let track: HTMLDivElement | null = $state(null);
  let dragging: 0 | 1 | null = $state(null);

  const pipPositions = $derived(
    marks.filter((mark) => mark.kind === 'pip').map((mark) => mark.position),
  );

  function snapped(position: number): number {
    let best = position;
    let bestDistance = PIP_SNAP_RANGE;
    for (const pip of pipPositions) {
      const distance = Math.abs(pip - position);
      if (distance <= bestDistance) {
        best = pip;
        bestDistance = distance;
      }
    }
    return best;
  }

  const low = $derived(Math.min(values[0], values[1]));
  const high = $derived(Math.max(values[0], values[1]));

  function positionFromEvent(event: PointerEvent): number {
    if (!track) {
      return 0;
    }
    const rect = track.getBoundingClientRect();
    const fraction = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    return Math.round((fraction * 100) / STEP) * STEP;
  }

  function moveHandle(handle: 0 | 1, position: number) {
    const next: [number, number] = [...values];
    next[handle] = clamp(position, 0, 100);
    values = next;
    oninput?.(next);
  }

  function startDrag(handle: 0 | 1, event: PointerEvent) {
    event.preventDefault();
    dragging = handle;
    moveHandle(handle, snapped(positionFromEvent(event)));
  }

  // Window-level listeners: the drag must follow and release outside
  // the track (element-level pointer capture dies when the captured
  // child re-renders mid-drag).
  function onWindowMove(event: PointerEvent) {
    if (dragging !== null) {
      moveHandle(dragging, snapped(positionFromEvent(event)));
    }
  }

  function endDrag() {
    if (dragging !== null) {
      dragging = null;
      onchange?.(values);
    }
  }

  function onTrackDown(event: PointerEvent) {
    // Clicking the track grabs the nearest handle, like the legacy
    // slider; pip clicks bubble here and snap exactly onto the pip.
    const position = snapped(positionFromEvent(event));
    const nearest: 0 | 1 = Math.abs(position - values[0]) <= Math.abs(position - values[1]) ? 0 : 1;
    startDrag(nearest, event);
  }

  function onHandleKey(handle: 0 | 1, event: KeyboardEvent) {
    const step = event.shiftKey ? 10 : 1;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') {
      event.preventDefault();
      moveHandle(handle, values[handle] - step);
    } else if (event.key === 'ArrowRight' || event.key === 'ArrowUp') {
      event.preventDefault();
      moveHandle(handle, values[handle] + step);
    }
  }

  function endpointColor(position: number): string {
    // Every legacy slider carried the .reversed CSS variant: the left
    // end (worst roll / lowest ISK) is red, the right end green.
    if (position === 0) {
      return 'text-red-500';
    }
    if (position === 100) {
      return 'text-green-500';
    }
    return 'text-muted-foreground';
  }

  function metaDot(metaGroupId: number | null): string {
    switch (metaGroupId) {
      case 2:
        return 'bg-orange-500';
      case 3:
        return 'bg-green-300';
      case 4:
        return 'bg-green-500';
      case 5:
        return 'bg-purple-500';
      case 6:
        return 'bg-blue-500';
      default:
        return 'bg-gray-500';
    }
  }
</script>

<svelte:window onpointermove={onWindowMove} onpointerup={endDrag} onpointercancel={endDrag} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative pt-3 pb-5 select-none">
  <div
    bind:this={track}
    class="relative h-1 cursor-pointer rounded-full bg-muted"
    onpointerdown={onTrackDown}
  >
    <!-- The filled range between the handles. -->
    <div
      class="absolute h-1 rounded-full bg-primary"
      style="left: {low}%; width: {high - low}%"
    ></div>

    {#each marks as mark (mark.kind + mark.position)}
      {#if mark.kind === 'pip'}
        {@const active = mark.position >= low && mark.position <= high}
        <div
          class="group absolute top-1/2 z-20 size-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-popover ring-2 {active
            ? 'ring-primary'
            : 'ring-muted'}"
          style="left: {mark.position}%"
        >
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="absolute bottom-4 left-1/2 z-50 hidden -translate-x-1/2 rounded-lg border bg-popover p-4 group-hover:block"
            onpointerdown={(event) => event.stopPropagation()}
          >
            <span class="text-xs leading-none font-medium text-foreground uppercase">
              {mark.formatted}
            </span>
            {#each mark.types ?? [] as markType (markType.id)}
              <div class="flex items-center gap-1.5 text-xs whitespace-nowrap">
                <span class="size-1.5 shrink-0 rounded-full {metaDot(markType.meta_group_id)}"
                ></span>
                {markType.name}
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div
          class="pointer-events-none absolute top-3 z-10 -translate-x-1/2 text-[0.625rem] font-medium {endpointColor(
            mark.position,
          )}"
          style="left: {mark.position}%"
        >
          {mark.label}
        </div>
      {/if}
    {/each}

    {#each [0, 1] as handle (handle)}
      <button
        type="button"
        role="slider"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={values[handle as 0 | 1]}
        aria-valuetext={valueText?.(values[handle as 0 | 1])}
        aria-label={labels[handle as 0 | 1]}
        class="absolute top-1/2 z-30 size-3.5 -translate-x-1/2 -translate-y-1/2 cursor-grab rounded-full bg-primary shadow ring-2 ring-background focus-visible:ring-ring active:cursor-grabbing"
        style="left: {values[handle as 0 | 1]}%"
        onpointerdown={(event) => startDrag(handle as 0 | 1, event)}
        onkeydown={(event) => onHandleKey(handle as 0 | 1, event)}
      >
        {#if dragging === handle && tooltip}
          <div class="absolute bottom-5 left-1/2 z-50 -translate-x-1/2 whitespace-nowrap">
            {@render tooltip(values[handle as 0 | 1])}
          </div>
        {/if}
      </button>
    {/each}
  </div>
</div>
