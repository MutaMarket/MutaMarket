// Pointer-driven 3D tilt for premium holo cards: the card leans toward
// the cursor while CSS variables carry the pointer position, so the
// foil glare and sheen in layout.css can track it. Original
// implementation of the well-known tilt-card technique.
import type { Action } from 'svelte/action';

/** Maximum lean toward the pointer at a card's edge. */
export const MAX_TILT_DEG = 7;

export interface TiltFrame {
	/** Lean around the horizontal axis, degrees. */
	rotateX: number;
	/** Lean around the vertical axis, degrees. */
	rotateY: number;
	/** Pointer position across the card, percent. */
	pointerX: number;
	pointerY: number;
}

function clamp(value: number, low: number, high: number): number {
	return Math.min(Math.max(value, low), high);
}

/**
 * Maps a pointer offset inside a width×height box to the tilt frame:
 * zero at the center, `MAX_TILT_DEG` at the edges, positions clamped
 * into the box.
 */
export function pointerTilt(width: number, height: number, x: number, y: number): TiltFrame {
	const px = clamp(x / width, 0, 1);
	const py = clamp(y / height, 0, 1);
	return {
		rotateX: (0.5 - py) * 2 * MAX_TILT_DEG,
		rotateY: (px - 0.5) * 2 * MAX_TILT_DEG,
		pointerX: px * 100,
		pointerY: py * 100
	};
}

/**
 * `use:holoTilt={enabled}` — writes `--tilt-x/--tilt-y` (degrees) and
 * `--pointer-x/--pointer-y` (percent) while the pointer hovers the
 * element, and clears them on leave so the CSS defaults ease the card
 * back. Inert when disabled or when the user prefers reduced motion.
 */
export const holoTilt: Action<HTMLElement, boolean> = (node, enabled) => {
	let active = false;

	function move(event: PointerEvent) {
		const rect = node.getBoundingClientRect();
		const frame = pointerTilt(
			rect.width,
			rect.height,
			event.clientX - rect.left,
			event.clientY - rect.top
		);
		node.style.setProperty('--tilt-x', `${frame.rotateX.toFixed(2)}deg`);
		node.style.setProperty('--tilt-y', `${frame.rotateY.toFixed(2)}deg`);
		node.style.setProperty('--pointer-x', `${frame.pointerX.toFixed(1)}%`);
		node.style.setProperty('--pointer-y', `${frame.pointerY.toFixed(1)}%`);
	}

	function leave() {
		for (const name of ['--tilt-x', '--tilt-y', '--pointer-x', '--pointer-y']) {
			node.style.removeProperty(name);
		}
	}

	function apply(on: boolean | undefined) {
		const wanted =
			on === true && !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		if (wanted === active) return;
		active = wanted;
		if (wanted) {
			node.addEventListener('pointermove', move);
			node.addEventListener('pointerleave', leave);
		} else {
			node.removeEventListener('pointermove', move);
			node.removeEventListener('pointerleave', leave);
			leave();
		}
	}

	apply(enabled);
	return {
		update: apply,
		destroy: () => apply(false)
	};
};
