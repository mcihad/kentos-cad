import { describe, expect, it } from 'vitest';
import { SHEET_CUT, sheetCells } from './tables';

/**
 * A file sheet's cells (docs/adr/0184 §4): the reader stops one row past a table's, so a sheet it cut is refused in
 * the sheet's words, not by the reader's count; the desktop's `kentos_interaction::table::sheet_cells` alike.
 */
describe('sheetCells', () => {
  const sheet = (n: number, cut: boolean) => ({ rows: Array.from({ length: n }, () => ['a']), cut });

  it('says a cut sheet is longer than a table', () => {
    expect(sheetCells(sheet(10_001, true), false).problem).toBe(SHEET_CUT);
  });

  it("keeps the rule's words for a sheet read whole", () => {
    expect(sheetCells(sheet(10_001, false), false).problem).toContain('10001');
  });

  it('makes the table of a sheet cut past its words', () => {
    const cells = sheetCells(sheet(3, true), true);
    expect(cells.problem ?? null).toBeNull();
    expect(cells.cells).toEqual([['a'], ['a'], ['a']]);
  });
});
