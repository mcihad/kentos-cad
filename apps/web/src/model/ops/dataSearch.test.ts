import { describe, expect, it } from 'vitest';
import { dataSearch, type SearchFields, type SearchQuery, type SearchRecord } from './dataSearch';
import { fixture } from './dataSearchFixture';

/**
 * Veride ara's matching (docs/adr/0178 §2, §4) through the WASM core, against the independent reference in
 * fixtures/search/v1/cases.json (scripts/fixtures/data_search_cases.py, no KentOS code), the cases the core runs natively
 * in crates/shared/geometry-core/tests/all/data_search.rs: exactly, the attribute's name of a row that has none null.
 */
/** JSON with each object's keys in order, to compare answers whatever order the fields were written in. */
const canon = (v: unknown) => JSON.stringify(v, (_k, x: unknown) => (x && typeof x === 'object' && !Array.isArray(x) ? Object.fromEntries(Object.entries(x).sort(([a], [b]) => (a < b ? -1 : 1))) : x));

/** A row as the reference writes it: a field the core leaves out (the attribute's name of a label) is null. */
const written = (f: { rows: { name?: string | null }[]; total: number }) => ({ ...f, rows: f.rows.map((r) => ({ ...r, name: r.name ?? null })) });

const ALL: SearchFields = { label: true, text: true, block: true, attrs: true, attrName: null };

describe('Veride ara', () => {
  it('is the reference’s file', () => expect([fixture.format, fixture.version]).toEqual(['kentos.search-fixtures', 1]));

  it('matches a word as the reference does', () => {
    expect(fixture.match.length).toBeGreaterThanOrEqual(100);
    // The search trims the word and the value and an empty one asks or holds nothing: the cases it reads as they are
    // (the core's own test runs every one through `matches`).
    const plain = fixture.match.filter((c) => c.text.trim() === c.text && c.pattern.trim() === c.pattern && c.text !== '' && c.pattern !== '');
    expect(plain.length).toBeGreaterThanOrEqual(100);
    const off = plain.flatMap((c) => {
      const record: SearchRecord = { kind: '', layer: '', label: c.text, text: null, block: null, attrs: [] };
      const query: SearchQuery = { pattern: c.pattern, matchCase: c.matchCase, wholeWord: c.wholeWord, fields: ALL, sort: null, descending: false, limit: 0 };
      const found = dataSearch([record], query).total === 1;
      return found === c.expected ? [] : [`${c.name}: ${found} ≠ ${c.expected}`];
    });
    expect(off).toEqual([]);
  });

  it('finds, orders and cuts the rows as the reference does', () => {
    expect(fixture.search.length).toBeGreaterThanOrEqual(100);
    const off = fixture.search.flatMap((c) => {
      const got = written(dataSearch(fixture.drawings[c.drawing], c.query));
      const want = c.expected;
      return canon(got) === canon(want) ? [] : [`${c.name}: ${canon(got).slice(0, 200)} ≠ ${canon(want).slice(0, 200)}`];
    });
    expect(off).toEqual([]);
  });

  it('asks nothing of a word that is empty once trimmed', () => {
    const q: SearchQuery = { pattern: '   ', matchCase: false, wholeWord: false, fields: ALL, sort: null, descending: false, limit: 0 };
    expect(dataSearch(fixture.drawings[0], q)).toEqual({ rows: [], total: 0 });
  });
});
