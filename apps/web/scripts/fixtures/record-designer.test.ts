// Records the symbol designer's model into fixtures/style/v1/designer.json (ui/style/designerModel.ts): the
// layer types each kind of symbol takes and their names, how a new layer starts, layers' summaries, form
// patches turned into layers, the layer list's edits (add, move, duplicate, remove, switch off), new drafts,
// the slots' starting symbols, titles, saved names, the preview's scale and the window's words. Runs only on
// purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-designer.test.ts
// The answers are the web's and were read when recorded; rewriting them is a deliberate change, to be read in
// the diff. src/ui/style/designerFixture.test.ts keeps checking them; the desktop's designer
// (crates/native/style/tests/all/designer.rs) checks the same file.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import type { MarkerLayer, Symbol } from '../../src/model/style';
import {
  addLayer,
  addParent,
  ANCHORS,
  applyPatch,
  canMove,
  canRemove,
  CAPS,
  countSuffix,
  defaultFor,
  DESIGNER_TEXTS,
  designerTitle,
  duplicateLayer,
  FONTS,
  GEOMETRIES,
  KIND_TITLE,
  LAYER_LABEL,
  LAYER_TYPES,
  layerAt,
  moveLayer,
  newDraft,
  newLayer,
  OPEN_SHAPES,
  pathOf,
  PLACEMENTS,
  POSITIONS,
  putLayer,
  removeLayer,
  RINGS,
  savedAs,
  setEnabled,
  SHAPES,
  summary,
  uid,
  UNITS,
  WAVES,
  WEIGHTS,
  ZOOM,
  zoomed,
  zoomText,
  type AnyLayer,
  type LayerPath,
  type LayerType,
  type Patch,
  type SymbolKind,
} from '../../src/ui/style/designerModel';

const OUT = new URL('../../../../fixtures/style/v1/designer.json', import.meta.url);

const KINDS: SymbolKind[] = ['fill', 'line', 'marker'];

/** Layers whose summaries are recorded: every type, its units, and what each summary looks at. */
const SUMMARIES: AnyLayer[] = [
  { id: '0', type: 'simpleFill', color: '#C9D6E3' },
  { id: '0', type: 'simpleFill', color: { expr: 'Renk', fallback: '#FF0000' } },
  { id: '0', type: 'hatchFill', angle: 45, spacing: 2, width: 0.2, color: 'ink' },
  { id: '0', type: 'hatchFill', angle: 22.5, spacing: 1.255, width: 0.2, color: 'ink', unit: 'px' },
  { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3.333, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3, stagger: true, jitter: 0.4, unit: 'm', marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3, stagger: false, jitter: 0, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'imageFill', asset: '', tileSize: 5 },
  { id: '0', type: 'imageFill', asset: 'a-1', tileSize: 7.5 },
  { id: '0', type: 'centroidMarker', marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'centroidMarker', position: 'centroid', marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 },
  { id: '0', type: 'simpleLine', color: 'ink', width: { expr: '[Genişlik] / 2' }, dash: [4, 1.5], offset: 0.5, wave: { shape: 'sine', length: 5, amplitude: 0.8 } },
  { id: '0', type: 'simpleLine', color: 'ink', width: 0.25, dash: [], offset: { expr: '[Genişlik] / 2', fallback: 1 }, unit: 'm' },
  { id: '0', type: 'simpleLine', color: 'ink', width: 0.25, offset: 0 },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 6, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 1, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 2, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 3, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 6, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 9, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  { id: '0', type: 'markerLine', placement: 'interval', interval: 4, group: { count: 10, spacing: 1 }, marker: { type: 'marker', layers: [] } },
  ...PLACEMENTS.filter((p) => p.value !== 'interval').map((p): AnyLayer => ({ id: '0', type: 'markerLine', placement: p.value, marker: { type: 'marker', layers: [] } })),
  ...SHAPES.map((s): AnyLayer => ({ id: '0', type: 'shape', shape: s.value, size: 3 })),
  { id: '0', type: 'shape', shape: 'circle', size: { expr: 'Boy', fallback: 3 }, unit: 'px' },
  { id: '0', type: 'svg', asset: '', size: 5 },
  { id: '0', type: 'svg', asset: 'a-1', size: 4.25 },
  { id: '0', type: 'raster', asset: '', size: 5 },
  { id: '0', type: 'raster', asset: 'a-2', size: 12, unit: 'm' },
  { id: '0', type: 'text', text: 'A', size: 3 },
  { id: '0', type: 'text', text: { expr: 'etiket', fallback: 'A' }, size: 3 },
];

/** Form patches and the layers they are made on: every helper key, and the values that remove a field. */
const PATCHES: { layer: AnyLayer; patch: Patch }[] = [
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }, patch: { width: 0.5 } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }, patch: { shiftX: 0.4 } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35, shift: [0.4, -0.2] }, patch: { shiftY: 0.1 } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35, shift: [0.4, 0] }, patch: { shiftX: 0 } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35, dash: [4, 1] }, patch: { dash: null } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }, patch: { dash: [4, 1.5, 0.5, 1.5] } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }, patch: { wave: { shape: 'zigzag', length: 5, amplitude: 0.8, connect: true } } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35, wave: { shape: 'sine', length: 5, amplitude: 0.8 } }, patch: { wave: undefined } },
  { layer: { id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }, patch: { offset: { expr: '[Genişlik] / 2', fallback: 0 } } },
  { layer: { id: '0', type: 'shape', shape: 'circle', size: 3 }, patch: { offsetX: 1.5 } },
  { layer: { id: '0', type: 'shape', shape: 'circle', size: 3, offset: [1, 2] }, patch: { offsetY: -0.5 } },
  { layer: { id: '0', type: 'shape', shape: 'gear', size: 3 }, patch: { holePct: 35 } },
  { layer: { id: '0', type: 'shape', shape: 'gear', size: 3 }, patch: { teethDepthPct: 25 } },
  { layer: { id: '0', type: 'shape', shape: 'circle', size: 3, fill: 'ink' }, patch: { fill: null } },
  { layer: { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3, marker: { type: 'marker', layers: [] } }, patch: { jitterPct: 40 } },
  { layer: { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3, marker: { type: 'marker', layers: [] } }, patch: { coveragePct: 55 } },
  { layer: { id: '0', type: 'patternFill', spacingX: 3, spacingY: 3, offset: [0.5, 0.25], marker: { type: 'marker', layers: [] } }, patch: { offsetX: 1 } },
  { layer: { id: '0', type: 'markerLine', placement: 'interval', interval: 6, marker: { type: 'marker', layers: [] } }, patch: { groupCount: 3 } },
  { layer: { id: '0', type: 'markerLine', placement: 'interval', interval: 6, group: { count: 3, spacing: 1 }, marker: { type: 'marker', layers: [] } }, patch: { groupSpacing: 0.75 } },
  { layer: { id: '0', type: 'markerLine', placement: 'interval', interval: 6, group: { count: 3, spacing: 1 }, marker: { type: 'marker', layers: [] } }, patch: { groupCount: 1 } },
  { layer: { id: '0', type: 'markerLine', placement: 'interval', interval: 6, marker: { type: 'marker', layers: [] } }, patch: { groupCount: 2.5 } },
  { layer: { id: '0', type: 'text', text: 'A', size: 3, halo: { color: '#FFFFFF', width: 0.3 } }, patch: { haloWidth: 0.5 } },
  { layer: { id: '0', type: 'text', text: 'A', size: 3, halo: null }, patch: { haloWidth: 0.5 } },
  { layer: { id: '0', type: 'text', text: 'A', size: 3 }, patch: { haloWidth: 0.5 } },
  { layer: { id: '0', type: 'text', text: 'A', size: 3 }, patch: { halo: { color: '#FFFFFF', width: 0.3 } } },
  { layer: { id: '0', type: 'text', text: 'A', size: 3, italic: true }, patch: { italic: false, weight: 900 } },
  { layer: { id: '0', type: 'hatchFill', angle: 45, spacing: 2, width: 0.2, color: 'ink', dash: [1, 1] }, patch: { dash: null, dashOffset: 0.5 } },
  { layer: { id: '0', type: 'simpleFill', color: '#C9D6E3', opacity: 0.5 }, patch: { opacity: 1, unit: 'px' } },
  { layer: { id: '0', type: 'simpleFill', color: '#C9D6E3', enabled: false }, patch: { enabled: { expr: "[Nitelik] = 'Arsa'", fallback: true } } },
];

const dotMarker = (id: string): MarkerLayer => ({ id, type: 'shape', shape: 'circle', size: 1, fill: 'ink' });

/** Symbols the list's edits start from. */
const AREA: Symbol = {
  type: 'fill',
  layers: [
    { id: '0', type: 'simpleFill', color: '#C9D6E3' },
    { id: '1', type: 'patternFill', spacingX: 3, spacingY: 3, marker: { type: 'marker', layers: [dotMarker('0'), { id: '1', type: 'text', text: 'A', size: 3 }] } },
    { id: '2', type: 'simpleLine', color: 'ink', width: 0.25, enabled: false },
  ],
};
const ONE: Symbol = { type: 'line', layers: [{ id: '0', type: 'simpleLine', color: 'ink', width: 0.35 }] };
const GAPS: Symbol = { type: 'marker', layers: [dotMarker('0'), { id: '2', type: 'text', text: 'B', size: 2 }] };
const LONELY: Symbol = { type: 'line', layers: [{ id: '0', type: 'markerLine', placement: 'first', marker: { type: 'marker', layers: [dotMarker('7')] } }] };

type Op =
  | { op: 'add'; type: LayerType; parent: number | null }
  | { op: 'move'; delta: number }
  | { op: 'duplicate' }
  | { op: 'remove' }
  | { op: 'enable'; at: LayerPath; on: boolean }
  | { op: 'put'; at: LayerPath; layer: AnyLayer };

const EDITS: { name: string; symbol: Symbol; selected: LayerPath; op: Op }[] = [
  { name: 'a layer added to the symbol', symbol: AREA, selected: [0], op: { op: 'add', type: 'hatchFill', parent: null } },
  { name: 'a layer added to a marker', symbol: AREA, selected: [1], op: { op: 'add', type: 'raster', parent: 1 } },
  { name: 'a line layer of an area takes the area’s width', symbol: AREA, selected: [2], op: { op: 'add', type: 'simpleLine', parent: null } },
  { name: 'a line symbol’s own line', symbol: ONE, selected: [0], op: { op: 'add', type: 'simpleLine', parent: null } },
  { name: 'an id past the gaps', symbol: GAPS, selected: [1], op: { op: 'add', type: 'shape', parent: null } },
  { name: 'moved up', symbol: AREA, selected: [2], op: { op: 'move', delta: -1 } },
  { name: 'moved down', symbol: AREA, selected: [0], op: { op: 'move', delta: 1 } },
  { name: 'not above the top', symbol: AREA, selected: [0], op: { op: 'move', delta: -1 } },
  { name: 'not below the bottom', symbol: AREA, selected: [2], op: { op: 'move', delta: 1 } },
  { name: 'a marker’s layer moved down', symbol: AREA, selected: [1, 0], op: { op: 'move', delta: 1 } },
  { name: 'duplicated', symbol: AREA, selected: [0], op: { op: 'duplicate' } },
  { name: 'a marker’s layer duplicated', symbol: AREA, selected: [1, 1], op: { op: 'duplicate' } },
  { name: 'removed, the next one chosen', symbol: AREA, selected: [1], op: { op: 'remove' } },
  { name: 'the last removed, the one before chosen', symbol: AREA, selected: [2], op: { op: 'remove' } },
  { name: 'a symbol keeps one layer', symbol: ONE, selected: [0], op: { op: 'remove' } },
  { name: 'a marker’s layer removed', symbol: AREA, selected: [1, 0], op: { op: 'remove' } },
  { name: 'a marker’s only layer removed, its parent chosen', symbol: LONELY, selected: [0, 0], op: { op: 'remove' } },
  { name: 'switched on', symbol: AREA, selected: [2], op: { op: 'enable', at: [2], on: true } },
  { name: 'a marker’s layer switched off', symbol: AREA, selected: [1, 1], op: { op: 'enable', at: [1, 1], on: false } },
  { name: 'a marker’s layer replaced', symbol: AREA, selected: [1, 0], op: { op: 'put', at: [1, 0], layer: { id: '0', type: 'shape', shape: 'star', size: 2, fill: '#E15759' } } },
];

function run(symbol: Symbol, selected: LayerPath, op: Op): { symbol: Symbol; selected: LayerPath } | null {
  switch (op.op) {
    case 'add':
      return addLayer(symbol, op.type, op.parent);
    case 'move':
      return moveLayer(symbol, selected, op.delta);
    case 'duplicate':
      return duplicateLayer(symbol, selected);
    case 'remove':
      return removeLayer(symbol, selected);
    case 'enable':
      return { symbol: setEnabled(symbol, op.at, op.on), selected };
    case 'put':
      return { symbol: putLayer(symbol, op.at, op.layer), selected };
  }
}

it.runIf(!!process.env.GOLDEN_WRITE)('records the designer', () => {
  const plain = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;
  const file = {
    format: 'kentos.style-designer',
    version: 1,
    labels: LAYER_LABEL,
    layerTypes: LAYER_TYPES,
    kindTitles: KIND_TITLE,
    geometries: GEOMETRIES,
    options: { units: UNITS, shapes: SHAPES, openShapes: [...OPEN_SHAPES], placements: PLACEMENTS, anchors: ANCHORS, fonts: FONTS, weights: WEIGHTS, caps: CAPS, rings: RINGS, waves: WAVES, positions: POSITIONS },
    newLayers: KINDS.flatMap((context) => Object.keys(LAYER_LABEL).map((type) => ({ type, id: '3', context, layer: newLayer(type as LayerType, '3', context) }))),
    summaries: SUMMARIES.map((layer) => ({ layer, summary: summary(layer) })),
    countSuffixes: [...Array.from({ length: 20 }, (_, i) => i + 1), 30, 40, 50, 60, 70, 80, 90, 100, 1000, 1100, 2000].map((n) => [n, countSuffix(n)]),
    // A key set to undefined leaves the layer (JSON has no undefined): listed apart as `unset`.
    patches: PATCHES.map(({ layer, patch }) => ({ layer, patch: plain(patch), unset: Object.keys(patch).filter((k) => patch[k] === undefined), result: plain(applyPatch(layer, patch)) })),
    uids: [[], ['0'], ['1'], ['0', '1'], ['0', '2'], ['1', '2', '3']].map((ids) => ({ ids, uid: uid(ids.map((id) => ({ id, type: 'simpleFill', color: 'ink' }))) })),
    edits: EDITS.map((e) => ({
      name: e.name,
      symbol: e.symbol,
      selected: e.selected,
      layer: layerAt(e.symbol, e.selected) ?? null,
      addParent: addParent(e.symbol, e.selected),
      canMoveUp: canMove(e.symbol, e.selected, -1),
      canMoveDown: canMove(e.symbol, e.selected, 1),
      canRemove: canRemove(e.symbol, e.selected),
      op: e.op,
      result: plain(run(e.symbol, e.selected, e.op)),
    })),
    drafts: [
      ...KINDS.map((kind) => ({ kind, path: null, draft: newDraft(kind) })),
      { kind: 'fill', path: ['Proje', 'Alanlar'], draft: newDraft('fill', ['Proje', 'Alanlar']) },
    ],
    defaults: { fill: defaultFor('fill'), line: defaultFor('line'), marker: defaultFor('marker') },
    titles: KINDS.flatMap((kind) => [
      { kind, inline: null, dirty: false, title: designerTitle(kind, null, false) },
      { kind, inline: null, dirty: true, title: designerTitle(kind, null, true) },
      { kind, inline: 'Tek sembol (alan)', dirty: false, title: designerTitle(kind, 'Tek sembol (alan)', false) },
      { kind, inline: 'Konut (nokta)', dirty: true, title: designerTitle(kind, 'Konut (nokta)', true) },
    ]),
    saved: [
      { name: 'Bahçe alanı', path: ['Sembollerim', 'Alanlar'] },
      { name: '  ', path: [] },
      { name: '  Yol ', path: ['Ulaşım'] },
    ].map((d) => ({ name: d.name, path: d.path, as: savedAs({ ...d, symbol: ONE }) })),
    paths: ['Sembollerim', ' Ana / Alt ', 'Ana//Alt/', '', ' / '].map((text) => ({ text, path: pathOf(text) })),
    zoom: {
      ...ZOOM,
      steps: [
        [4, ZOOM.step],
        [4, 1 / ZOOM.step],
        [39, ZOOM.step],
        [1.1, 1 / ZOOM.step],
        [ZOOM.real, ZOOM.step],
      ].map(([px, factor]) => ({ px, factor, result: zoomed(px, factor) })),
      texts: [4, ZOOM.real, 1, 40, 12.2070312, 3.2768].map((px) => ({ px, text: zoomText(px) })),
    },
    texts: {
      ...DESIGNER_TEXTS,
      intoMarker: { parent: 'Desen', text: DESIGNER_TEXTS.intoMarker('Desen') },
      inMarker: { parent: 'Çizgi boyunca işaret', text: DESIGNER_TEXTS.inMarker('Çizgi boyunca işaret') },
      notSaved: [
        { first: 'sembol › katman 1: renk geçerli bir renk değil', more: 0, text: DESIGNER_TEXTS.notSaved('sembol › katman 1: renk geçerli bir renk değil', 0) },
        { first: 'sembol › katman 2: kimlik yok', more: 2, text: DESIGNER_TEXTS.notSaved('sembol › katman 2: kimlik yok', 2) },
      ],
      saved: { name: 'Bahçe alanı', text: DESIGNER_TEXTS.saved('Bahçe alanı') },
    },
  };
  writeFileSync(OUT, `${JSON.stringify(file, null, 1)}\n`);
});
