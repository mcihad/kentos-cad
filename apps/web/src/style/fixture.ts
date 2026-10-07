import type { GeometrySource } from '../render/styledLayer';
import type { StyledBatch } from '../render/types';
import type { PickIndex } from '../viewport/picking';

/**
 * Test support for the style fixtures (fixtures/style/v1/cases.json,
 * scripts/fixtures/record-style.test.ts): what the page gives the style
 * core for a layer build and what comes back, caught on the way through
 * the geometry store.
 */

/** A layer build's call to the core: the program, the objects' four numbers each, the expression table, and the answer. */
export interface StyledCall {
  readonly program: string;
  readonly objects: readonly number[];
  readonly table: { readonly texts: string; readonly lens: readonly number[]; readonly numbers: readonly (number | string)[] };
  readonly batches: readonly unknown[];
  readonly data: readonly (number | string)[];
  /** Screen-sized symbols (paper mm drawn as px); absent: plot-sized. */
  readonly screen?: boolean;
}

/** A number as JSON cannot keep it: "NaN", "Infinity", "-Infinity" and "-0" as text. */
export const encodeNumber = (x: number): number | string => (Object.is(x, -0) ? '-0' : Number.isFinite(x) ? x : String(x));
export const decodeNumber = (x: number | string): number => (typeof x === 'number' ? x : x === '-0' ? -0 : Number(x));

/** A float32 in the fewest digits that read back as the same float32 (through a double, as both readers do). */
export function encodeFloat32(x: number): number | string {
  if (!Number.isFinite(x) || Object.is(x, -0)) return encodeNumber(x);
  for (let p = 1; p < 9; p++) {
    const s = Number(x.toPrecision(p));
    if (Math.fround(s) === x) return s;
  }
  return x;
}

/** The store as a layer build's geometry source, telling `seen` about each styled call. */
export function captureStyled(index: PickIndex, seen: (c: StyledCall) => void): GeometrySource {
  return {
    drawn: (ids, oriented, clip) => index.drawn(ids, oriented, clip),
    blockPieces: (block) => index.blockPieces(block),
    insertPieces: (id) => index.insertPieces(id),
    styled: (program, ids, objects, pieces, table, clip, origin, plotScale, screen, view) => {
      const out = index.styled(program, ids, objects, pieces, table, clip, origin, plotScale, screen, view);
      seen({
        program: program.json,
        objects: [...objects],
        table: { texts: table.texts, lens: [...table.lens], numbers: [...table.numbers].map(encodeNumber) },
        batches: JSON.parse(out.json) as unknown[],
        data: [...out.data].map(encodeFloat32),
        ...(screen ? { screen } : {}),
      });
      return out;
    },
  };
}

/** A styled layer's GPU batches as fixtures/style/v1/batches.json writes them: typed arrays as float32 lists, the rest as it is. */
export function batchesJson(batches: readonly StyledBatch[]): unknown[] {
  return batches.map((b) => JSON.parse(JSON.stringify(b, (_k, v: unknown) => (v instanceof Float32Array ? [...v].map(encodeFloat32) : v))) as unknown);
}

/** How the core draws an object (style/build.rs `MODE_*`), by name. */
export const DRAW_MODES = ['skip', 'dimension', 'set', 'own', 'renderer'] as const;

/** An object's four numbers (mode, set or own symbol, simple-look set, colour) with the mode named. */
export function decisionsOf(ids: readonly number[], objects: readonly number[]): { id: number; mode: (typeof DRAW_MODES)[number]; a: number; simple: number; color: number }[] {
  return ids.map((id, i) => ({ id, mode: DRAW_MODES[objects[4 * i]], a: objects[4 * i + 1], simple: objects[4 * i + 2], color: objects[4 * i + 3] }));
}
