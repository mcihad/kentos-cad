import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import type { Entity, EntityGeometry } from '../model/entities';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { writeObjects } from './createCommand';
import { ArcTool, CircleTool } from './curveTools';
import { LineTool, PointTool } from './drawTools';
import { PathTool } from './pathTool';
import type { TemplateRun } from './templateStamp';

/**
 * New objects take the toolbar's Kalınlık (docs/adr/0139): every tool that
 * draws with lines writes it explicitly in its command's input (CMD-07). A
 * point and a text are not drawn with lines and take none; “Katmana göre”
 * (null) writes none; 0 is the thinnest line, not “none”. The desktop's twin
 * is crates/native/interaction/tests/all/line_weight.rs. Typed input only, over a
 * document, without a view.
 */
function harness(weight: number | null) {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'cizim', name: 'Çizim' }], 'cizim'), origin: { x: 0, y: 0 } });
  const ctx = {
    doc,
    log: new MessageLog(),
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null), lineWeight: new Signal<number | null>(weight), template: new Signal<TemplateRun | null>(null), overlap: new Signal<'allow' | 'layer' | 'layers'>('allow'), overlapLast: new Signal<'layer' | 'layers'>('layer'), overlapLayers: new Signal<ReadonlySet<string>>(new Set()), topology: new Signal(false), topologyPoints: new Signal(false) },
    view: { requestOverlay: () => {} },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  return { ctx, doc };
}

/** The objects `draw` wrote with the toolbar at `weight`, oldest first. */
function drawn(weight: number | null, draw: (ctx: AppContext) => void): Entity[] {
  const { ctx, doc } = harness(weight);
  draw(ctx);
  const written = [...doc.all()].sort((a, b) => a.id - b.id);
  expect(written.length).toBeGreaterThan(0);
  return written;
}

/** Types `inputs` into a freshly activated tool, then Enter when `finish`. */
function typed(tool: { activate(): void; input(text: string): boolean; confirm(): void }, inputs: string[], finish = false): void {
  tool.activate();
  for (const text of inputs) expect(tool.input(text), text).toBe(true);
  if (finish) tool.confirm();
}

const ELLIPSE: EntityGeometry = { kind: 'ellipse', c: { x: 0, y: 0 }, major: { x: 8, y: 6 }, ratio: 0.5, t0: 0, t1: 0 };
const TEXT: EntityGeometry = { kind: 'text', p: { x: 4, y: 2 }, text: 'Ada 101', height: 2.5, rotation: 0 };

describe('the toolbar’s Kalınlık on new objects', () => {
  const tools: [string, (ctx: AppContext) => void][] = [
    ['Çizgi', (ctx) => typed(new LineTool(ctx), ['0,0', '10,0'])],
    ['Çoklu çizgi', (ctx) => typed(new PathTool(ctx, { id: 'polyline', label: 'Çoklu çizgi', closed: false }), ['0,0', '10,0', '10,10'], true)],
    ['Kapalı alan', (ctx) => typed(new PathTool(ctx, { id: 'polygon', label: 'Kapalı alan', closed: true }), ['0,0', '10,0', '10,10'], true)],
    ['Daire', (ctx) => typed(new CircleTool(ctx), ['0,0', '5'])],
    ['Yay', (ctx) => typed(new ArcTool(ctx), ['10,0', '0,10', '-10,0'])],
    ['Elips (cad.entities.create)', (ctx) => expect(writeObjects(ctx, [ELLIPSE])).not.toBeNull()],
  ];

  it.each(tools)('%s writes the current weight; 0 is the thinnest line; “Katmana göre” writes none', (_name, draw) => {
    expect(drawn(0.5, draw).map((e) => e.lineWeight)).toEqual([0.5]);
    expect(drawn(0, draw).map((e) => e.lineWeight)).toEqual([0]);
    expect(drawn(null, draw).map((e) => 'lineWeight' in e)).toEqual([false]);
  });

  it('a point and a text take none, beside a line that takes it', () => {
    expect(drawn(0.5, (ctx) => typed(new PointTool(ctx, { id: 'point', label: 'Nokta', askZ: false }), ['1,2'])).map((e) => 'lineWeight' in e)).toEqual([false]);
    const line: EntityGeometry = { kind: 'line', a: { x: 0, y: 0 }, b: { x: 10, y: 0 } };
    expect(drawn(0.5, (ctx) => writeObjects(ctx, [TEXT, line])).map((e) => [e.kind, e.lineWeight])).toEqual([
      ['text', undefined],
      ['line', 0.5],
    ]);
  });
});
