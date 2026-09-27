// Records the legend into fixtures/style/v1/legend.json (style/legend.ts): which layers it reads, the rows each
// gives (the layer's own look, single, categorized, graduated and rule-based renderers, objects' own symbols),
// the picture's layout with and without layer headings, and the window's texts. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-legend.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in
// the diff. src/style/legendFixture.test.ts keeps checking them; the desktop's legend checks the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { CadDocument } from '../../src/model/document';
import type { NewEntity } from '../../src/model/entities';
import { LayerStore, type LayerInit } from '../../src/model/layers';
import type { Symbol } from '../../src/model/style';
import { LEGEND_PAPER, LEGEND_TEXTS, legendLayers, legendLayout, legendOf } from '../../src/style/legend';

const OUT = new URL('../../../../fixtures/style/v1/legend.json', import.meta.url);

const LIBRARY: Record<string, { name?: string; symbol: Symbol }> = {
  yol: { name: 'Yol', symbol: { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color: '#9C755F', width: 0.7 }] } },
  arsa: { name: 'Arsa', symbol: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#EDC948' }] } },
  tarla: { name: 'Tarla', symbol: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#8CD17D' }] } },
  kot: { name: 'Kot', symbol: { type: 'marker', layers: [{ id: 'c', type: 'shape', shape: 'circle', size: 1.5, fill: '#4E79A7' }] } },
  agac: { name: 'Ağaç', symbol: { type: 'marker', layers: [{ id: 't', type: 'shape', shape: 'triangle', size: 2, fill: '#59A14F' }] } },
  // In the library without a name: the legend shows its id.
  adsiz: { symbol: { type: 'marker', layers: [{ id: 's', type: 'shape', shape: 'square', size: 1, fill: '#B07AA1' }] } },
};

const plainFill = (color: string): Symbol => ({ type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color }] });
const plainLine = (color: string): Symbol => ({ type: 'line', layers: [{ id: 'l', type: 'simpleLine', color, width: 0.35 }] });

const LAYERS: (LayerInit & { visible?: boolean })[] = [
  // Its own look, areas and points: a row per geometry, named after it.
  { id: 'yapi', name: 'Yapılar', style: { color: '#E15759', lineType: 'continuous', lineWeight: 0.35, fill: '#E1575933', point: { symbol: 'ring', size: 7 } } },
  { id: 'yollar', name: 'Yollar', style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25, renderer: { type: 'single', symbols: { line: { ref: 'yol' }, fill: { ref: 'arsa' } } } } },
  {
    id: 'parsel',
    name: 'Parseller',
    style: {
      color: 'fg',
      lineType: 'continuous',
      lineWeight: 0.25,
      renderer: {
        type: 'categorized',
        expr: 'Nitelik',
        categories: [
          { value: 'Arsa', label: 'Arsa alanı', symbols: { fill: { ref: 'arsa' } } },
          { value: 'Tarla', label: 'Tarla', symbols: { fill: { ref: 'tarla' } }, enabled: false },
          { value: 'Bahçe', label: '', symbols: { fill: plainFill('#76B7B2') } },
          { value: 'Yok', label: 'Kitaplıkta yok', symbols: { fill: { ref: 'silinmis' } } },
        ],
        other: { fill: plainFill('#BAB0AC'), line: plainLine('#BAB0AC') },
      },
    },
  },
  {
    id: 'kotlar',
    name: 'Kot noktaları',
    style: {
      color: 'fg',
      lineType: 'continuous',
      lineWeight: 0.25,
      renderer: {
        type: 'graduated',
        expr: 'Kot',
        classes: [
          { min: 100, max: 110, label: '100 – 110', symbols: { marker: { ref: 'kot' } } },
          { min: 110, max: 120, label: '110 – 120', symbols: { marker: { ref: 'kot' }, fill: { ref: 'arsa' } } },
        ],
      },
    },
  },
  {
    id: 'plan',
    name: 'Plan',
    style: {
      color: 'fg',
      lineType: 'continuous',
      lineWeight: 0.25,
      renderer: {
        type: 'rules',
        rules: [
          {
            id: 'r1',
            label: 'Konut',
            filter: "Kullanım = 'Konut'",
            symbols: { fill: plainFill('#F28E2B'), line: plainLine('#F28E2B') },
            children: [
              { id: 'r1a', label: 'Yüksek', filter: 'Kat > 5', symbols: { fill: plainFill('#E15759') } },
              { id: 'r1b', label: 'Kapalı kural', enabled: false, symbols: { fill: plainFill('#000000') } },
            ],
          },
          { id: 'r2', label: 'Diğerleri', isElse: true, symbols: { line: { ref: 'yol' } } },
        ],
      },
    },
  },
  // Only a text: no geometry to show, no group.
  { id: 'yazi', name: 'Yazılar', style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 } },
  // Objects with their own symbols (one twice, one missing from the library, one without a name).
  { id: 'kendi', name: 'Semboller', style: { color: 'fg', lineType: 'dashed', lineWeight: 0.25 } },
  // Hidden: read only when every layer is asked for.
  { id: 'gizli', name: 'Gizli katman', visible: false, style: { color: '#9C755F', lineType: 'dotted', lineWeight: 0.25 } },
];

const O = { x: 487000, y: 4420000 };
const P = (x: number, y: number) => ({ x: O.x + x, y: O.y + y });
const sq = (x: number) => [P(x, 0), P(x + 5, 0), P(x + 5, 5), P(x, 5)];
const ENTITIES: NewEntity[] = [
  { kind: 'polygon', layerId: 'yapi', attrs: {}, pts: sq(0) },
  { kind: 'point', layerId: 'yapi', attrs: {}, p: P(2, 8) },
  { kind: 'polyline', layerId: 'yollar', attrs: {}, pts: [P(0, 10), P(20, 10)] },
  { kind: 'polygon', layerId: 'parsel', attrs: { Nitelik: 'Arsa' }, pts: sq(10) },
  { kind: 'polygon', layerId: 'parsel', attrs: { Nitelik: 'Tarla' }, pts: sq(20) },
  { kind: 'point', layerId: 'kotlar', attrs: { Kot: '105' }, p: P(30, 8) },
  { kind: 'polygon', layerId: 'plan', attrs: { Kullanım: 'Konut', Kat: '6' }, pts: sq(40) },
  { kind: 'polyline', layerId: 'plan', attrs: {}, pts: [P(40, 10), P(50, 10)] },
  { kind: 'text', layerId: 'yazi', attrs: {}, p: P(0, 20), text: 'Açıklama', height: 2, rotation: 0 },
  { kind: 'point', layerId: 'kendi', attrs: {}, symbol: 'agac', p: P(60, 0) },
  { kind: 'point', layerId: 'kendi', attrs: {}, symbol: 'agac', p: P(62, 0) },
  { kind: 'point', layerId: 'kendi', attrs: {}, symbol: 'silinmis', p: P(64, 0) },
  { kind: 'point', layerId: 'kendi', attrs: {}, symbol: 'adsiz', p: P(66, 0) },
  { kind: 'polyline', layerId: 'gizli', attrs: {}, pts: [P(0, 30), P(10, 30)] },
];

it.runIf(!!process.env.GOLDEN_WRITE)('records the legend', () => {
  const doc = new CadDocument({ name: 'Lejant', layers: new LayerStore(LAYERS, 'yapi'), origin: O });
  doc.load([...ENTITIES]);
  const leaves = doc.layers.leaves().map((n) => ({ id: n.id, name: n.name, style: n.style, visible: doc.layers.isVisible(n.id) }));
  const src = { entities: (id: string) => doc.byLayer(id), symbol: (r: string) => LIBRARY[r]?.symbol, itemName: (id: string) => LIBRARY[id]?.name };
  const run = (visibleOnly: boolean) => {
    const layers = legendLayers(leaves, visibleOnly);
    return { visibleOnly, layers: layers.map((l) => l.id), groups: legendOf(layers, src) };
  };
  const shown = run(true);
  const all = run(false);
  const left = new Set(['kotlar']);
  const kept = shown.groups.filter((g) => !left.has(g.layerId));
  const file = {
    format: 'kentos.style-legend',
    version: 1,
    note: 'Lejant (style/legend.ts): okuduğu katmanlar (listenin üstü önce; istenirse yalnız görünenler), her katmanın satırları (işleyicisi yoksa kendi görünüşü, tek sembol, kategoriler (kapalı kategori girmez, boş etiket yerine değer, kitaplıkta olmayan sembol girmez, Diğer değerler), aralıklar, kurallar (üst › alt, kapalı kural girmez) ve nesnelerin kendi sembolleri (her sembol bir kez, kitaplıktaki adıyla, adı yoksa kimliğiyle); yalnız katmanda olan geometriler, birden çoksa satır adında (alan), (çizgi), (nokta); satırı olmayan katman girmez), resmin yerleşimi (mantıksal piksel, 2× çizilir; başlıklı ve başlıksız; bir katman bırakılmış) ve pencerenin sözleri. Semboller kitaplıktan çözülmüş hâlleriyle yazılıdır. Yanıtlar web’indir ve kaydedilirken okunmuştur.',
    origin: O,
    layers: LAYERS,
    activeLayer: 'yapi',
    library: LIBRARY,
    entities: [...doc.all()],
    drawingName: 'Ada 101 uygulama',
    legends: [shown, all],
    layouts: [
      { headings: true, left: [], layout: legendLayout(shown.groups, true, 'Ada 101 uygulama') },
      { headings: false, left: [], layout: legendLayout(shown.groups, false, 'Ada 101 uygulama') },
      { headings: true, left: [...left], rows: kept.reduce((n, g) => n + g.entries.length, 0), layout: legendLayout(kept, true, 'Ada 101 uygulama') },
    ],
    paper: LEGEND_PAPER,
    texts: { ...LEGEND_TEXTS, rows: { n: 12, text: LEGEND_TEXTS.rows(12) } },
  };
  writeFileSync(OUT, `${JSON.stringify(file, null, 1)}\n`);
});
