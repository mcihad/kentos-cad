import type { SearchQuery, SearchRecord } from './dataSearch';

/**
 * The independent reference of Veride ara (fixtures/search/v1/cases.json, scripts/fixtures/data_search_cases.py, no
 * KentOS code), read for the tests of the matching (./dataSearch.test.ts) and of the records (../dataSearch.test.ts).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

export interface SearchFixture {
  format: string;
  version: number;
  match: { name: string; text: string; pattern: string; matchCase: boolean; wholeWord: boolean; expected: boolean }[];
  drawings: SearchRecord[][];
  search: { name: string; drawing: number; query: SearchQuery; expected: unknown }[];
  names: { name: string; drawing: number; expected: string[] }[];
  records: { name: string; layers: Record<string, string>; blocks: Record<string, string>; entity: unknown; expected: SearchRecord | null }[];
}

export const fixture = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/search/v1/cases.json', import.meta.url), 'utf8')) as SearchFixture;
