import type { AttributeDefinition } from '../../contracts/generated/AttributeDefinition';
import type { Vec2 } from '../../model/geometry';
import { readNumber, type Row } from '../calc/read';

/**
 * The Blok öznitelikleri window's table (docs/adr/0144 §7) as texts: each
 * attribute definition is a row of its tag, prompt, default, text height,
 * turn (degrees) and place as east (Y) and north (X) of the base point, in
 * the block's own units. A number is written as a prompt writes it
 * (`+n.toFixed(6)`); a cell left as it was shown gives back the exact value
 * it showed, so a list saved unchanged is the list it was. A definition's
 * alignment and width factor (docs/adr/0145) have no cells: the row keeps
 * them. The desktop's `apps/desktop/src/attribute_table.rs` is the same.
 */

/** The table's columns: their keys, headings, units and whether they hold numbers. */
export const ATTRIBUTE_COLUMNS = [
  { key: 'tag', label: 'Etiket' },
  { key: 'prompt', label: 'Soru' },
  { key: 'value', label: 'Varsayılan' },
  { key: 'height', label: 'Yükseklik', unit: 'm', numeric: true },
  { key: 'rotation', label: 'Açı', unit: '°', numeric: true },
  { key: 'y', label: 'Y', unit: 'm', numeric: true },
  { key: 'x', label: 'X', unit: 'm', numeric: true },
] as const;

/** A row: its cells' texts, the exact values the numbers were shown from, and what the row has no cells for. */
export interface AttributeRow {
  cells: Row;
  exact: { height: number; rotation: number; p: Vec2 } & Pick<AttributeDefinition, 'align' | 'widthFactor'>;
}

/** A number as a cell shows it: at most six decimals, no trailing zeros, never “-0”. */
export const numberText = (n: number): string => String(+n.toFixed(6));

/** A row showing `a` of a block whose base point is `base`. */
export function rowOf(a: AttributeDefinition, base: Vec2): AttributeRow {
  return {
    cells: {
      tag: a.tag,
      prompt: a.prompt ?? '',
      value: a.value ?? '',
      height: numberText(a.height),
      rotation: numberText(a.rotation),
      y: numberText(a.p.x - base.x),
      x: numberText(a.p.y - base.y),
    },
    exact: {
      height: a.height,
      rotation: a.rotation,
      p: { x: a.p.x, y: a.p.y },
      ...(a.align && { align: a.align }),
      ...(a.widthFactor !== undefined && { widthFactor: a.widthFactor }),
    },
  };
}

/** A cell's number: the exact one while it reads as that was shown; else what it reads (`empty` for an empty cell, NaN for no number). */
function numberOf(text: string | undefined, exact: number, empty: number): number {
  return (text ?? '') === numberText(exact) ? exact : (readNumber(text) ?? empty);
}

/** A place's coordinate from its cell (east or north of the base's `from`): the exact one while it reads as shown. */
function coordinateOf(text: string | undefined, exact: number, from: number): number {
  return (text ?? '') === numberText(exact - from) ? exact : from + (readNumber(text) ?? 0);
}

/**
 * The definition a row gives: the tag, prompt and default trimmed (an empty
 * prompt or default left out), the height (an empty cell is no height), the
 * turn and the place (an empty cell is 0); its alignment and width factor as
 * they were.
 */
export function definitionOf(row: AttributeRow, base: Vec2): AttributeDefinition {
  const { cells, exact } = row;
  const prompt = (cells.prompt ?? '').trim();
  const value = (cells.value ?? '').trim();
  return {
    tag: (cells.tag ?? '').trim(),
    ...(prompt && { prompt }),
    ...(value && { value }),
    p: { x: coordinateOf(cells.y, exact.p.x, base.x), y: coordinateOf(cells.x, exact.p.y, base.y) },
    height: numberOf(cells.height, exact.height, Number.NaN),
    rotation: numberOf(cells.rotation, exact.rotation, 0),
    ...(exact.align && { align: exact.align }),
    ...(exact.widthFactor !== undefined && { widthFactor: exact.widthFactor }),
  };
}

/** The row with its place now `p` (picked on an insert, in the definition's coordinates). */
export function placed(row: AttributeRow, p: Vec2, base: Vec2): AttributeRow {
  return { cells: { ...row.cells, y: numberText(p.x - base.x), x: numberText(p.y - base.y) }, exact: { ...row.exact, p: { x: p.x, y: p.y } } };
}

/** A box east and north of the base point: the block's drawing as placed at its base, unturned. */
export interface Extent {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/**
 * A new row after `above` (the rows before it): a line and a half below the
 * last one's text, its height; the first one right of the block's drawing,
 * its text's top level with the drawing's (at the base point when the block
 * draws nothing), `height` high.
 */
export function newRow(above: readonly AttributeRow[], base: Vec2, extent: Extent | null, height: number): AttributeRow {
  const last = above.at(-1);
  const prev = last && definitionOf(last, base);
  const h = prev && Number.isFinite(prev.height) && prev.height > 0 ? prev.height : height;
  const p =
    prev && Number.isFinite(prev.p.x) && Number.isFinite(prev.p.y)
      ? { x: prev.p.x, y: prev.p.y - 1.5 * h }
      : extent
        ? { x: base.x + extent.maxX + 0.2 * h, y: base.y + extent.maxY - h }
        : { x: base.x, y: base.y };
  return placed({ cells: { tag: '', prompt: '', value: '', height: numberText(h), rotation: '0', y: '', x: '' }, exact: { height: h, rotation: 0, p } }, p, base);
}

/** The box of outline paths (`flags, n, x0, y0, …`, the geometry store's `insertOutlines`); null for none. */
export function outlineExtent(paths: ArrayLike<number>): Extent | null {
  let box: Extent | null = null;
  for (let i = 0; i + 1 < paths.length; ) {
    const n = paths[i + 1];
    for (let k = 0; k < n; k++) {
      const x = paths[i + 2 + 2 * k];
      const y = paths[i + 3 + 2 * k];
      box = box
        ? { minX: Math.min(box.minX, x), minY: Math.min(box.minY, y), maxX: Math.max(box.maxX, x), maxY: Math.max(box.maxY, y) }
        : { minX: x, minY: y, maxX: x, maxY: y };
    }
    i += 2 + 2 * n;
  }
  return box;
}
