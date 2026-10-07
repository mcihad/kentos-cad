import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/exchange/v1/cases.json?raw';
import { blockKey, exchangeFold, fileBlock, selectionDrawing, takeFrom, uniqueNames, type Picks, type Same } from './exchange';

/**
 * Çizimler arası alışveriş (docs/adr/0193) on the shared cases fixtures/exchange/v1/cases.json
 * (scripts/fixtures/exchange_cases.py, no KentOS code): the selection's drawing, taking from another drawing, a drawing
 * as a block (its base by the WASM core's boxes). The desktop plays the same file
 * (crates/native/domain/tests/all/exchange.rs).
 */
const f = JSON.parse(text);

describe('Çizimler arası alışveriş (fixtures/exchange/v1)', () => {
  it("keeps what the selection's objects use", () => {
    for (const c of f.selections) expect(selectionDrawing(f.drawings[c.from], c.uids, 'Seçim'), c.name).toEqual(c.expect);
  });

  it('takes from another drawing, the same names skipped or replaced', () => {
    for (const c of f.takes) expect(takeFrom(f.drawings[c.into], f.drawings[c.from], c.picks as Picks, c.same as Same), c.name).toEqual(c.expect);
  });

  it('makes a drawing one block', () => {
    for (const c of f.files) {
      const got = fileBlock(f.drawings[c.into], f.drawings[c.from], c.file);
      expect(got?.drawing, c.name).toEqual(c.expect);
      expect([got?.images, got?.tables], c.name).toEqual([c.left.images, c.left.tables]);
    }
    expect(fileBlock(f.drawings.ours, { ...f.drawings.ours, entities: [], uids: [] }, 'Boş')).toBeNull();
  });

  it('folds names as the rule says', () => {
    expect(exchangeFold('  ada NO ')).toBe(exchangeFold('Ada no'));
    expect(exchangeFold('Yol adı')).toBe('yol adi');
    expect(blockKey('DİREK')).toBe(blockKey('direk'));
    expect(uniqueNames(['DİREK'], ['Direk', 'Direk'])).toEqual(['Direk (2)', 'Direk (3)']);
  });
});
