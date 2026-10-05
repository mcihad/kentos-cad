// Records the .kstil style file's rules into fixtures/style/v1/kstil.json (style/file.ts): reading a file (what
// is refused and why, what an accepted file holds after its SVG drawings are cleaned), the symbol checks and
// their texts, SVG cleaning, what an export takes (the assets its symbols use, from any source), what an import
// does in each conflict mode, and a new SVG drawing's size. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-kstil.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in
// the diff. src/style/kstilFixture.test.ts keeps checking them; the desktop's style files check the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { LibraryItem, Symbol } from '../../src/model/style';
import { exportStyles, importStyles, parseStyleFile, sanitizeSvg, STYLE_FORMAT, STYLE_VERSION, svgAsset, validateSymbol, type ImportReport, type StyleFile } from '../../src/style/file';
import { StyleLibrary } from '../../src/style/library';

const OUT = new URL('../../../../fixtures/style/v1/kstil.json', import.meta.url);

type State = { system: LibraryItem[]; user: LibraryItem[]; project: LibraryItem[] };

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

const SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 30"><circle cx="10" cy="10" r="8" fill="param(fill) #2E7D32"/></svg>';
const asset = (id: string, name: string, data = SVG): LibraryItem => ({ kind: 'asset', id, name, path: ['Çizimler'], format: 'svg', data, width: 20, height: 30 });
const symbol = (id: string, name: string, sym: Symbol, path = ['Semboller']): LibraryItem => ({ kind: 'symbol', id, name, path, symbol: sym });
const svgMarker = (assetId: string): Symbol => ({ type: 'marker', layers: [{ id: 'v', type: 'svg', asset: assetId, size: 4 }] });
const tileFill = (assetId: string): Symbol => ({ type: 'fill', layers: [{ id: 'i', type: 'imageFill', asset: assetId, tileSize: 5 }] });
const plainFill: Symbol = { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#EDC948' }] };
const file = (items: unknown[], extra: Record<string, unknown> = {}, version = 1) => JSON.stringify({ format: STYLE_FORMAT, version, exported: '2026-09-27T08:00:00.000Z', items, ...extra });
// Object templates (docs/adr/0176): a template that draws with a symbol of the library, and one with its own look.
const template = (id: string, name: string, body: Record<string, unknown>, path = ['Şablonlar']): LibraryItem => ({ kind: 'template', id, name, path, template: body }) as unknown as LibraryItem;
const parcelTemplate = { tool: 'polygon', layer: { path: ['Kadastro'], name: 'Parsel', color: '#E5484D', lineWeight: 0.35 }, symbol: 's-benim', attrs: { Tür: 'Parsel' }, label: 'P' };
const pointTemplate = { tool: 'point', layer: { path: [], name: 'Nokta' }, color: '#3E63DD', point: { name: 'P1', code: 'SN' } };

const PARSE: { id: string; text: string }[] = [
  { id: 'not-json', text: '{ "format": "kentos-style", ' },
  { id: 'not-kstil', text: JSON.stringify({ format: 'kentos.document', version: 1 }) },
  { id: 'newer', text: JSON.stringify({ format: STYLE_FORMAT, version: 3, items: [] }) },
  { id: 'no-version', text: JSON.stringify({ format: STYLE_FORMAT, items: [] }) },
  { id: 'version-not-whole', text: JSON.stringify({ format: STYLE_FORMAT, version: 1.5, items: [] }) },
  { id: 'no-items', text: JSON.stringify({ format: STYLE_FORMAT, version: 1 }) },
  { id: 'categories-not-a-list', text: file([], { categories: 5 }) },
  { id: 'bad-categories', text: file([], { categories: ['A', { path: 'A' }, { path: ['A', 7], order: 'ilk', description: 3 }, { path: ['B'], order: 2, description: 'Açıklama' }] }) },
  { id: 'categories-kept-to-what-they-are', text: file([], { categories: [{ path: ['A', 'B'], order: 1, description: 'Açıklama', source: 'system', renk: '#000000' }, { path: [] }] }) },
  { id: 'empty', text: file([]) },
  {
    id: 'bad-items',
    text: file([
      'metin',
      { kind: 'symbol', name: 'Kimliksiz', path: ['A'], symbol: plainFill },
      { kind: 'symbol', id: 's2', path: 'A', symbol: plainFill },
      { kind: 'palette', id: 'x', name: 'X', path: [] },
      { kind: 'asset', id: 'a1', name: 'Biçimsiz', path: [], format: 'gif', data: 'GIF89a', width: 1, height: 1 },
      { kind: 'asset', id: 'a2', name: 'SVG değil', path: [], format: 'svg', data: '<html></html>', width: 1, height: 1 },
      { kind: 'asset', id: 'a3', name: 'Adres', path: [], format: 'png', data: 'https://example.com/a.png', width: 1, height: 1 },
      { kind: 'asset', id: 'a4', name: 'Boyutsuz', path: [], format: 'svg', data: SVG, width: 0 },
      { kind: 'asset', id: 'a5', name: 'Verisiz', path: [], format: 'svg', width: 1, height: 1 },
    ]),
  },
  {
    id: 'bad-symbol',
    text: file([
      symbol('s1', 'Bozuk', {
        type: 'line',
        layers: [
          { id: 'l', type: 'simpleLine', color: 'kırmızı', width: -1, dash: [0, 0] },
          { id: 'm', type: 'markerLine', placement: 'interval', marker: { type: 'marker', layers: [{ id: 's', type: 'shape', shape: 'yıldızımsı', size: 1 }] } },
        ],
      } as unknown as Symbol),
    ]),
  },
  {
    id: 'accepted-and-cleaned',
    text: file(
      [
        asset(
          'a-temiz',
          'Kirli çizim',
          '<?xml version="1.0"?><!DOCTYPE svg><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" onload="alert(1)"><script>alert(2)</script><foreignObject><div>x</div></foreignObject><image href="https://example.com/x.png"/><rect width="10" height="10" fill="url(https://example.com/p)" style="fill:url(#g)"/></svg>',
        ),
        symbol('s-temiz', 'Ağaç', svgMarker('a-temiz')),
      ],
      { categories: [{ path: ['Semboller'], order: 1 }] },
    ),
  },
  // Version 2 brings templates; a template is checked by its own rules (fixtures/style/v1/object-templates.json).
  { id: 'templates', text: file([symbol('s-benim', 'Benim sembolüm', plainFill), template('t-parsel', 'Parsel sınırı', parcelTemplate), template('t-nokta', 'Poligon noktası', pointTemplate)], {}, 2) },
  { id: 'bad-templates', text: file([template('t-bozuk', 'Bozuk şablon', { tool: 'arc', color: 'mavi' }), { kind: 'template', id: 't-2', name: 'Şablonsuz', path: [] }], {}, 2) },
];

const SANITIZE = [
  '<svg xmlns="http://www.w3.org/2000/svg"><a xlink:href="javascript:alert(1)"><text onclick=\'x()\'>t</text></a></svg>',
  '<svg xmlns="http://www.w3.org/2000/svg"><use href="#iç"/><image href="data:image/png;base64,AAAA"/><image href="dış.png"/></svg>',
  '  <svg xmlns="http://www.w3.org/2000/svg"><script src="a.js"/><path d="M0 0" fill="url( \'x.svg#g\' )"/></svg>  ',
];

const SYMBOLS: { id: string; symbol: unknown; where?: string }[] = [
  { id: 'fine', symbol: plainFill },
  { id: 'not-object', symbol: 7 },
  { id: 'unknown-type', symbol: { type: 'volume', layers: [] } },
  { id: 'no-layers', symbol: { type: 'fill' } },
  { id: 'wrong-layer-kind', symbol: { type: 'marker', layers: [{ id: 'l', type: 'simpleLine', color: '#000000', width: 1 }] } },
  {
    id: 'numbers-and-colours',
    where: 'öğe 3 (Deneme)',
    symbol: {
      type: 'marker',
      layers: [
        { type: 'shape', shape: 'gear', size: 'büyük', fill: '#12345', stroke: null, strokeWidth: -2, teeth: 2, opacity: -1, unit: 'inch' },
        { id: 't', type: 'text', size: { expr: '[Kat]' }, color: { expr: 'renk' } },
        { id: 'v', type: 'svg', size: 3 },
      ],
    },
  },
  {
    id: 'lines-and-fills',
    symbol: {
      type: 'fill',
      layers: [
        { id: 'w', type: 'simpleLine', color: 'fg-dim', width: 0.3, dash: [1, -1], shift: [1], wave: { shape: 'bump', length: 1 } },
        { id: 'w2', type: 'simpleLine', color: 'ink', width: 0.3, wave: { shape: 'sine', length: 0, amplitude: -1, spacing: 0 } },
        { id: 'h', type: 'hatchFill', spacing: 0, width: 0.1, color: 'paper' },
        { id: 'p', type: 'patternFill', spacingX: 1, marker: { type: 'line', layers: [] } },
        { id: 'i', type: 'imageFill', tileSize: 0 },
        { id: 'c', type: 'centroidMarker', marker: { type: 'marker', layers: [{ id: 's', type: 'shape', shape: 'star', size: 1, fill: '#FF000080' }] } },
        { id: 'm', type: 'markerLine', placement: 'everywhere', marker: { type: 'marker', layers: [] } },
      ],
    },
  },
];

const EXPORT_LIBRARY: State = {
  system: [asset('a-sistem', 'Sistem çizimi'), symbol('s-sistem', 'Sistem ağacı', svgMarker('a-sistem'), ['MPYY'])],
  user: [
    asset('a-kullanici', 'Benim çizimim'),
    symbol('s-benim', 'Benim sembolüm', svgMarker('a-kullanici')),
    symbol('s-doku', 'Dokulu', tileFill('a-sistem')),
    template('t-parsel', 'Parsel sınırı', parcelTemplate),
  ],
  project: [symbol('s-proje', 'Proje sembolü', plainFill, ['Proje']), template('t-nokta', 'Poligon noktası', pointTemplate, ['Proje'])],
};
// A template takes along the symbol it draws with, and that symbol its assets; a file with a template is version 2.
const EXPORTS = [['s-benim'], ['s-doku', 's-benim', 's-doku'], ['s-proje', 'yok'], ['s-sistem'], ['t-parsel'], ['t-nokta', 's-proje']];

// The template comes before the symbol it draws with: an import takes assets, then symbols, then templates, and a template copied
// beside a renamed symbol points at the new one.
const INCOMING: StyleFile = {
  format: STYLE_FORMAT,
  version: 2,
  exported: '2026-09-27T08:00:00.000Z',
  items: [
    template('t-gelen', 'Gelen şablon', { ...parcelTemplate, symbol: 's-benim' }),
    symbol('s-benim', 'Gelen sembol', svgMarker('a-kullanici')),
    asset('a-kullanici', 'Gelen çizim'),
    symbol('s-yeni', 'Yeni sembol', plainFill),
    symbol('s-sistem', 'Sisteme çakışan', plainFill),
  ],
  categories: [{ path: ['Semboller', 'Gelen'], order: 0 }],
};
const IMPORTS = (['replace', 'copy', 'skip'] as const).flatMap((mode) =>
  (['user', 'project'] as const).map((to) => ({ id: `${mode}-into-${to}`, library: EXPORT_LIBRARY, file: INCOMING, to, mode })),
);

const SVG_ASSETS = [
  { name: 'Görünüm kutulu', path: ['Çizimler'], svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 12" width="48" height="24"><rect width="24" height="12"/></svg>' },
  { name: 'Boyutlu', path: [], svg: '<svg xmlns="http://www.w3.org/2000/svg" width="40.5" height="10"><rect width="40" height="10"/></svg>' },
  { name: 'Boyutsuz', path: ['A', 'B'], svg: '<svg xmlns="http://www.w3.org/2000/svg"><script>x()</script><circle r="5"/></svg>' },
];

it.runIf(!!process.env.GOLDEN_WRITE)('records the .kstil rules', () => {
  const lib = libraryOf(EXPORT_LIBRARY);
  const out = {
    format: 'kentos.style-file-cases',
    version: 1,
    note: '.kstil stil dosyası (style/file.ts): okuma (neyin neden reddedildiği; kabul edilen dosyada SVG çizimleri temizlenmiş öğeler; sürüm 2 nesne şablonlarını getirir, şablon kendi kurallarıyla denetlenir, ADR 0176), sembol denetimi ve sözleri, SVG temizliği (XML başlığı, DOCTYPE, betik, olay işleyicisi, foreignObject, dışarıya bağlantı ve dış url() kalkar; iç bağlantı ve data:image kalır), dışa aktarmanın aldıkları (sembollerin kullandığı varlıklar, şablonların çizdiği semboller, hangi kaynaktan olursa olsun; her öğe bir kez; kaynak alanı yazılmaz; şablonlu dosya sürüm 2, öbürü 1), içe aktarmanın her çakışma kipinde yaptıkları (varlıklar, sonra semboller, sonra şablonlar; değiştir yalnız aynı kaynakta, başka kaynakta ya da sistemde atlar; kopya yeni kimlik alır, semboller yeni varlık kimliğine, şablonlar yeni sembol kimliğine döner; kategoriler eklenir) ve yeni SVG çiziminin boyu (viewBox, yoksa width/height, yoksa 100). Yeni kimlikler rastgeledir: {new:eski} ile yazılıdır, önekleri prefixes’tedir. exported zamanı karşılaştırılmaz. Yanıtlar web’indir ve kaydedilirken okunmuştur.',
    styleFormat: STYLE_FORMAT,
    styleVersion: STYLE_VERSION,
    parse: PARSE.map((c) => {
      const r = parseStyleFile(c.text);
      return { ...c, expect: { issues: r.issues, ...(r.file ? { items: r.file.items, categories: r.file.categories ?? [] } : {}) } };
    }),
    sanitize: SANITIZE.map((svg) => ({ svg, expect: sanitizeSvg(svg) })),
    symbols: SYMBOLS.map((c) => ({ ...c, expect: validateSymbol(c.symbol, c.where) })),
    exportLibrary: EXPORT_LIBRARY,
    exports: EXPORTS.map((ids) => {
      const f = exportStyles(lib, ids);
      return { ids, expect: { format: f.format, version: f.version, items: f.items } };
    }),
    imports: IMPORTS.map((c) => {
      const target = libraryOf(c.library);
      const report = importStyles(target, c.file, c.to, c.mode);
      return { id: c.id, to: c.to, mode: c.mode, expect: normalizeImport(report, target) };
    }),
    importFile: INCOMING,
    svgAssets: SVG_ASSETS.map((c) => {
      const { id: _id, ...rest } = svgAsset(c.name, c.path, c.svg);
      return { ...c, expect: rest };
    }),
  };
  writeFileSync(OUT, `${JSON.stringify(out, null, 1)}\n`);
});
