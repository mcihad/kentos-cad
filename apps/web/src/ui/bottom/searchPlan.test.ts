import { describe, expect, it } from 'vitest';
import { Formatter } from '../../app/format';
import { ProjectSettings } from '../../model/projectSettings';
import { countText, fieldCell, fieldsOn, nextSort, placeOf, queryOf, SEARCH_COLUMNS, SEARCH_LIMIT, SEARCH_TEXTS, searchDefaults } from './searchPlan';

/**
 * The rules of Arama (docs/adr/0178) that do not touch the page: the columns, the header's three-way sort, the count line,
 * a row's Alan, the core's query for the panel's choices, the place a typed word names and the coordinate's wording in the
 * project type's axes. The desktop's are `apps/desktop/src/search/` (its tests say the same).
 */
describe('Arama: the panel’s rules', () => {
  it('lists Sıra, the drawing’s order, then the four columns the core sorts by', () => {
    expect(SEARCH_COLUMNS.map((c) => [c.label, c.sort])).toEqual([
      ['Sıra', null],
      ['Katman', 'layer'],
      ['Tür', 'kind'],
      ['Alan', 'field'],
      ['Değer', 'value'],
    ]);
  });

  it('sorts a header ascending, then descending, then in the drawing’s order; Sıra is the drawing’s order', () => {
    let s = nextSort(null, false, 'value');
    expect(s).toEqual({ sort: 'value', descending: false });
    s = nextSort(s.sort, s.descending, 'value');
    expect(s).toEqual({ sort: 'value', descending: true });
    s = nextSort(s.sort, s.descending, 'value');
    expect(s).toEqual({ sort: null, descending: false });
    // Another header starts ascending; Sıra puts the drawing’s order back.
    expect(nextSort('layer', true, 'kind')).toEqual({ sort: 'kind', descending: false });
    expect(nextSort('layer', false, null)).toEqual({ sort: null, descending: false });
  });

  it('says how many answer and, when the list is cut, how many are listed', () => {
    expect(countText(3, 3)).toBe('3 sonuç');
    expect(countText(SEARCH_LIMIT, 7000)).toBe('5000 / 7000 sonuç');
  });

  it('names a row’s field, an attribute by its name, with the others that answer counted', () => {
    expect(fieldCell({ field: 'label', name: null, more: 0 })).toBe('Ad / etiket');
    expect(fieldCell({ field: 'label', name: null, more: 2 })).toBe('Ad / etiket +2');
    expect(fieldCell({ field: 'text', name: null, more: 0 })).toBe('Yazı');
    expect(fieldCell({ field: 'block', name: null, more: 1 })).toBe('Blok adı +1');
    expect(fieldCell({ field: 'attr', name: 'Parsel', more: 0 })).toBe('Parsel');
  });

  it('asks the core for what the choices are, every field on at first', () => {
    const s = { ...searchDefaults(), text: 'Ada *', wholeWord: true, sort: 'value' as const, descending: true };
    expect(queryOf(s)).toEqual({
      pattern: 'Ada *',
      matchCase: false,
      wholeWord: true,
      fields: { label: true, text: true, block: true, attrs: true, attrName: null },
      sort: 'value',
      descending: true,
      limit: SEARCH_LIMIT,
    });
    expect(fieldsOn(s.fields)).toBe(true);
    expect(fieldsOn({ label: false, text: false, block: false, attrs: false, attrName: null })).toBe(false);
  });

  it('takes an absolute point for a place and nothing else, in metres', () => {
    const same = (v: number) => v;
    expect(placeOf('487012.5,4420000', same)).toEqual({ x: 487012.5, y: 4420000 });
    expect(placeOf(' 12 ; -3.5 ', same)).toEqual({ x: 12, y: -3.5 });
    expect(placeOf('1000 2000', (v) => v / 1000)).toEqual({ x: 1, y: 2 });
    for (const text of ['@12,5', '12<45', '101', 'Ada 101', '12,5,3', '', '1.,2']) expect(placeOf(text, same), text).toBeNull();
  });

  it('names the coordinate order as the project type does (Y,X in CBS, X,Y in CAD)', () => {
    const settings = new ProjectSettings({ workspace: 'gis' });
    const f = new Formatter(settings);
    const words = () => [f.axesText(SEARCH_TEXTS.placeholder), f.axesText(SEARCH_TEXTS.searchHint), f.axesText(SEARCH_TEXTS.noQuery)];
    expect(words().every((w) => w.includes('Y,X') && !w.includes('X,Y'))).toBe(true);
    settings.assign({ workspace: 'cad' });
    expect(words().every((w) => w.includes('X,Y') && !w.includes('Y,X'))).toBe(true);
    // The typed order is east then north in both: only its name changes.
    expect(placeOf('10,20', (v) => v)).toEqual({ x: 10, y: 20 });
  });
});
