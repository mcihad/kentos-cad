import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { LayerStyle } from '../model/layers';
import type { LibraryAsset, Symbol } from '../model/style';
import { layerDocument } from '../style/cases';
import { batchesJson, captureStyled, decisionsOf, type StyledCall } from '../style/fixture';
import { PickIndex } from '../viewport/picking';
import { symbolScaleOf } from '../viewport/symbolScale';
import type { CanvasPalette } from './color';
import { buildStyledLayer, viewModesOf } from './styledLayer';

/**
 * A styled layer's way to the GPU as fixtures/style/v1/batches.json holds it
 * (scripts/fixtures/record-batches.test.ts): the scale symbols are compiled
 * at, how each object is drawn, what the page sends the style core, and the
 * GPU batches it makes of the answer with the file's palette. The desktop's
 * drawing checks the same file.
 */

interface Case {
  id: string;
  title: string;
  style: LayerStyle;
  entities: Entity[];
  plotScale: number;
  view: { symbolSize: 'plot' | 'screen'; pxPerM: number; lineWeights: boolean; colorMode?: 'color' | 'mono' | 'gray'; fills?: boolean; areaEdges?: boolean; transparency?: boolean };
  expect: { symbolScale: number; decisions: unknown[]; program: unknown; objects: number[]; table: unknown; batches: unknown[] };
}

const files = import.meta.glob<string>('../../../../fixtures/style/v1/batches.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  palette: CanvasPalette;
  assets: LibraryAsset[];
  library: Record<string, Symbol>;
  origin: Vec2;
  layer: { id: string; name: string };
  cases: Case[];
};

describe('styled layers’ way to the GPU (fixtures/style/v1/batches.json)', () => {
  it('is a v1 batches file', () => expect([F.format, F.version]).toEqual(['kentos.style-batches', 1]));

  for (const c of F.cases)
    it(`${c.id}: ${c.title}`, () => {
      const doc = layerDocument(F.layer.name, c.style, c.entities);
      const entities = [...doc.all()];
      const symbolScale = symbolScaleOf(c.view.symbolSize, c.plotScale, c.view.pxPerM);
      expect(symbolScale, 'symbolScale').toBe(c.expect.symbolScale);
      let call: StyledCall | null = null;
      const layer = buildStyledLayer(F.layer.id, entities, doc.layers.get(F.layer.id)!.style, {
        origin: F.origin,
        palette: F.palette,
        plotScale: symbolScale,
        screen: c.view.symbolSize === 'screen',
        hairlines: !c.view.lineWeights,
        view: viewModesOf(c.view),
        library: { symbol: (id) => F.library[id], asset: (id) => F.assets.find((a) => a.id === id) },
        layerName: (id) => doc.layers.get(id)?.name ?? id,
        geometry: captureStyled(new PickIndex(doc), (x) => (call = x)),
      });
      const got = call as StyledCall | null;
      expect(got, 'the core is called').not.toBeNull();
      expect(
        decisionsOf(
          entities.map((e) => e.id),
          got!.objects,
        ),
        'decisions',
      ).toEqual(c.expect.decisions);
      expect(JSON.parse(got!.program), 'program').toEqual(c.expect.program);
      expect(got!.objects, 'objects').toEqual(c.expect.objects);
      expect(got!.table, 'table').toEqual(c.expect.table);
      expect(batchesJson(layer.styled ?? []), 'batches').toEqual(c.expect.batches);
    });
});
