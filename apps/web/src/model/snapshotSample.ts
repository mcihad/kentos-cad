import { CadDocument } from './document';
import { LayerStore } from './layers';
import type { NewEntity } from './entities';

/**
 * A small drawing with every object kind, TM36 coordinates, bulges, holes
 * and a project symbol: the round-trip sample for `.kcad` (snapshot.test.ts)
 * and, recorded as fixtures/document/v1/sample.json, for the Rust contracts.
 * Not used by the app.
 */
export function snapshotSampleDocument(): CadDocument {
  const E = 486512.34;
  const N = 4420187.52;
  const layers = new LayerStore(
    [
      { id: 'layer-g', name: 'Kadastro', type: 'group', children: [
        { id: 'parsel', name: 'Parsel', style: { color: '#E06C75', lineType: 'continuous', lineWeight: 0.35, pickInterior: true, label: { placement: 'center', size: 12, weight: 600 } } },
        { id: 'bina', name: 'Bina', locked: true, style: { color: 'ink', lineType: 'dashed', lineWeight: 0.25, fill: '#7FB2E526' } },
      ] },
      { id: 'cizim', name: 'Çizim', visible: false, style: { color: 'fg', lineType: 'dashdot', lineWeight: 0.18, point: { symbol: 'cross', size: 8 } } },
    ],
    'parsel',
  );
  const doc = new CadDocument({ name: 'Örnek pafta.kcad', layers, origin: { x: E, y: N }, settings: { srid: 5256, areaUnit: 'm2', angleUnit: 'grad', plotScale: 1000, lengthDecimals: 3, areaDecimals: 2 } });
  const at = (dx: number, dy: number) => ({ x: E + dx, y: N + dy });
  const list: NewEntity[] = [
    { kind: 'point', layerId: 'cizim', p: at(1.001, 2.002), z: 1024.35, attrs: { Ad: 'P1' }, label: 'P1' },
    { kind: 'line', layerId: 'cizim', a: at(0, 0), b: at(10.1, 0.2), attrs: {}, color: '#FF0000' },
    { kind: 'polyline', layerId: 'cizim', pts: [at(0, 0), at(5, 0), at(5, 5)], bulges: [0.414213562373095, 0, 0], attrs: {} },
    {
      kind: 'polygon',
      layerId: 'parsel',
      pts: [at(0, 0), at(23.417, 1.203), at(25.881, 31.466), at(2.004, 33.012)],
      bulges: [0, 0.12, 0, -0.05],
      holes: [{ pts: [at(5, 5), at(5, 10), at(10, 10), at(10, 5)] }],
      attrs: { Ada: '101', Parsel: '7', 'Tapu alanı': '723.52' },
      label: '101/7',
      symbol: 'mpyy.uip.konut.konut-alani',
    },
    { kind: 'circle', layerId: 'bina', c: at(12, 12), r: 3.25, attrs: {} },
    { kind: 'arc', layerId: 'bina', c: at(12, 12), r: 4, a0: 0.25, a1: 2.5, attrs: {} },
    { kind: 'ellipse', layerId: 'cizim', c: at(30, 30), major: { x: 6, y: 2 }, ratio: 0.4, t0: 0, t1: 0, attrs: {} },
    { kind: 'spline', layerId: 'cizim', pts: [at(0, 40), at(10, 45), at(20, 38)], closed: false, attrs: {} },
    { kind: 'xline', layerId: 'cizim', p: at(0, 50), dir: { x: Math.SQRT1_2, y: Math.SQRT1_2 }, attrs: {} },
    { kind: 'ray', layerId: 'cizim', p: at(0, 55), dir: { x: 1, y: 0 }, attrs: {} },
    { kind: 'text', layerId: 'cizim', p: at(3, 60), text: 'Çınar sokağı', height: 2.5, rotation: 15, attrs: {} },
    { kind: 'dimension', layerId: 'cizim', a: at(0, 0), b: at(23.417, 1.203), offset: 3, height: 1.8, style: 'linear', angle: 0, attrs: {} },
    { kind: 'hatch', layerId: 'bina', ring: [at(40, 0), at(50, 0), at(50, 10), at(40, 10)], holes: [[at(42, 2), at(42, 4), at(44, 4), at(44, 2)]], pattern: { type: 'lines', angle: 45, spacing: 1.5 }, attrs: {} },
  ];
  doc.load(list);
  doc.homeView = { minX: E - 10, minY: N - 10, maxX: E + 60, maxY: N + 70 };
  doc.styles.set({ items: [{ kind: 'symbol', id: 'proje.sembol', name: 'Proje sembolü', path: ['Proje'], symbol: { type: 'marker', layers: [{ id: 'a', type: 'shape', shape: 'circle', size: 2, fill: 'ink' }] } }], categories: [{ path: ['Proje'] }] });
  doc.dirty.set(false);
  return doc;
}
