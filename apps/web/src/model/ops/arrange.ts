import type { ArrangeMode } from '../../contracts/generated/ArrangeMode';
import { op } from '../../wasm/core';
import type { Entity } from '../entities';
import type { Bounds, Vec2 } from '../geometry';

/**
 * Hizala ve dağıt (docs/adr/0194 §2): the rules are the geometry core's (`ops::arrange`), as on the desktop. The
 * tool previews with them and `cad.entities.transform`'s `arrange` writes with them.
 */

/** The methods in the tool's order. */
export const ARRANGE_MODES: readonly ArrangeMode[] = ['left', 'center', 'right', 'top', 'middle', 'bottom', 'horizontal', 'vertical'];

/** Each method's name, the undo step's too. */
export const ARRANGE_LABELS: Record<ArrangeMode, string> = {
  left: 'Sola hizala',
  center: 'Ortala',
  right: 'Sağa hizala',
  top: 'Üste hizala',
  middle: 'Ortaya hizala',
  bottom: 'Alta hizala',
  horizontal: 'Yatay dağıt',
  vertical: 'Dikey dağıt',
};

/** Whether the mode meets a reference (the six alignments) rather than spreads. */
export const aligns = (mode: ArrangeMode) => mode !== 'horizontal' && mode !== 'vertical';

/** Whether the mode moves objects east and west (else north and south). */
export const eastward = (mode: ArrangeMode) => mode === 'left' || mode === 'center' || mode === 'right' || mode === 'horizontal';

/** The objects' boxes as the drawing measures them: an insert with its block's pieces (`blocks`, the contract's list), a text in the drawing's typeface. */
export const arrangeBoxes = op<(objects: readonly Entity[], blocks: readonly unknown[] | null, font: string | null) => Bounds[]>('arrangeBoxes');

/** A reference box's side or middle the mode meets (`at`). */
export const arrangeAt = op<(box: Bounds, mode: ArrangeMode) => number>('arrangeAt');

/** The boxes' union (the selection's box); null for none. */
export const arrangeUnion = op<(boxes: readonly Bounds[]) => Bounds | null>('arrangeUnion');

/** Each box's displacement, east and north: an alignment's to `at`, a spread's at equal gaps (fewer than three move nothing). */
export const arrangeMoves = op<(boxes: readonly Bounds[], mode: ArrangeMode, at: number | null) => Vec2[]>('arrangeMoves');
