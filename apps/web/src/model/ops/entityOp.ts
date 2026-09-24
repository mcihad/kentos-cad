import { op } from '../../wasm/core';

/**
 * A core operation whose result holds entities or entity geometry
 * (docs/adr/0008, S3). The core writes no field it has no value for, where
 * the TypeScript it replaced set a cleared `bulges` or `holes` to
 * undefined. `CadDocument.update` merges its patch into the entity, so a
 * missing field would keep the old arcs or holes: every polyline, polygon
 * and hatch in the result gets the field back, undefined.
 */
export function entityOp<F extends (...args: never[]) => unknown>(name: string): F {
  const call = op<(...args: unknown[]) => unknown>(name);
  return ((...args: unknown[]) => {
    const v = call(...args);
    clear(v);
    return v;
  }) as unknown as F;
}

function clear(v: unknown): void {
  if (Array.isArray(v)) {
    for (const x of v) if (x && typeof x === 'object') clear(x);
    return;
  }
  if (!v || typeof v !== 'object') return;
  const o = v as Record<string, unknown>;
  // A point: nothing inside.
  if ('x' in o) return;
  const kind = o.kind;
  if ((kind === 'polyline' || kind === 'polygon') && !('bulges' in o)) o.bulges = undefined;
  if ((kind === 'polygon' || kind === 'hatch') && !('holes' in o)) o.holes = undefined;
  for (const key in o) {
    const x = o[key];
    if (x && typeof x === 'object') clear(x);
  }
}
