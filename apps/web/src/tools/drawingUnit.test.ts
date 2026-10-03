import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { CircleTool } from './curveTools';
import { LineTool } from './drawTools';

/**
 * A local project's drawing unit (docs/adr/0165 §2): what is typed is in
 * it, what the drawing keeps is metres. Typed input only, over a document
 * whose formatter follows its settings, as the app's does.
 */
function harness(unit: 'mm' | 'cm' | 'm') {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore([{ id: 'cizim', name: 'Çizim' }], 'cizim'),
    origin: { x: 0, y: 0 },
  });
  // A local CAD project, its angles in degrees, as a new one is (docs/adr/0165 §2, §4).
  doc.settings.assign({ srid: 0, drawingUnit: unit, workspace: 'cad', angleUnit: 'deg' });
  const log = new MessageLog();
  const ctx = {
    doc,
    log,
    selection: new Selection(),
    format: new Formatter(doc.settings),
    settings: { color: new Signal<string | null>(null), lineWeight: new Signal<number | null>(null) },
    view: { requestOverlay: () => {}, trackAlong: () => null },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  return { ctx, doc, said: () => log.entries.value.map((e) => e.text) };
}

describe('a local project in millimetres', () => {
  it('takes typed points in millimetres: absolute, relative and polar', () => {
    const { ctx, doc } = harness('mm');
    const tool = new LineTool(ctx);
    tool.activate();
    for (const p of ['100,250', '@500,0', '@100<90']) expect(tool.input(p)).toBe(true);
    const lines = [...doc.all()].map((e) => (e.kind === 'line' ? [e.a, e.b] : null));
    expect(lines[0]).toEqual([
      { x: 0.1, y: 0.25 },
      { x: 0.6, y: 0.25 },
    ]);
    const [a, b] = lines[1]!;
    expect(a).toEqual({ x: 0.6, y: 0.25 });
    expect(b.x).toBeCloseTo(0.6, 12);
    expect(b.y).toBeCloseTo(0.35, 12);
  });

  it('takes a typed radius in millimetres', () => {
    const { ctx, doc } = harness('mm');
    const tool = new CircleTool(ctx);
    tool.activate();
    expect(tool.input('0,0')).toBe(true);
    expect(tool.input('12.5')).toBe(true);
    const circle = [...doc.all()][0];
    expect(circle?.kind === 'circle' && circle.r).toBe(0.0125);
  });

  it('reads lengths and areas in the unit, the drawing kept in metres', () => {
    const { ctx } = harness('cm');
    const f = ctx.format;
    expect([f.coord(1.5), f.length(0.12), f.area(0.0001), f.lengthUnitLabel, f.areaUnitLabel]).toEqual(['150.000', '12.000 cm', '1.00 cm²', 'cm', 'cm²']);
    expect([f.toMetres(250), f.fromMetres(0.25)]).toEqual([2.5, 25]);
    // A limit with the digits it needs, and the unit in words.
    expect([f.plain(1e-6), f.lengthUnitName]).toEqual(['0.0001', 'santimetre']);
    // The Hesap windows stay in metres (docs/adr/0165 §2).
    expect(f.metric().length(0.12)).toBe('0.120 m');
  });
});

describe('a project with a coordinate system', () => {
  it('is in metres whatever drawing unit it once had', () => {
    const { ctx, doc } = harness('mm');
    doc.settings.assign({ srid: 5254 });
    expect(ctx.format.unit).toBe('m');
    expect(ctx.format.length(0.12)).toBe('0.120 m');
    const tool = new LineTool(ctx);
    tool.activate();
    for (const p of ['100,250', '@5,0']) expect(tool.input(p)).toBe(true);
    const line = [...doc.all()][0];
    expect(line?.kind === 'line' && [line.a, line.b]).toEqual([
      { x: 100, y: 250 },
      { x: 105, y: 250 },
    ]);
  });
});
