import { describe, expect, it } from 'vitest';
import type { AttributeDefinition } from '../../contracts/generated/AttributeDefinition';
import type { Vec2 } from '../../model/geometry';
import { definitionOf, newRow, numberText, rowOf, type Extent } from './attributeTable';

/**
 * The Blok öznitelikleri window's table (fixtures/blocks/v1/attribute-table.json,
 * written by scripts/fixtures/attribute_table_cases.py from the rule): a
 * definition as a row and back, and where a new row goes. The desktop reads
 * the same file (apps/desktop/src/attribute_table.rs).
 */

const files = import.meta.glob<string>('../../../../../fixtures/blocks/v1/attribute-table.json', { query: '?raw', import: 'default', eager: true });
/** JSON carries no NaN: the file writes "NaN". */
const read = (text: string) => JSON.parse(text, (_k, v: unknown) => (v === 'NaN' ? Number.NaN : v));
const fixture = read(Object.values(files)[0]) as {
  numbers: [number, string][];
  rows: { name: string; base: Vec2; definition: AttributeDefinition; cells: Record<string, string> }[];
  definitions: { name: string; base: Vec2; from: AttributeDefinition; cells: Record<string, string>; definition: AttributeDefinition }[];
  newRows: { name: string; base: Vec2; above: AttributeDefinition[]; extent: Extent | null; height: number; cells: Record<string, string>; p: Vec2; exactHeight: number }[];
};

describe('the block attributes table (fixtures/blocks/v1/attribute-table.json)', () => {
  it('writes a number with six decimals at most, never -0', () => {
    for (const [n, text] of fixture.numbers) expect(numberText(n), String(n)).toBe(text);
  });

  it('shows a definition as its row', () => {
    for (const c of fixture.rows) expect(rowOf(c.definition, c.base).cells, c.name).toEqual(c.cells);
  });

  it('gives a row back as its definition: untouched cells exactly, typed ones as read', () => {
    for (const c of fixture.definitions) {
      const row = { ...rowOf(c.from, c.base), cells: c.cells };
      // toEqual takes NaN for NaN, and a key left out is not an undefined one.
      expect(definitionOf(row, c.base), c.name).toStrictEqual(c.definition);
    }
  });

  it('puts a new row below the last one, the first right of the drawing', () => {
    for (const c of fixture.newRows) {
      const row = newRow(
        c.above.map((a) => rowOf(a, c.base)),
        c.base,
        c.extent,
        c.height,
      );
      expect(row.cells, c.name).toEqual(c.cells);
      expect(row.exact, c.name).toEqual({ height: c.exactHeight, rotation: 0, p: c.p });
    }
  });
});
