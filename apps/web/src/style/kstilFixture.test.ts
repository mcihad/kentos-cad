import { describe, expect, it } from 'vitest';
import type { LibraryItem } from '../model/style';
import { exportStyles, importStyles, parseStyleFile, sanitizeSvg, STYLE_FORMAT, STYLE_VERSION, svgAsset, validateSymbol, type ConflictMode, type ImportReport, type StyleFile } from './file';
import { StyleLibrary, type EditableSource } from './library';

/**
 * The .kstil style file's rules as fixtures/style/v1/kstil.json holds them
 * (scripts/fixtures/record-kstil.test.ts): reading and refusing files,
 * symbol checks, SVG cleaning, export and import. The desktop's style files
 * check the same file.
 */

type State = { system: LibraryItem[]; user: LibraryItem[]; project: LibraryItem[] };

const files = import.meta.glob<string>('../../../../fixtures/style/v1/kstil.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  styleFormat: string;
  styleVersion: number;
  parse: { id: string; text: string; expect: unknown }[];
  sanitize: { svg: string; expect: string }[];
  symbols: { id: string; symbol: unknown; where?: string; expect: string[] }[];
  exportLibrary: State;
  exports: { ids: string[]; expect: unknown }[];
  imports: { id: string; to: EditableSource; mode: ConflictMode; expect: unknown }[];
  importFile: StyleFile;
  svgAssets: { name: string; path: string[]; svg: string; expect: unknown }[];
};

const libraryOf = (s: State): StyleLibrary => {
  const lib = new StyleLibrary({ items: s.system });
  lib.load('user', s.user);
  lib.load('project', s.project);
  return lib;
};

/** An import's report and the editable sources after it, new ids written as {new:old}. */
function normalizeImport(report: ImportReport, lib: StyleLibrary): unknown {
  const back = new Map(Object.entries(report.renamed).map(([old, id]) => [id, `{new:${old}}`]));
  const swap = (v: unknown): unknown => JSON.parse(JSON.stringify(v, (_k, x: unknown) => (typeof x === 'string' && back.has(x) ? back.get(x) : x)));
  return swap({
    report: { added: report.added, replaced: report.replaced, skipped: report.skipped, renamed: report.renamed },
    prefixes: Object.fromEntries(Object.values(report.renamed).map((id) => [back.get(id), id.split('-')[0]])),
    user: lib.dump('user'),
    project: lib.dump('project'),
  });
}

describe('.kstil style files (fixtures/style/v1/kstil.json)', () => {
  it('is a v1 file for format kentos-style v1', () => {
    expect([F.format, F.version, F.styleFormat, F.styleVersion]).toEqual(['kentos.style-file-cases', 1, STYLE_FORMAT, STYLE_VERSION]);
  });

  for (const c of F.parse)
    it(`reads: ${c.id}`, () => {
      const r = parseStyleFile(c.text);
      expect({ issues: r.issues, ...(r.file ? { items: r.file.items, categories: r.file.categories ?? [] } : {}) }).toEqual(c.expect);
    });

  it('cleans SVG drawings', () => {
    for (const s of F.sanitize) expect(sanitizeSvg(s.svg), s.svg).toBe(s.expect);
  });

  for (const c of F.symbols) it(`checks a symbol: ${c.id}`, () => expect(validateSymbol(c.symbol, c.where)).toEqual(c.expect));

  it('exports the chosen items and the assets their symbols use', () => {
    const lib = libraryOf(F.exportLibrary);
    for (const x of F.exports) {
      const f = exportStyles(lib, x.ids);
      expect({ format: f.format, version: f.version, items: f.items }, x.ids.join(', ')).toEqual(x.expect);
    }
  });

  for (const c of F.imports)
    it(`imports: ${c.id}`, () => {
      const lib = libraryOf(F.exportLibrary);
      expect(normalizeImport(importStyles(lib, F.importFile, c.to, c.mode), lib)).toEqual(c.expect);
    });

  it('sizes a new SVG drawing', () => {
    for (const a of F.svgAssets) {
      const { id: _id, ...rest } = svgAsset(a.name, a.path, a.svg);
      expect(rest, a.name).toEqual(a.expect);
    }
  });
});
