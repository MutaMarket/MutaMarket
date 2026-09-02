import { describe, expect, it } from 'vitest';
import { MAX_TILT_DEG, pointerTilt } from './holo-tilt';

describe('pointerTilt', () => {
  it('is level at the center', () => {
    const frame = pointerTilt(200, 300, 100, 150);
    expect(frame.rotateX).toBe(0);
    expect(frame.rotateY).toBe(0);
    expect(frame.pointerX).toBe(50);
    expect(frame.pointerY).toBe(50);
  });

  it('leans fully toward the corners', () => {
    const topLeft = pointerTilt(200, 300, 0, 0);
    expect(topLeft.rotateX).toBe(MAX_TILT_DEG);
    expect(topLeft.rotateY).toBe(-MAX_TILT_DEG);
    expect(topLeft.pointerX).toBe(0);
    expect(topLeft.pointerY).toBe(0);

    const bottomRight = pointerTilt(200, 300, 200, 300);
    expect(bottomRight.rotateX).toBe(-MAX_TILT_DEG);
    expect(bottomRight.rotateY).toBe(MAX_TILT_DEG);
  });

  it('clamps pointers outside the box', () => {
    const frame = pointerTilt(200, 300, -50, 400);
    expect(frame.pointerX).toBe(0);
    expect(frame.pointerY).toBe(100);
    expect(frame.rotateX).toBe(-MAX_TILT_DEG);
    expect(frame.rotateY).toBe(-MAX_TILT_DEG);
  });
});
