import type { FillSymbol, LibraryCategory, LibraryItem, LineSymbol, MarkerSymbol } from '../../model/style';

/**
 * "Temel" system symbols: CAD line types, simple markers and area paints
 * every drawing needs. Sizes are paper millimetres at the plot scale.
 * The regulation's full legend (MPYY) lives next to this file.
 */

const ROOT = 'Temel';
const line = (id: string, name: string, sym: LineSymbol['layers'], tags: string[] = []): LibraryItem => ({ kind: 'symbol', id: `temel.cizgi.${id}`, name, path: [ROOT, 'Çizgi tipleri'], symbol: { type: 'line', layers: sym }, tags });
const marker = (id: string, name: string, sym: MarkerSymbol['layers']): LibraryItem => ({ kind: 'symbol', id: `temel.isaret.${id}`, name, path: [ROOT, 'İşaretler'], symbol: { type: 'marker', layers: sym } });
const fill = (id: string, name: string, sym: FillSymbol['layers']): LibraryItem => ({ kind: 'symbol', id: `temel.alan.${id}`, name, path: [ROOT, 'Alanlar'], symbol: { type: 'fill', layers: sym } });

const INK = 'ink';

export const BASIC_CATEGORIES: readonly LibraryCategory[] = [
  { path: [ROOT], order: 1, description: 'Her çizimde gereken çizgi tipleri, işaretler ve alan dolguları' },
  { path: [ROOT, 'Çizgi tipleri'], order: 1 },
  { path: [ROOT, 'İşaretler'], order: 2 },
  { path: [ROOT, 'Alanlar'], order: 3 },
];

export const BASIC_ITEMS: readonly LibraryItem[] = [
  line('surekli', 'Sürekli (0,25 mm)', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.25 }]),
  line('kalin', 'Kalın (0,70 mm)', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.7, cap: 'round', join: 'round' }]),
  line('kesikli', 'Kesikli', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.25, dash: [3, 1.5] }]),
  line('noktali-kesik', 'Noktalı kesik', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.25, dash: [5, 1.2, 0.6, 1.2] }]),
  line('iki-noktali-kesik', 'İki noktalı kesik', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.25, dash: [6, 1, 0.5, 1, 0.5, 1] }]),
  line('noktali', 'Noktalı', [{ id: 'l', type: 'simpleLine', color: INK, width: 0.35, dash: [0.35, 1], cap: 'butt' }]),
  line('cift', 'Çift çizgi', [
    { id: 'a', type: 'simpleLine', color: INK, width: 0.18, offset: 0.6 },
    { id: 'b', type: 'simpleLine', color: INK, width: 0.18, offset: -0.6 },
  ]),
  line('tirnakli', 'Tırnaklı (sola)', [
    { id: 'l', type: 'simpleLine', color: INK, width: 0.25 },
    { id: 't', type: 'markerLine', placement: 'interval', interval: 2, offsetAlong: 1, offset: 0.5, rotate: true, marker: { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'line', size: 1, rotation: 90, stroke: INK, strokeWidth: 0.2 }] } },
  ]),
  line('oklu', 'Yön oklu', [
    { id: 'l', type: 'simpleLine', color: INK, width: 0.25 },
    { id: 'o', type: 'markerLine', placement: 'interval', interval: 12, offsetAlong: 6, rotate: true, marker: { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'arrowhead', size: 2, fill: INK }] } },
  ]),
  line('boncuk', 'Daireli sınır', [
    { id: 'l', type: 'simpleLine', color: INK, width: 0.25 },
    { id: 'o', type: 'markerLine', placement: 'interval', interval: 6, offsetAlong: 3, marker: { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'circle', size: 1.4, fill: '#FFFFFF', stroke: INK, strokeWidth: 0.2 }] } },
  ]),
  marker('daire', 'Daire', [{ id: 'm', type: 'shape', shape: 'circle', size: 2, fill: INK }]),
  marker('halka', 'Halka ve nokta', [{ id: 'm', type: 'shape', shape: 'ring', size: 7, unit: 'px', stroke: INK, strokeWidth: 1.3 }]),
  marker('kare', 'Kare', [{ id: 'm', type: 'shape', shape: 'square', size: 2, fill: null, stroke: INK, strokeWidth: 0.2 }]),
  marker('ucgen', 'Üçgen (poligon)', [{ id: 'm', type: 'shape', shape: 'triangle', size: 3, fill: null, stroke: INK, strokeWidth: 0.25 }]),
  marker('arti', 'Artı (kot)', [{ id: 'm', type: 'shape', shape: 'cross', size: 2, stroke: INK, strokeWidth: 0.2 }]),
  marker('yildiz', 'Yıldız', [{ id: 'm', type: 'shape', shape: 'star', size: 3, fill: INK }]),
  marker('daire-arti', 'Artılı daire', [
    { id: 'c', type: 'shape', shape: 'circle', size: 2.4, fill: null, stroke: INK, strokeWidth: 0.2 },
    { id: 'p', type: 'shape', shape: 'cross', size: 2.4, stroke: INK, strokeWidth: 0.2 },
  ]),
  fill('dolu', 'Düz dolgu', [
    { id: 'f', type: 'simpleFill', color: '#9E9E9E80' },
    { id: 'o', type: 'simpleLine', color: INK, width: 0.25 },
  ]),
  fill('tarama-45', 'Tarama 45°', [
    { id: 'h', type: 'hatchFill', angle: 45, spacing: 2, width: 0.13, color: INK },
    { id: 'o', type: 'simpleLine', color: INK, width: 0.25 },
  ]),
  fill('capraz', 'Çapraz tarama', [
    { id: 'a', type: 'hatchFill', angle: 45, spacing: 2, width: 0.13, color: INK },
    { id: 'b', type: 'hatchFill', angle: 135, spacing: 2, width: 0.13, color: INK },
    { id: 'o', type: 'simpleLine', color: INK, width: 0.25 },
  ]),
  fill('noktali', 'Noktalı desen', [
    { id: 'p', type: 'patternFill', spacingX: 2, spacingY: 2, stagger: true, marker: { type: 'marker', layers: [{ id: 'd', type: 'shape', shape: 'circle', size: 0.4, fill: INK }] } },
    { id: 'o', type: 'simpleLine', color: INK, width: 0.25 },
  ]),
  fill('kenar-ici', 'İçe tırnaklı kenar', [
    { id: 'o', type: 'simpleLine', color: INK, width: 0.35 },
    { id: 't', type: 'markerLine', placement: 'interval', interval: 2, offset: 0.6, rotate: true, marker: { type: 'marker', layers: [{ id: 'm', type: 'shape', shape: 'line', size: 1.2, rotation: 90, stroke: INK, strokeWidth: 0.2 }] } },
  ]),
];
