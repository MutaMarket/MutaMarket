<script lang="ts">
	// A two-thumb range slider over 0..100 in steps of 1, replacing the
	// legacy vue-slider-component. Committing happens on pointer release,
	// so a drag causes one navigation, not many.
	let {
		values = $bindable(),
		onCommit
	}: { values: [number, number]; onCommit: (values: [number, number]) => void } = $props();

	let track: HTMLDivElement | undefined = $state();
	let dragging: 'lower' | 'upper' | null = null;

	function valueAt(clientX: number): number {
		if (!track) return 0;
		const rect = track.getBoundingClientRect();
		const width = Math.max(rect.width, 1);
		return Math.round(Math.min(Math.max(((clientX - rect.left) / width) * 100, 0), 100));
	}

	function moveThumb(thumb: 'lower' | 'upper', value: number) {
		if (thumb === 'lower') {
			values = [Math.min(value, values[1]), values[1]];
		} else {
			values = [values[0], Math.max(value, values[0])];
		}
	}

	function onPointerDown(event: PointerEvent) {
		const value = valueAt(event.clientX);
		const [lower, upper] = values;

		// The nearest thumb moves; when both sit together, going left grabs
		// the lower thumb and going right the upper.
		let thumb: 'lower' | 'upper';
		if (Math.abs(value - lower) < Math.abs(value - upper)) {
			thumb = 'lower';
		} else if (Math.abs(value - lower) > Math.abs(value - upper)) {
			thumb = 'upper';
		} else {
			thumb = value < lower ? 'lower' : 'upper';
		}

		track?.setPointerCapture(event.pointerId);
		dragging = thumb;
		moveThumb(thumb, value);
	}

	function onPointerMove(event: PointerEvent) {
		if (dragging !== null) {
			moveThumb(dragging, valueAt(event.clientX));
		}
	}

	function onPointerUp() {
		if (dragging !== null) {
			dragging = null;
			onCommit(values);
		}
	}
</script>

<div
	class="cursor-pointer touch-none select-none py-2"
	bind:this={track}
	role="slider"
	aria-label="Attribute range"
	aria-valuemin={0}
	aria-valuemax={100}
	aria-valuenow={values[0]}
	aria-valuetext="{values[0]} to {values[1]}"
	tabindex="0"
	onpointerdown={onPointerDown}
	onpointermove={onPointerMove}
	onpointerup={onPointerUp}
	onpointercancel={onPointerUp}
>
	<div class="relative h-1 rounded bg-card-2">
		<div
			class="absolute h-full rounded bg-primary"
			style="left: {values[0]}%; width: {values[1] - values[0]}%"
		></div>
		<div
			class="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-border bg-white"
			style="left: {values[0]}%"
		></div>
		<div
			class="absolute top-1/2 size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-border bg-white"
			style="left: {values[1]}%"
		></div>
	</div>
</div>
