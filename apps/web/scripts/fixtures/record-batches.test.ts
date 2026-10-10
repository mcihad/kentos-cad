// Records a styled layer's way to the GPU into fixtures/style/v1/batches.json (docs/STYLE.md §6): how each
// object is drawn (its own symbol, the layer's renderer, the layer's simple look), what the page sends the style
// core, and the GPU batches the page makes of the answer (colours from a fixed palette, atlas images, reach,
// scale ranges), in plot and screen symbol sizes and with line weights on and off. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-batches.test.ts
// The answers are the web's; rewriting them is a deliberate change of the drawing, to be read in the diff.
// src/render/styledFixture.test.ts keeps checking them; the desktop's drawing checks the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { NewEntity } from '../../src/model/entities';
import type { LayerStyle } from '../../src/model/layers';
import type { Symbol } from '../../src/model/style';
import type { ColorMode } from '../../src/render/color';
import { buildStyledLayer, viewModesOf } from '../../src/render/styledLayer';
import { ASSETS, layerDocument, PALETTE } from '../../src/style/cases';
import { batchesJson, captureStyled, decisionsOf, type StyledCall } from '../../src/style/fixture';
import { PickIndex } from '../../src/viewport/picking';
import { symbolScaleOf } from '../../src/viewport/symbolScale';

const OUT = new URL('../../../../fixtures/style/v1/batches.json', import.meta.url);

const LIBRARY: Record<string, Symbol> = {
  'cizgi-oklu': {
    type: 'line',
    layers: [
      { id: 'l', type: 'simpleLine', color: '#E15759', width: 0.5, dash: [3, 1.5] },
      { id: 'm', type: 'markerLine', placement: 'interval', interval: 10, rotate: true, marker: { type: 'marker', layers: [{ id: 's', type: 'shape', shape: 'arrowhead', size: 2, fill: '#E15759' }] } },
    ],
  },
  'alan-tarama': {
    type: 'fill',
    layers: [
      { id: 'f', type: 'simpleFill', color: '#4E79A780' },
      { id: 'h', type: 'hatchFill', angle: 45, spacing: 2, width: 0.25, color: 'fg' },
      { id: 'o', type: 'simpleLine', color: 'fg', width: 0.35 },
    ],
  },
  'alan-desen': {
    type: 'fill',
    layers: [{ id: 'p', type: 'patternFill', spacingX: 4, spacingY: 4, stagger: true, marker: { type: 'marker', layers: [{ id: 'c', type: 'shape', shape: 'circle', size: 1, fill: '#59A14F' }] } }],
  },
  'alan-doku': { type: 'fill', layers: [{ id: 'i', type: 'imageFill', asset: 'doku', tileSize: 5 }] },
  'nokta-agac': { type: 'marker', layers: [{ id: 'v', type: 'svg', asset: 'agac', size: 4, fill: '#2E7D32' }] },
  'nokta-harf': { type: 'marker', layers: [{ id: 't', type: 'text', text: 'T', size: 3, font: 'sans', weight: 700, color: 'fg', halo: { color: 'paper', width: 0.5 } }] },
};

const O = { x: 487000, y: 4420000 };
const P = (x: number, y: number) => ({ x: O.x + x, y: O.y + y });
const SIMPLE: LayerStyle = { color: 'fg', lineType: 'dashdot', lineWeight: 0.5, fill: '#E5484D33', point: { symbol: 'cross', size: 9 } };

/** A parcel, a path, a point, a text, a hatch and a dimension: every way an object can be drawn. */
const DRAWING: NewEntity[] = [
  { kind: 'polygon', layerId: 'k', attrs: { Nitelik: 'Arsa' }, pts: [P(0, 0), P(20, 0), P(20, 15), P(0, 15)] },
  { kind: 'polyline', layerId: 'k', attrs: { Nitelik: 'Yol' }, pts: [P(0, 20), P(30, 20), P(30, 35)] },
  { kind: 'point', layerId: 'k', attrs: { Nitelik: 'Ağaç' }, p: P(40, 10) },
  { kind: 'text', layerId: 'k', attrs: {}, p: P(5, 5), text: 'Etiket', height: 2, rotation: 0 },
  { kind: 'hatch', layerId: 'k', attrs: {}, ring: [P(25, 0), P(35, 0), P(35, 8), P(25, 8)], pattern: { type: 'lines', angle: 30, spacing: 1.5 } },
  { kind: 'dimension', layerId: 'k', attrs: {}, a: P(0, -5), b: P(20, -5), offset: 2, height: 1.5 },
];

/** The same drawing with objects that carry their own symbols and a colour of their own. */
const OWN: NewEntity[] = [
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-tarama', pts: [P(0, 0), P(20, 0), P(20, 15), P(0, 15)] },
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-desen', pts: [P(25, 0), P(45, 0), P(45, 15), P(25, 15)] },
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-doku', pts: [P(50, 0), P(60, 0), P(60, 10), P(50, 10)] },
  { kind: 'polyline', layerId: 'k', attrs: {}, symbol: 'cizgi-oklu', pts: [P(0, 20), P(30, 20), P(30, 35)] },
  { kind: 'point', layerId: 'k', attrs: {}, symbol: 'nokta-agac', p: P(40, 25) },
  { kind: 'point', layerId: 'k', attrs: {}, symbol: 'nokta-harf', p: P(45, 25) },
  { kind: 'point', layerId: 'k', attrs: {}, color: '#F28E2B', p: P(50, 25) },
];

/**
 * Objects with their own line weights (docs/adr/0139): the thinnest, DXF's heaviest, one with its own colour too,
 * and one without (the layer's). Each is drawn in its layer's simple look at its own weight.
 */
const WEIGHTS: NewEntity[] = [
  { kind: 'polyline', layerId: 'k', attrs: {}, lineWeight: 0, pts: [P(0, 0), P(20, 0)] },
  { kind: 'polyline', layerId: 'k', attrs: {}, lineWeight: 2.11, pts: [P(0, 5), P(20, 5)] },
  { kind: 'polygon', layerId: 'k', attrs: {}, lineWeight: 0.25, color: '#F28E2B', pts: [P(0, 10), P(10, 10), P(10, 20)] },
  { kind: 'polyline', layerId: 'k', attrs: {}, pts: [P(0, 25), P(20, 25)] },
];

/**
 * Objects at the anchor and the same ones 4 400 km from it, in a local survey's coordinates (docs/adr/0157): the far
 * objects make batches of their own, packed from their tile, the world paints' phases (hatch, pattern, tile) folded.
 */
const L = (x: number, y: number) => ({ x: 1000 + x, y: 2000 + y });
const FAR: NewEntity[] = [
  ...OWN,
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-tarama', pts: [L(0, 0), L(20, 0), L(20, 15), L(0, 15)] },
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-desen', pts: [L(25, 0), L(45, 0), L(45, 15), L(25, 15)] },
  { kind: 'polygon', layerId: 'k', attrs: {}, symbol: 'alan-doku', pts: [L(50, 0), L(60, 0), L(60, 10), L(50, 10)] },
  { kind: 'polyline', layerId: 'k', attrs: {}, symbol: 'cizgi-oklu', pts: [L(0, 20), L(30, 20), L(30, 35)] },
  { kind: 'point', layerId: 'k', attrs: {}, symbol: 'nokta-agac', p: L(40, 25) },
  { kind: 'point', layerId: 'k', attrs: {}, p: L(50, 25) },
];

const CATEGORIZED: LayerStyle = {
  ...SIMPLE,
  renderer: {
    type: 'categorized',
    expr: 'Nitelik',
    categories: [
      { value: 'Arsa', label: 'Arsa', symbols: { fill: { ref: 'alan-tarama' } } },
      { value: 'Yol', label: 'Yol', symbols: { line: { ref: 'cizgi-oklu' } } },
    ],
    other: { marker: { ref: 'nokta-harf' }, line: { type: 'line', layers: [{ id: 'o', type: 'simpleLine', color: 'fgDim', width: 0.18 }] } },
  },
};

/** Arsa switched off: its parcel draws nothing, and does not fall to the others either (as in QGIS). */
const CATEGORIZED_OFF: LayerStyle = {
  ...SIMPLE,
  renderer: {
    type: 'categorized',
    expr: 'Nitelik',
    categories: [
      { value: 'Arsa', label: 'Arsa', symbols: { fill: { ref: 'alan-tarama' } }, enabled: false },
      { value: 'Yol', label: 'Yol', symbols: { line: { ref: 'cizgi-oklu' } } },
    ],
    other: { marker: { ref: 'nokta-harf' }, line: { type: 'line', layers: [{ id: 'o', type: 'simpleLine', color: 'fgDim', width: 0.18 }] } },
  },
};

const RULES: LayerStyle = {
  ...SIMPLE,
  renderer: {
    type: 'rules',
    rules: [
      { id: 'r1', label: 'Yakın', filter: "Nitelik = 'Arsa'", maxScale: 2000, symbols: { fill: { ref: 'alan-desen' } } },
      { id: 'r2', label: 'Uzak', filter: "Nitelik = 'Arsa'", minScale: 2000, symbols: { fill: { type: 'fill', layers: [{ id: 's', type: 'simpleFill', color: '#9C755F' }] } } },
      { id: 'r3', label: 'Diğerleri', isElse: true, symbols: { line: { ref: 'cizgi-oklu' }, marker: { ref: 'nokta-agac' } } },
    ],
  },
};

/** Districts with their numbers, two stops on one place and a road: what the thematic renderers read (docs/adr/0213). */
const THEMATIC: NewEntity[] = [
  { kind: 'polygon', layerId: 'k', attrs: { Nüfus: '120', Gelir: '20', Erkek: '600', Kadın: '640', Konut: '70', Ticaret: '30' }, pts: [P(0, 0), P(20, 0), P(20, 15), P(0, 15)] },
  { kind: 'polygon', layerId: 'k', attrs: { Nüfus: '40', Gelir: '45', Erkek: '200', Kadın: '260', Konut: '20', Ticaret: '60' }, pts: [P(25, 0), P(45, 0), P(45, 15), P(25, 15)] },
  // No number: “Değeri olmayanlar”.
  { kind: 'polygon', layerId: 'k', attrs: { Gelir: '10' }, pts: [P(50, 0), P(60, 0), P(60, 10), P(50, 10)] },
  { kind: 'polyline', layerId: 'k', attrs: { Nüfus: '80' }, pts: [P(0, 20), P(30, 20), P(30, 35)] },
  { kind: 'point', layerId: 'k', attrs: { Nüfus: '150', Hat: '12' }, p: P(40, 25) },
  { kind: 'point', layerId: 'k', attrs: { Nüfus: '20', Hat: '14' }, p: P(40.5, 25) },
];

const MAIN_FILL: Symbol = { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#000000' }, { id: 'l', type: 'simpleLine', color: '#FFFFFF', width: 0.3 }] };
const MAIN_LINE: Symbol = { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color: '#000000', width: 0.5 }] };
const CIRCLE: Symbol = { type: 'marker', layers: [{ id: 'c', type: 'shape', shape: 'circle', size: 3, fill: '#000000', stroke: '#FFFFFF', strokeWidth: 0.2 }] };
const GROUND: Symbol = { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#F2F2F2' }, { id: 'l', type: 'simpleLine', color: '#B0B0B0', width: 0.25 }] };
const thematic = (renderer: LayerStyle['renderer']): LayerStyle => ({ ...SIMPLE, renderer });

const UNCLASSED = thematic({ type: 'unclassed', expr: 'Nüfus', min: 0, max: 200, ramp: ['#FFF5B8', '#E66101', '#A50F15'], symbols: { fill: MAIN_FILL, line: MAIN_LINE, marker: CIRCLE }, other: { fill: GROUND } });
const PROPORTIONAL = thematic({ type: 'proportional', expr: 'Nüfus', minValue: 0, maxValue: 200, minSize: 1, maxSize: 6, unit: 'mm', scaling: 'flannery', symbols: { marker: CIRCLE, line: MAIN_LINE, fill: GROUND } });
const BIVARIATE = thematic({ type: 'bivariate', exprX: 'Nüfus', exprY: 'Gelir', breaksX: [100], breaksY: [30], colors: ['#E8E8E8', '#5AC8C8', '#BE64AC', '#3B4994'], symbols: { fill: MAIN_FILL } });
const DOTS = thematic({ type: 'dotDensity', fields: [{ expr: 'Erkek', color: '#4E79A7' }, { expr: 'Kadın', color: '#E15759' }], dotValue: 50, dotSize: 0.6, unit: 'mm', seed: 3, symbols: { fill: GROUND } });
const PIES = thematic({
  type: 'chart',
  kind: 'pie',
  fields: [{ expr: 'Konut', color: '#E15759' }, { expr: 'Ticaret', color: '#4E79A7' }],
  size: 6,
  unit: 'mm',
  sizeBy: { minValue: 50, maxValue: 100, minSize: 4, maxSize: 8 },
  outline: { color: '#FFFFFF', width: 0.2 },
  symbols: { fill: GROUND },
});
const BARS = thematic({ type: 'chart', kind: 'stacked', fields: [{ expr: 'Konut', color: '#E15759' }, { expr: 'Ticaret', color: '#4E79A7' }], size: 8, unit: 'mm', maxValue: 100, barWidth: 1.5 });
const CLUSTER = thematic({
  type: 'cluster',
  distance: 30,
  unit: 'px',
  grow: true,
  renderer: { type: 'categorized', expr: 'Hat', categories: [{ value: '12', label: '12', symbols: { marker: CIRCLE } }], other: { marker: { ref: 'nokta-harf' } } },
});
const DISPLACEMENT = thematic({ type: 'displacement', tolerance: 4, unit: 'px', placement: 'rings', spacing: 1, center: CIRCLE, circle: { color: '#7D7D7D', width: 1 }, renderer: { type: 'single', symbols: { marker: { ref: 'nokta-agac' } } } });
const INVERTED = thematic({ type: 'inverted', symbols: { fill: { type: 'fill', layers: [{ id: 'f', type: 'simpleFill', color: '#FFFFFFB3' }, { id: 'l', type: 'simpleLine', color: '#8E4EC6', width: 0.6 }] } }, merge: true });
/** Ters alan's box (absolute): the construction lines' box of a view round the districts. */
const CLIP = { minX: O.x - 40, minY: O.y - 40, maxX: O.x + 100, maxY: O.y + 75 };

interface Input {
  id: string;
  title: string;
  style: LayerStyle;
  entities: NewEntity[];
  plotScale: number;
  /** The box construction lines (and Ters alan) are drawn to, absolute; none where a case gives none. */
  clip?: { minX: number; minY: number; maxX: number; maxY: number };
  /** Görünüm kipleri (docs/adr/0195) where a case gives them: Renkli, with fills, edges and transparency otherwise. */
  view: { symbolSize: 'plot' | 'screen'; pxPerM: number; lineWeights: boolean; colorMode?: ColorMode; fills?: boolean; areaEdges?: boolean; transparency?: boolean };
}

const INPUTS: Input[] = [
  { id: 'simple-plot', title: 'Katmanın düz görünüşü, çizim ölçeğinde, kalınlıklar açık', style: SIMPLE, entities: DRAWING, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'simple-hairlines', title: 'Katmanın düz görünüşü, kalınlıklar kapalı: çizgiler bir piksel', style: SIMPLE, entities: DRAWING, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: false } },
  { id: 'simple-screen-4', title: 'Ekranda sabit semboller, 4 px/m: ölçek 1024', style: SIMPLE, entities: DRAWING, plotScale: 1000, view: { symbolSize: 'screen', pxPerM: 4, lineWeights: true } },
  { id: 'simple-screen-12', title: 'Ekranda sabit semboller, 12 px/m: ölçek çeyrek oktav adımına yuvarlanır', style: SIMPLE, entities: DRAWING, plotScale: 1000, view: { symbolSize: 'screen', pxPerM: 12, lineWeights: true } },
  { id: 'own-symbols', title: 'Kendi sembolü olan nesneler: tarama, desen, doku, oklu çizgi, SVG ve yazı işaretleri; kendi rengi olan nokta', style: SIMPLE, entities: OWN, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'own-symbols-screen', title: 'Kendi sembolü olan nesneler, ekranda sabit, 12 px/m', style: SIMPLE, entities: OWN, plotScale: 1000, view: { symbolSize: 'screen', pxPerM: 12, lineWeights: true } },
  { id: 'categorized', title: 'Kategorili işleyici: Arsa taralı, Yol oklu, diğerleri harf ve ince çizgi', style: CATEGORIZED, entities: DRAWING, plotScale: 500, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'rules-scale', title: 'Kurallı işleyici: ölçek aralıklı iki kural ve değilse kuralı', style: RULES, entities: DRAWING, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: false } },
  { id: 'categorized-off', title: 'Kategorili işleyici, Arsa kapalı: Arsa çizilmez, diğer değerlere de düşmez', style: CATEGORIZED_OFF, entities: DRAWING, plotScale: 500, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'own-weights', title: 'Kendi kalınlığı olan nesneler: en ince, DXF’in en kalını, kendi rengiyle biri ve kalınlığı olmayan (katmanınki)', style: SIMPLE, entities: WEIGHTS, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'own-weights-hairlines', title: 'Kendi kalınlığı olan nesneler, kalınlıklar kapalı: hepsi bir piksel', style: SIMPLE, entities: WEIGHTS, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: false } },
  { id: 'view-mono', title: 'Tek renk (ADR 0195): her renk temanın mürekkebi, saydamlıklar ve yazının halesi kalır', style: SIMPLE, entities: OWN, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true, colorMode: 'mono' } },
  { id: 'view-gray', title: 'Gri (ADR 0195): her renk kendi parlaklığında, saydamlıklar kalır', style: SIMPLE, entities: OWN, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true, colorMode: 'gray' } },
  { id: 'view-no-fills', title: 'Dolgular ve taramalar kapalı (ADR 0195): alanların dolguları, taramalar ve desenler çizilmez; çizgiler ve işaretler çizilir', style: SIMPLE, entities: [...DRAWING, ...OWN], plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true, fills: false } },
  { id: 'view-no-edges', title: 'Alan sınırları kapalı (ADR 0195): alanların çizgileri çizilmez, dolguları çizilir; çoklu çizgi kalır', style: SIMPLE, entities: [...DRAWING, ...OWN], plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true, areaEdges: false } },
  { id: 'view-opaque', title: 'Saydamlık kapalı (ADR 0195): yarı saydam dolgular tam örtücü', style: SIMPLE, entities: [...DRAWING, ...OWN], plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true, transparency: false } },
  { id: 'far-tile', title: 'Çapada ve 4 400 km ötede, yerel koordinatlarda aynı nesneler: uzaktakiler kendi karolarının toplulukları, dünyaya bağlı desenlerin evresi katlanmış', style: SIMPLE, entities: FAR, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  // The thematic renderers (docs/adr/0213 §2): the same page call and batches on both platforms; a cluster's and a
  // displacement's pixels as paper at the plot scale (no view given), Ters alan over its box.
  { id: 'unclassed', title: 'Sürekli renk: rampadan renkler, değeri olmayan alan zeminiyle', style: UNCLASSED, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'proportional', title: 'Orantılı sembol: Flannery, noktada işaret, çizgide kalınlık, alanın iç noktasında işaret', style: PROPORTIONAL, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'bivariate', title: 'İki değişkenli renk: 2 × 2 ızgara', style: BIVARIATE, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'dot-density', title: 'Nokta yoğunluğu: iki değerin noktaları, tohum 3, zemin', style: DOTS, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'chart-pie', title: 'Grafik: toplama göre büyüyen pasta, çerçeveli', style: PIES, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'chart-stacked', title: 'Grafik: yığılmış çubuk', style: BARS, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'cluster', title: 'Kümeleme: büyüyen küme işareti, tek noktalar Kategorili', style: CLUSTER, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'displacement', title: 'Yayma: iç içe halkalar, merkez ve halka, tek noktalar Tek sembol', style: DISPLACEMENT, entities: THEMATIC, plotScale: 1000, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
  { id: 'inverted', title: 'Ters alan: kutunun alanların dışı, örtüşenler boş', style: INVERTED, entities: THEMATIC, plotScale: 1000, clip: CLIP, view: { symbolSize: 'plot', pxPerM: 4, lineWeights: true } },
];

it.runIf(!!process.env.GOLDEN_WRITE)('records the styled layers’ way to the GPU', () => {
  const cases = INPUTS.map((c) => {
    const doc = layerDocument('Kadastro', c.style, c.entities);
    const entities = [...doc.all()];
    const symbolScale = symbolScaleOf(c.view.symbolSize, c.plotScale, c.view.pxPerM);
    let call: StyledCall | null = null;
    const layer = buildStyledLayer('k', entities, doc.layers.get('k')!.style, {
      origin: O,
      palette: PALETTE,
      plotScale: symbolScale,
      screen: c.view.symbolSize === 'screen',
      hairlines: !c.view.lineWeights,
      view: viewModesOf(c.view),
      ...(c.clip && { clip: c.clip }),
      library: { symbol: (id) => LIBRARY[id], asset: (id) => ASSETS.find((a) => a.id === id) },
      layerName: (id) => doc.layers.get(id)?.name ?? id,
      geometry: captureStyled(new PickIndex(doc), (x) => (call = x)),
    });
    const got = call as StyledCall | null;
    if (!got) throw new Error(`${c.id}: the core was not called`);
    return {
      ...c,
      entities,
      expect: {
        symbolScale,
        decisions: decisionsOf(
          entities.map((e) => e.id),
          got.objects,
        ),
        program: JSON.parse(got.program) as unknown,
        objects: got.objects,
        table: got.table,
        batches: batchesJson(layer.styled ?? []),
      },
    };
  });
  const file = {
    format: 'kentos.style-batches',
    version: 1,
    note: 'Stilli bir katmanın GPU’ya yolu (docs/STYLE.md §6): her nesnenin nasıl çizildiği (kendi sembolü, katmanın işleyicisi ya da düz görünüşü; yazı çizilmez, ölçü kendi ince çizgileriyle), sayfanın stil çekirdeğine verdikleri (program, nesne başına dört sayı: kip, küme ya da sembol, düz görünüş kümesi, renk; ifade tablosu) ve sayfanın çekirdeğin yanıtından yaptığı GPU toplulukları (sabit bir paletten renkler, atlas görüntüleri, erişim, ölçek aralığı; float32 sayılar). Sembol boyu çizim ölçeğinde ya da ekranda sabit (symbolScale: ekran ölçeği paydası, çeyrek oktav adımına yuvarlanır), kalınlıklar açık ya da kapalı. Yanıtlar web’indir; doğruluklarını stil çekirdeğinin testleri ve cases.json taşır, burada iki platformun aynı toplulukları kurması sabitlenir. Koordinatlar mutlaktır; origin topluluk sayılarının göreli olduğu noktadır.',
    palette: PALETTE,
    assets: ASSETS,
    library: LIBRARY,
    origin: O,
    layer: { id: 'k', name: 'Kadastro' },
    cases,
  };
  writeFileSync(OUT, `${JSON.stringify(file, null, 1)}\n`);
});
