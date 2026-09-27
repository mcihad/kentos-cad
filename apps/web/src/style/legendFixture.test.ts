import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { LayerStore, type LayerInit } from '../model/layers';
import type { Symbol } from '../model/style';
import { LEGEND_PAPER, LEGEND_TEXTS, legendLayers, legendLayout, legendOf, type LegendGroup } from './legend';

/**
 * The legend as fixtures/style/v1/legend.json holds it
 * (scripts/fixtures/record-legend.test.ts): the layers it reads, each
 * layer's rows, the picture's layout and the window's texts. The desktop's
 * legend checks the same file.
 */

const files = import.meta.glob<string>('../../../../fixtures/style/v1/legend.json', { query: '?raw', import: 'default', eager: true });
const F = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  origin: Vec2;
  layers: LayerInit[];
  activeLayer: string;
  library: Record<string, { name?: string; symbol: Symbol }>;
  entities: Entity[];
  drawingName: string;
  legends: { visibleOnly: boolean; layers: string[]; groups: LegendGroup[] }[];
  layouts: { headings: boolean; left: string[]; rows?: number; layout: unknown }[];
  paper: unknown;
  texts: Record<string, unknown>;
};

const doc = new CadDocument({ name: 'Lejant', layers: new LayerStore(F.layers, F.activeLayer), origin: F.origin });
doc.load([...F.entities]);
const leaves = doc.layers.leaves().map((n) => ({ id: n.id, name: n.name, style: n.style, visible: doc.layers.isVisible(n.id) }));
const src = { entities: (id: string) => doc.byLayer(id), symbol: (r: string) => F.library[r]?.symbol, itemName: (id: string) => F.library[id]?.name };

describe('legend (fixtures/style/v1/legend.json)', () => {
  it('is a v1 legend file with the paper and the texts the window uses', () => {
    expect([F.format, F.version]).toEqual(['kentos.style-legend', 1]);
    expect(LEGEND_PAPER).toEqual(F.paper);
    expect({ ...LEGEND_TEXTS, rows: { n: 12, text: LEGEND_TEXTS.rows(12) } }).toEqual(F.texts);
  });

  for (const l of F.legends)
    it(`rows of ${l.visibleOnly ? 'the visible layers' : 'every layer'}`, () => {
      const layers = legendLayers(leaves, l.visibleOnly);
      expect(layers.map((x) => x.id)).toEqual(l.layers);
      expect(legendOf(layers, src)).toEqual(l.groups);
    });

  for (const c of F.layouts)
    it(`picture ${c.headings ? 'with' : 'without'} headings${c.left.length ? `, ${c.left.join(', ')} left out` : ''}`, () => {
      const groups = legendOf(legendLayers(leaves, true), src).filter((g) => !c.left.includes(g.layerId));
      if (c.rows !== undefined) expect(groups.reduce((n, g) => n + g.entries.length, 0)).toBe(c.rows);
      expect(legendLayout(groups, c.headings, F.drawingName)).toEqual(c.layout);
    });
});
