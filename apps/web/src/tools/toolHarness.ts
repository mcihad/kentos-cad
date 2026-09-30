import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { LayerStore } from '../model/layers';
import { entityEdges } from '../model/ops/edges';
import { extendEntity, trimEntity } from '../model/ops/trim';
import { Selection } from '../model/selection';
import type { TextInputRequest } from '../viewport/ViewportController';
import type { ToolPointer } from './Tool';

/**
 * A document, a log, a selection and a stand-in for the viewport, for the tests
 * of tools that pick objects and write through a product command, without a
 * canvas: `hit` is what the pick under the cursor answers, `inWindow` the
 * ids a window picks. The drawing has the layers “cizim”, “yol” and a locked
 * “kilitli”.
 */
/** A new object without the fields the harness fills in (the layer, the attributes), whatever its kind. */
type Loose<T> = T extends unknown ? Omit<T, 'layerId' | 'attrs'> & { layerId?: string; attrs?: Record<string, string> } : never;

export function toolHarness() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        { id: 'cizim', name: 'Çizim' },
        { id: 'yol', name: 'Yol' },
        { id: 'kilitli', name: 'Kilitli', locked: true },
      ],
      'cizim',
    ),
    origin: { x: 0, y: 0 },
  });
  const log = new MessageLog();
  const state = {
    hit: null as Entity | null,
    inWindow: [] as number[],
    exited: 0,
    tool: null as { cancel?(): boolean } | null,
    /** The text fields the tool asked for (Yazı), oldest first. */
    textInputs: [] as TextInputRequest[],
  };
  const palette = { accent: '#0af', snap: '#fa0', danger: '#f33', fg: '#eee', labelHalo: '#000' };
  /** The edges of every object but `except`: the boundaries the store hands trim and extend. */
  const boundaries = (except: Entity) => [...doc.all()].filter((e) => e.id !== except.id).flatMap((e) => entityEdges(e));
  const ctx = {
    doc,
    log,
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null), lineWeight: new Signal<number | null>(null), ortho: new Signal(false), polar: new Signal(false) },
    prefs: { snapAperture: new Signal(8), pickAperture: new Signal(8), polarIncrement: new Signal(15) },
    view: {
      palette,
      requestOverlay: () => {},
      worldTolerance: (px: number) => px,
      pick: () => state.hit,
      pickEdge: (_s: unknown, filter?: (e: Entity) => boolean) => (state.hit && (!filter || filter(state.hit)) ? state.hit : null),
      pickRect: () => state.inWindow,
      trackAlong: () => null,
      camera: { visibleBounds: () => ({ minX: -50, minY: -50, maxX: 50, maxY: 50 }), worldToScreen: (p: { x: number; y: number }) => p },
      entitiesIn: () => [...doc.all()],
      trim: (target: Entity, at: { x: number; y: number }) => trimEntity(target, at, boundaries(target)),
      extend: (target: Entity, at: { x: number; y: number }) => extendEntity(target, at, boundaries(target)),
      ghosts: () => new Float64Array(),
      dimensionText: (l: { value: number }) => l.value.toFixed(3),
      requestTextInput: (req: TextInputRequest) => void state.textInputs.push(req),
      focus: () => {},
    },
    // The manager's `exit` (tools/ToolManager.ts): the running tool's `cancel` first, and it leaves when that says no.
    tools: { exit: () => void (state.tool?.cancel?.() || state.exited++) },
  } as unknown as AppContext;
  const said = () => log.entries.value.map((e) => e.text);
  const add = (e: Loose<NewEntity>): Entity => doc.add({ layerId: 'cizim', attrs: {}, ...e } as NewEntity);
  /** Registers the tool as the running one, so `ctx.tools.exit()` asks its `cancel` as the manager does. */
  const use = <T extends object>(tool: T): T => ((state.tool = tool as { cancel?(): boolean }), tool);
  return { ctx, doc, log, said, state, add, use };
}

export const pt = (x: number, y: number) => ({ x, y });

/** A left-button pointer at a world point (screen = world). */
export const at = (x: number, y: number, extra: Partial<ToolPointer> = {}): ToolPointer => ({
  world: pt(x, y),
  raw: pt(x, y),
  screen: pt(x, y),
  snap: null,
  track: null,
  button: 0,
  shift: false,
  ctrl: false,
  alt: false,
  ...extra,
});

/** A canvas context that records the names of the calls made on it. */
export function recorder() {
  const calls: string[] = [];
  const g = new Proxy(
    {},
    {
      get: (_t, key: string) => (key === 'measureText' ? () => ({ width: 40 }) : (..._a: unknown[]) => void calls.push(key)),
      set: () => true,
    },
  ) as unknown as CanvasRenderingContext2D;
  const view = { worldToScreen: (p: { x: number; y: number }) => p } as never;
  return { g, view, calls };
}

/**
 * A canvas context that records what a tool's preview draws: the paths stroked (the points the
 * calls moved through, and whether the stroke was dashed), the texts drawn and the centres of
 * the arcs (marks). `view` is the identity: the screen is the world.
 */
export function canvasLog() {
  const paths: { points: [number, number][]; dashed: boolean }[] = [];
  const texts: string[] = [];
  const arcs: [number, number][] = [];
  let points: [number, number][] = [];
  let dashed = false;
  const calls: Record<string, (...a: never[]) => unknown> = {
    measureText: () => ({ width: 40 }),
    beginPath: () => void (points = []),
    moveTo: ((x: number, y: number) => void (points = [[x, y]])) as never,
    lineTo: ((x: number, y: number) => void points.push([x, y])) as never,
    setLineDash: ((d: number[]) => void (dashed = d.length > 0)) as never,
    stroke: () => void (points.length > 1 && paths.push({ points: [...points], dashed })),
    fillText: ((t: string) => void texts.push(t)) as never,
    arc: ((x: number, y: number) => void arcs.push([x, y])) as never,
  };
  const g = new Proxy({}, { get: (_t, key: string) => (key === 'canvas' ? undefined : (calls[key] ?? (() => {}))), set: () => true }) as unknown as CanvasRenderingContext2D;
  const view = { worldToScreen: (p: { x: number; y: number }) => p, scale: 1, width: 100, height: 100 } as never;
  return { g, view, paths, texts, arcs };
}
