// The browser half of scripts/e2e/compose.mjs (docs/adr/0226): the fixtures' scenes (fixtures/compose/v1/cases.json,
// written by scripts/fixtures/effects_cases.py from the ADR's definitions) drawn by the app's own WebGPU and WebGL2
// backends, read back and compared pixel by pixel.
import cases from '../../../../../fixtures/compose/v1/cases.json';
import type { BatchEffect, FrameState, LayerComposeFrame, RenderBackend, RGBA, SceneLayer, StyledBatch } from '../../../src/render/types';
import { WebGL2Backend } from '../../../src/render/webgl2/WebGL2Backend';
import { WebGPUBackend } from '../../../src/render/webgpu/WebGPUBackend';

type Box = readonly [number, number, number, number];
interface Fx {
  shadow?: { dx: number; dy: number; blur: number; color: number[] };
  glow?: { spread: number; blur: number; color: number[] };
  innerShadow?: { dx: number; dy: number; blur: number; color: number[] };
  innerGlow?: { spread: number; blur: number; color: number[] };
  blur?: number;
}

const [W, H] = cases.size as [number, number];
const MODES = ['normal', 'multiply', 'screen', 'overlay', 'darken', 'lighten', 'colorDodge', 'colorBurn', 'hardLight', 'softLight', 'difference', 'exclusion', 'addition'];

/** A box of the cases' pixels (x right, y down) in the world: one metre a pixel, the origin in the middle. */
const world = ([x0, y0, x1, y1]: Box): Box => [x0 - W / 2, H / 2 - y1, x1 - W / 2, H / 2 - y0];

const rgba = (c: number[]): RGBA => [c[0], c[1], c[2], c[3]];

function fill(box: Box, color: number[], effect?: BatchEffect): StyledBatch {
  const [x0, y0, x1, y1] = world(box);
  return {
    kind: 'fill',
    positions: new Float32Array([x0, y0, x1, y0, x1, y1, x0, y0, x1, y1, x0, y1]),
    paint: { kind: 'solid', color: rgba(color) },
    bounds: [x0, y0, x1, y1],
    reach: 0,
    reachUnit: 'world',
    ...(effect && { group: 1, effect }),
  };
}

/** A case's effects (device px, y down) as a batch carries them (y up). */
function effect(fx: Fx): BatchEffect {
  const shadow = (s: NonNullable<Fx['shadow']>) => ({ dx: s.dx, dy: -s.dy, blur: s.blur, color: rgba(s.color) });
  const glow = (g: NonNullable<Fx['glow']>) => ({ spread: g.spread, blur: g.blur, color: rgba(g.color) });
  return {
    unit: 'px',
    ...(fx.shadow && { shadow: shadow(fx.shadow) }),
    ...(fx.glow && { glow: glow(fx.glow) }),
    ...(fx.innerShadow && { innerShadow: shadow(fx.innerShadow) }),
    ...(fx.innerGlow && { innerGlow: glow(fx.innerGlow) }),
    ...(fx.blur && { blur: fx.blur }),
  };
}

const layer = (id: string, styled: StyledBatch[]): SceneLayer => ({ id, lines: [], fills: [], points: [], styled });

interface Scene {
  name: string;
  layers: SceneLayer[];
  compositions?: Map<string, LayerComposeFrame>;
  picture: number[];
}

function scenes(): Scene[] {
  const out: Scene[] = [];
  for (const c of cases.effects) out.push({ name: c.name, layers: [layer('a', [fill(c.square as unknown as Box, c.color, effect(c.effects as Fx))])], picture: c.picture });
  for (const c of cases.blends) {
    const under = c.under as number[][];
    out.push({
      name: c.name,
      layers: [layer('under', [fill([0, 0, W / 2, H], under[0]), fill([W / 2, 0, W, H], under[1])]), layer('top', [fill(c.square as unknown as Box, c.color)])],
      compositions: new Map([['top', { opacity: c.opacity, blend: MODES.indexOf(c.mode), masks: [] }]]),
      picture: c.picture,
    });
  }
  for (const c of cases.masks) {
    const masks = (c.boxes as number[][]).map(([cx, cy, hx, hy, turn, grow]) => ({ markers: null, labels: [cx - W / 2, H / 2 - cy, 2 * hx, 2 * hy, (-turn * 180) / Math.PI], grow }));
    out.push({
      name: c.name,
      layers: [layer('band', [fill(c.band as unknown as Box, c.color)])],
      compositions: new Map([['band', { opacity: c.opacity, blend: MODES.indexOf(c.mode), masks }]]),
      picture: c.picture,
    });
  }
  return out;
}

/** A backend's frame read back: the canvas copied in the same task (WebGPU's texture is the canvas's until then). */
function readBack(canvas: HTMLCanvasElement): Uint8ClampedArray {
  const copy = document.createElement('canvas');
  copy.width = W;
  copy.height = H;
  const g = copy.getContext('2d', { willReadFrequently: true })!;
  g.drawImage(canvas, 0, 0);
  return g.getImageData(0, 0, W, H).data;
}

export interface Result {
  name: string;
  worst: number;
  at: [number, number];
  off: number;
  /** The worst pixel as drawn and as expected. */
  got: number[];
  want: number[];
}

async function run(kind: 'webgpu' | 'webgl2', samples: number): Promise<{ label: string; results: Result[] } | { error: string }> {
  const canvas = document.createElement('canvas');
  document.body.append(canvas);
  const backend: RenderBackend = kind === 'webgpu' ? new WebGPUBackend() : new WebGL2Backend();
  try {
    await backend.init(canvas, { samples });
  } catch (e) {
    return { error: String(e) };
  }
  backend.resize(W, H, 1);
  const g = cases.ground as number[];
  const results: Result[] = [];
  for (const s of scenes()) {
    for (const l of s.layers) backend.upload(l);
    const frame: FrameState = {
      view: { center: { x: 0, y: 0 }, scale: 1, width: W, height: H, dpr: 1 },
      scaleDenominator: 1 / 0.00026458,
      clearColor: [g[0] / 255, g[1] / 255, g[2] / 255, 1],
      order: s.layers.map((l) => l.id),
      underlays: [],
      overlays: [],
      keepBase: false,
      ...(s.compositions && { compositions: s.compositions }),
    };
    backend.render(frame);
    const got = readBack(canvas);
    let worst = 0;
    let at: [number, number] = [0, 0];
    let off = 0;
    for (let i = 0; i < got.length; i++) {
      const d = Math.abs(got[i] - s.picture[i]);
      if (d > worst) {
        worst = d;
        at = [(i >> 2) % W, Math.floor((i >> 2) / W)];
      }
      if (d > cases.tolerance) off++;
    }
    const k = (at[1] * W + at[0]) * 4;
    results.push({ name: s.name, worst, at, off, got: Array.from(got.slice(k, k + 4)), want: s.picture.slice(k, k + 4) });
    for (const l of s.layers) backend.remove(l.id);
  }
  const label = backend.label;
  backend.dispose();
  canvas.remove();
  return { label, results };
}

// ── The timing (docs/adr/0226 §7): the desktop's scene (crates/render/wgpu/tests/gpu/compose_timing.rs) ──
// 1440 × 900: land use, a relief drawn with Çoğalt at 70 %, roads masked by 300 labels, 2 000 buildings with a shadow,
// an outer glow and an inner glow (values of 1:1000, a millimetre 3.78 px); each composed part alone besides. Every
// frame's base is drawn again and waited for (a pixel read back): the time is the main thread's and the GPU's.

const TW = 1440;
const TH = 900;
const MM = 96 / 25.4;

type Rect = readonly [number, number, number, number];

function fills(rects: readonly Rect[], color: number[], effect?: BatchEffect): StyledBatch {
  const pos = new Float32Array(rects.length * 12);
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  rects.forEach(([a, b, c, d], i) => {
    pos.set([a, b, c, b, c, d, a, b, c, d, a, d], i * 12);
    x0 = Math.min(x0, a);
    y0 = Math.min(y0, b);
    x1 = Math.max(x1, c);
    y1 = Math.max(y1, d);
  });
  return { kind: 'fill', positions: pos, paint: { kind: 'solid', color: rgba(color) }, bounds: [x0, y0, x1, y1], reach: 0, reachUnit: 'world', ...(effect && { group: 1, effect }) };
}

interface Parts {
  relief?: boolean;
  roads?: boolean;
  shadow?: boolean;
  glow?: boolean;
  innerGlow?: boolean;
}

function timingScene(parts: Parts): { layers: SceneLayer[]; compositions: Map<string, LayerComposeFrame> } {
  const hw = TW / 2;
  const hh = TH / 2;
  const colours = [
    [0.63, 0.85, 0.61, 1],
    [1, 0.89, 0.57, 1],
    [0.99, 0.73, 0.63, 1],
    [0.8, 0.85, 0.9, 1],
  ];
  const land = colours.map((c, ci) => {
    const rects: Rect[] = [];
    for (let k = ci; k < 12; k += 4) {
      const col = k % 4;
      const row = Math.floor(k / 4);
      rects.push([-hw + col * 360, -hh + row * 300, -hw + (col + 1) * 360, -hh + (row + 1) * 300]);
    }
    return fills(rects, c);
  });
  const relief = Array.from({ length: 16 }, (_, k) => {
    const x = -hw + k * 90;
    const v = 0.94 - k * 0.025;
    return fills([[x, -hh, x + 90, hh]], [v, v, v, 1]);
  });
  const half = 1.2 * MM;
  const roads: Rect[] = [];
  for (let k = 0; k < 12; k++) {
    const y = -hh + 37.5 + k * 75;
    roads.push([-hw, y - half, hw, y + half]);
  }
  for (let k = 0; k < 18; k++) {
    const x = -hw + 40 + k * 80;
    roads.push([x - half, -hh, x + half, hh]);
  }
  const buildings: Rect[] = [];
  for (let k = 0; k < 2000; k++) {
    const x = -hw + 6 + (k % 50) * 28.8;
    const y = -hh + 4 + Math.floor(k / 50) * 22.5;
    buildings.push([x, y, x + 18, y + 14]);
  }
  const fx: BatchEffect | undefined =
    parts.shadow || parts.glow || parts.innerGlow
      ? {
          unit: 'px',
          ...(parts.shadow && { shadow: { dx: 1.2 * MM * Math.SQRT1_2, dy: -1.2 * MM * Math.SQRT1_2, blur: 1.5 * MM, color: [0, 0, 0, 0.45] as RGBA } }),
          ...(parts.glow && { glow: { spread: 0.5 * MM, blur: 1.5 * MM, color: [1, 0.82, 0, 0.6] as RGBA } }),
          ...(parts.innerGlow && { innerGlow: { spread: 1 * MM, blur: 2.5 * MM, color: [1, 1, 1, 0.8] as RGBA } }),
        }
      : undefined;
  const labels: number[] = [];
  for (let k = 0; k < 300; k++) labels.push(((k * 97) % 1400) - 700, -(((k * 61) % 860) - 430), 64, 14, -(((k * 37) % 180) - 90));
  const compositions = new Map<string, LayerComposeFrame>();
  if (parts.relief) compositions.set('relief', { opacity: 0.7, blend: MODES.indexOf('multiply'), masks: [] });
  if (parts.roads) compositions.set('roads', { opacity: 1, blend: 0, masks: [{ markers: null, labels, grow: MM }] });
  return {
    layers: [layer('land', land), layer('relief', relief), layer('roads', [fills(roads, [1, 1, 1, 1])]), layer('buildings', [fills(buildings, [0.85, 0.8, 0.75, 1], fx)])],
    compositions,
  };
}

async function time(kind: 'webgpu' | 'webgl2', samples: number): Promise<{ label: string; rows: [string, number][] } | { error: string }> {
  const canvas = document.createElement('canvas');
  document.body.append(canvas);
  const backend: RenderBackend = kind === 'webgpu' ? new WebGPUBackend() : new WebGL2Backend();
  try {
    await backend.init(canvas, { samples });
  } catch (e) {
    return { error: String(e) };
  }
  backend.resize(TW, TH, 1);
  const pixel = document.createElement('canvas').getContext('2d', { willReadFrequently: true })!;
  const configs: [string, Parts][] = [
    ['yalın', {}],
    ['hepsi', { relief: true, roads: true, shadow: true, glow: true, innerGlow: true }],
    ['yalnız çoğaltan kabartma', { relief: true }],
    ['yalnız maskelenen yollar', { roads: true }],
    ['yalnız gölge', { shadow: true }],
    ['yalnız dış parıltı', { glow: true }],
    ['yalnız iç parıltı', { innerGlow: true }],
  ];
  const built = configs.map(([name, parts]) => ({ name, ...timingScene(parts) }));
  const times: number[][] = built.map(() => []);
  let pan = 0;
  for (let round = 0; round < 7; round++) {
    for (let c = 0; c < built.length; c++) {
      const s = built[c];
      for (const l of s.layers) backend.upload(l);
      for (let i = 0; i < 12; i++) {
        pan = pan === 0 ? 1 : 0;
        const frame: FrameState = {
          view: { center: { x: pan, y: 0 }, scale: 1, width: TW, height: TH, dpr: 1 },
          scaleDenominator: 1000,
          clearColor: [0.87, 0.87, 0.87, 1],
          order: s.layers.map((l) => l.id),
          underlays: [],
          overlays: [],
          keepBase: false,
          ...(s.compositions.size && { compositions: s.compositions }),
        };
        const t = performance.now();
        backend.render(frame);
        pixel.drawImage(canvas, 0, 0, 1, 1, 0, 0, 1, 1);
        pixel.getImageData(0, 0, 1, 1);
        const ms = performance.now() - t;
        if (round > 0 && i >= 2) times[c].push(ms);
      }
      for (const l of s.layers) backend.remove(l.id);
    }
  }
  const median = (v: number[]) => [...v].sort((a, b) => a - b)[v.length >> 1];
  const rows: [string, number][] = built.map((s, c) => [s.name, median(times[c])]);
  const label = backend.label;
  backend.dispose();
  canvas.remove();
  return { label, rows };
}

(window as unknown as { composeRun: typeof run; composeTime: typeof time }).composeRun = run;
(window as unknown as { composeTime: typeof time }).composeTime = time;
