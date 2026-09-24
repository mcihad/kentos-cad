import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * 2D affine transform as [a, b, c, d, e, f]:
 *   x' = a·x + c·y + e
 *   y' = b·x + d·y + f
 * All CAD modify operations (move, rotate, scale, mirror, array) are
 * expressed as one of these so every entity kind needs one code path.
 * Computed by the geometry core (docs/adr/0008).
 */
export type Affine = readonly [number, number, number, number, number, number];

export const IDENTITY: Affine = [1, 0, 0, 1, 0, 0];

export const translation = op<(dx: number, dy: number) => Affine>('translation');

/** Rotation by `angle` radians (CCW) around `o` (the origin when left out). */
export const rotation = op<(angle: number, o?: Vec2) => Affine>('rotation');

/** Uniform scale around `o` (the origin when left out). */
export const scaling = op<(s: number, o?: Vec2) => Affine>('scaling');

/** Reflection across the line through `p` and `q`. */
export const mirror = op<(p: Vec2, q: Vec2) => Affine>('mirror');

/** m2 ∘ m1: first m1, then m2. */
export const compose = op<(m2: Affine, m1: Affine) => Affine>('compose');

export const apply = op<(m: Affine, p: Vec2) => Vec2>('apply');

/** Applies only the linear part (for direction vectors). */
export const applyLinear = op<(m: Affine, v: Vec2) => Vec2>('applyLinear');

export const determinant = op<(m: Affine) => number>('determinant');

/** Length scale factor for similarity transforms (rotation/uniform scale/mirror). */
export const lengthScale = op<(m: Affine) => number>('lengthScale');

/** True when the transform flips orientation (mirror). */
export const isReflection = op<(m: Affine) => boolean>('isReflection');
