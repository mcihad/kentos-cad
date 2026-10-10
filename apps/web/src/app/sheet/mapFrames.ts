import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { MapLayers } from '../../contracts/generated/sheet/MapLayers';
import type { Bounds } from '../../model/geometry';
import { Atlas } from '../../render/atlas';
import type { CanvasPalette } from '../../render/color';
import { buildStyledLayer, type StyledBuildOptions } from '../../render/styledLayer';
import type { RenderBackend } from '../../render/types';
import { WebGL2Backend } from '../../render/webgl2/WebGL2Backend';
import { LEGEND_PAPER } from '../../style/legend';
import { Camera } from '../../viewport/Camera';
import { drawLabels } from '../../viewport/overlay';
import type { ShownLabels } from '../../viewport/picking';
import type { AppContext } from '../context';
import { framedSpots, insideFrame } from './frameLabels';

/**
 * The content of a sheet's map frames (docs/sheet/design.md §8, §11): the
 * project's drawing as the drawing area draws it (the same styled layers
 * from the same geometry store, the same labels), on white paper, at the
 * map's view (its centre, its scale, its own turn), drawn off the screen
 * with a WebGL2 backend of its own and kept: a picture is drawn again only
 * when the view, the size, the layers or the drawing change. Symbols sized
 * in paper millimetres are compiled at the map's scale, so they come out
 * on the sheet at their own size. The painter (render/sheet/painter.ts)
 * puts a picture into the map's content rectangle and turns it with the
 * frame; the same pictures fill the gallery's small papers and the
 * exports (PNG at the export's dpi, the SVG's embedded PNG). The PDF's
 * fallback picture has no paper under the drawing (straight alpha, as the
 * desktop's): the backend's canvas has none, so each tile is drawn over
 * white and over black and what covers the paper is their difference.
 *
 * The screen's pictures are at most `SCREEN_SIDE` pixels a side (a closer
 * zoom shows them softer, until the next phase draws only what is seen); an
 * export's are drawn in tiles of `TILE` pixels, any size.
 */

const SCREEN_SIDE = 4096;
/** The smallest picture drawn (its longer side, pixels): a small paper's map is shrunk from it, its lines light. */
const MIN_SIDE = 640;
const TILE = 2048;
/** How long a picture of the wrong size or view is shown while its view still moves (a zoom of the desk). */
const SETTLE_MS = 140;
/** Pictures kept at once (the desk's and the small papers' of every map shown). */
const KEEP = 24;

interface Kept {
  key: string;
  canvas: HTMLCanvasElement;
}

/**
 * Straight-alpha pixels from the same picture drawn over white and over black (RGBA, as a canvas gives them):
 * over white a channel is c·a + 255·(1 − a), over black c·a, so the paper that shows is their difference.
 */
export function unpapered(white: Uint8ClampedArray, black: Uint8ClampedArray): Uint8ClampedArray<ArrayBuffer> {
  const out = new Uint8ClampedArray(white.length);
  for (let i = 0; i < out.length; i += 4) {
    const shows = (white[i] - black[i] + white[i + 1] - black[i + 1] + white[i + 2] - black[i + 2]) / 3;
    const a = Math.max(0, Math.min(255, 255 - shows));
    out[i + 3] = a;
    if (a > 0) for (let c = 0; c < 3; c++) out[i + c] = (black[i + c] * 255) / a;
  }
  return out;
}

/** The paper's palette over the screen's: white paper, black ink, whatever the theme (as the legend window draws). */
export function paperPalette(ctx: AppContext): CanvasPalette {
  return { ...ctx.view.palette, ...LEGEND_PAPER, background: [...LEGEND_PAPER.background], label: LEGEND_PAPER.fg, labelHalo: LEGEND_PAPER.paper };
}

/** How a map frame builds the drawing's layers: symbols at the map's scale, on the paper's palette. */
export function styleAt(ctx: AppContext, scale: number, palette: CanvasPalette, clip: Bounds): StyledBuildOptions {
  const { doc } = ctx;
  return { origin: doc.origin, palette, plotScale: scale, screen: false, hairlines: false, library: ctx.styles.library, layerName: (id: string) => doc.layers.get(id)?.name ?? id, geometry: ctx.view.geometry, clip };
}

/**
 * The label records a map frame writes, of a box at `pxPerM`: only of the map's layers when it shows a list of
 * them, and only those whose anchor is inside the frame (frameLabels.ts; screen, pictures and PDF alike).
 */
export function labelSpots(ctx: AppContext, prim: Pick<MapPrim, 'layers' | 'clip' | 'view'>, bounds: Bounds, pxPerM: number): ShownLabels {
  const shown = ctx.view.geometry.labels(bounds, pxPerM, null);
  const layers = prim.layers;
  const records = framedSpots(shown.records, (id) => ctx.doc.get(id), insideFrame(prim), layers.type === 'list' ? (e) => layers.layers.includes(e.layerId) : undefined);
  return { records, texts: shown.texts };
}

/** The layers a map shows, bottom first (the backend's draw order). A theme is not a thing of this app yet: all visible ones. */
export function shownLayers(ctx: AppContext, layers: MapLayers): string[] {
  const { doc } = ctx;
  const visible = doc.layers
    .leaves()
    .filter((l) => doc.layers.isVisible(l.id))
    .map((l) => l.id);
  const shown = layers.type === 'list' ? visible.filter((id) => layers.layers.includes(id)) : visible;
  return shown.reverse();
}

export class MapFrames {
  /** A picture finished drawing, or the ones on show went stale: the paper is painted again. */
  onChange: (() => void) | null = null;
  private readonly ctx: AppContext;
  private gl: { backend: RenderBackend; canvas: HTMLCanvasElement } | null = null;
  private starting: Promise<void> | null = null;
  private failed: string | null = null;
  private readonly atlas = new Atlas();
  private atlasEpoch = 0;
  private styleEpoch = 0;
  /** What the backend's layers were built for: the drawing's revision, the scale symbols were compiled at, the styles. */
  private uploaded = '';
  private readonly kept = new Map<string, Kept>();
  private readonly waiting = new Map<string, { prim: MapPrim; pxPerUm: number }>();
  private timer = 0;
  private readonly offs: (() => void)[] = [];

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.atlas.onChange = () => {
      this.atlasEpoch++;
      this.onChange?.();
    };
    this.offs.push(ctx.styles.library.events.on('changed', () => {
      this.styleEpoch++;
      this.onChange?.();
    }));
  }

  /** Why no picture can be drawn (WebGL2 could not start); null when it can. */
  get problem(): string | null {
    return this.failed;
  }

  /**
   * The picture of a map frame's content for a paint at `pxPerUm` device
   * pixels per micrometre: the kept one when it is for this view and size;
   * while the view still moves, the last one kept (drawn again once it
   * settles); otherwise drawn now. Null while WebGL2 starts or when it
   * cannot (`problem`), and for a map with no place.
   */
  picture(prim: MapPrim, pxPerUm: number): HTMLCanvasElement | null {
    if (!prim.view.center || this.failed) return null;
    if (!this.gl) {
      void this.start();
      return null;
    }
    // A small picture (a small paper's) is drawn larger and shrunk where it is shown: a hairline of the
    // drawing is a pixel at least on the GPU, and a few hundred of them in a thumbnail would be a dark blot.
    const side = Math.max(prim.clip.width, prim.clip.height);
    const k = Math.min(Math.max(pxPerUm, MIN_SIDE / side), SCREEN_SIDE / side);
    const key = this.keyOf(prim, k);
    const slot = this.slotOf(prim, k);
    const hit = this.kept.get(slot);
    if (hit?.key === key) return this.touch(slot, hit).canvas;
    // While the view still moves, the last picture of this map (of any size) stands in until it settles.
    const stale = hit ?? [...this.kept.entries()].reverse().find(([s]) => s.startsWith(`${prim.item}|`))?.[1];
    if (stale) {
      this.waiting.set(slot, { prim, pxPerUm: k });
      clearTimeout(this.timer);
      this.timer = window.setTimeout(() => this.settle(), SETTLE_MS);
      return stale.canvas;
    }
    const canvas = this.draw(prim, k, SCREEN_SIDE);
    this.keep(slot, { key, canvas });
    return canvas;
  }

  /** A picture's place in the cache: the map and the picture's size (the desk's and a small paper's apart). */
  private slotOf(prim: MapPrim, pxPerUm: number): string {
    // Sizes in steps of an eighth of an octave: a zoom that changes little keeps its picture until it settles.
    const step = Math.round(Math.log2(Math.max(1e-9, pxPerUm)) * 8);
    return `${prim.item}|${step}`;
  }

  private touch(slot: string, k: Kept): Kept {
    this.kept.delete(slot);
    this.kept.set(slot, k);
    return k;
  }

  /** Kept pictures, the last used last; the oldest go beyond `KEEP` (a picture of a large map is tens of megabytes). */
  private keep(slot: string, k: Kept): void {
    this.kept.delete(slot);
    this.kept.set(slot, k);
    while (this.kept.size > KEEP) this.kept.delete(this.kept.keys().next().value!);
  }

  /**
   * A picture for an export, at exactly `pxPerUm`, in tiles when it is large; not kept. Waits for WebGL2.
   * `transparent`: no paper under the drawing (the PDF's fallback picture).
   */
  async exportPicture(prim: MapPrim, pxPerUm: number, transparent = false): Promise<HTMLCanvasElement | null> {
    if (!prim.view.center) return null;
    await this.start();
    if (this.failed || !this.gl) return null;
    return this.draw(prim, pxPerUm, TILE, transparent);
  }

  /** Every kept picture is stale (the drawing or its look changed): drawn again at the next paint. */
  invalidate(): void {
    this.kept.clear();
  }

  dispose(): void {
    clearTimeout(this.timer);
    this.offs.forEach((off) => off());
    this.kept.clear();
    this.gl?.backend.dispose();
    this.gl = null;
  }

  private settle(): void {
    for (const [slot, w] of this.waiting) this.keep(slot, { key: this.keyOf(w.prim, w.pxPerUm), canvas: this.draw(w.prim, w.pxPerUm, SCREEN_SIDE) });
    this.waiting.clear();
    this.onChange?.();
  }

  private start(): Promise<void> {
    if (this.gl || this.failed) return Promise.resolve();
    return (this.starting ??= (async () => {
      const canvas = document.createElement('canvas');
      const backend = new WebGL2Backend();
      try {
        await backend.init(canvas, { samples: 4 });
        backend.useAtlas(this.atlas);
        this.gl = { backend, canvas };
      } catch (e) {
        backend.dispose();
        this.failed = `Harita çerçeveleri çizilemiyor: ${(e as Error).message}. Tarayıcıda donanım hızlandırmasını açın.`;
        this.ctx.log.error(this.failed);
      }
      this.onChange?.();
    })());
  }

  private keyOf(prim: MapPrim, pxPerUm: number): string {
    const v = prim.view;
    const w = Math.max(1, Math.round(prim.clip.width * pxPerUm));
    const h = Math.max(1, Math.round(prim.clip.height * pxPerUm));
    return `${v.center!.x},${v.center!.y}|${v.scale}|${v.rotation}|${w}x${h}|${JSON.stringify(prim.layers)}|${this.ctx.doc.revision}|${this.styleEpoch}|${this.atlasEpoch}`;
  }


  /** Builds every layer for symbols compiled at `scale` (once per drawing revision and scale). */
  private upload(backend: RenderBackend, scale: number, palette: CanvasPalette, clip: Bounds): void {
    const { doc } = this.ctx;
    const key = `${doc.revision}|${scale}|${this.styleEpoch}`;
    if (key === this.uploaded) return;
    this.uploaded = key;
    const style = styleAt(this.ctx, scale, palette, clip);
    for (const l of doc.layers.leaves()) {
      if (l.type !== 'layer') continue;
      backend.upload(buildStyledLayer(l.id, doc.byLayer(l.id), l.style, style));
    }
  }

  private palette(): CanvasPalette {
    return paperPalette(this.ctx);
  }

  /**
   * Draws a map's content: the drawing north up over the box its turned
   * content covers, frame by frame (tiles of at most `side` pixels), then the
   * labels over the whole, then turned into the content's rectangle. On
   * white paper, or `transparent`: none.
   */
  private draw(prim: MapPrim, pxPerUm: number, side: number, transparent = false): HTMLCanvasElement {
    const { doc } = this.ctx;
    const gl = this.gl!;
    const v = prim.view;
    const center = v.center!;
    const W = Math.max(1, Math.round(prim.clip.width * pxPerUm));
    const H = Math.max(1, Math.round(prim.clip.height * pxPerUm));
    const turn = ((v.rotation % 360_000) * Math.PI) / 180_000;
    const cos = Math.abs(Math.cos(turn));
    const sin = Math.abs(Math.sin(turn));
    const bw = turn ? Math.ceil(W * cos + H * sin) : W;
    const bh = turn ? Math.ceil(W * sin + H * cos) : H;
    // Device pixels per metre of ground: micrometres of paper per metre are 10⁶ over the scale.
    const pxPerM = (pxPerUm * 1_000_000) / Math.max(1, v.scale);
    // Device pixels per CSS pixel on the paper: a size in pixels (a point symbol, a label) is a CSS pixel of the
    // paper (1/96 in), so it prints as it shows at 100 % and is the same at every resolution and in the PDF's vectors.
    const k = (pxPerUm * 25_400) / 96;
    const palette = this.palette();
    const half = { x: bw / 2 / pxPerM, y: bh / 2 / pxPerM };
    const extent: Bounds = { minX: center.x - half.x, minY: center.y - half.y, maxX: center.x + half.x, maxY: center.y + half.y };
    this.upload(gl.backend, v.scale, palette, extent);
    const order = shownLayers(this.ctx, prim.layers);
    const big = document.createElement('canvas');
    big.width = bw;
    big.height = bh;
    const g = big.getContext('2d')!;
    const origin = doc.origin;
    // Without paper: a tile is read back over white and over black (unpapered).
    const scratch = transparent ? document.createElement('canvas').getContext('2d', { willReadFrequently: true }) : null;
    for (let ty = 0; ty < bh; ty += side) {
      for (let tx = 0; tx < bw; tx += side) {
        const tw = Math.min(side, bw - tx);
        const th = Math.min(side, bh - ty);
        // The tile's centre on the ground (y up on the ground, down in the picture).
        const cx = extent.minX + (tx + tw / 2) / pxPerM;
        const cy = extent.maxY - (ty + th / 2) / pxPerM;
        gl.backend.resize(tw / k, th / k, k);
        const pass = (clearColor: readonly [number, number, number, number]) =>
          gl.backend.render({
            view: { center: { x: cx - origin.x, y: cy - origin.y }, scale: pxPerM / k, width: tw / k, height: th / k, dpr: k },
            scaleDenominator: v.scale,
            clearColor,
            order,
            underlays: [],
            overlays: [],
          });
        if (!scratch) {
          pass(palette.background);
          // Copied in the same task as it was drawn: the WebGL picture is still there.
          g.drawImage(gl.canvas, tx, ty);
          continue;
        }
        scratch.canvas.width = tw;
        scratch.canvas.height = th;
        const over = (paper: readonly [number, number, number, number]) => {
          pass(paper);
          scratch.drawImage(gl.canvas, 0, 0);
          return scratch.getImageData(0, 0, tw, th).data;
        };
        const white = over([1, 1, 1, 1]);
        g.putImageData(new ImageData(unpapered(white, over([0, 0, 0, 1])), tw, th), tx, ty);
      }
    }
    this.labels(g, prim, center, pxPerM, bw, bh, palette, k);
    if (!turn) return big;
    const out = document.createElement('canvas');
    out.width = W;
    out.height = H;
    const o = out.getContext('2d')!;
    if (!transparent) {
      o.fillStyle = LEGEND_PAPER.paper;
      o.fillRect(0, 0, W, H);
    }
    o.translate(W / 2, H / 2);
    o.rotate(turn);
    o.drawImage(big, -bw / 2, -bh / 2);
    return out;
  }

  /**
   * The drawing's labels (texts, dimension values, names) as the drawing area writes them, on the layers shown.
   * A label's pixels are CSS pixels on the paper (as on the screen at 100 %): `k` picture pixels each, so a
   * label keeps its size on the paper whatever the picture's density (the PDF's vector texts are the same size).
   */
  private labels(g: CanvasRenderingContext2D, prim: MapPrim, center: { x: number; y: number }, pxPerM: number, w: number, h: number, palette: CanvasPalette, k: number): void {
    const { doc } = this.ctx;
    const cam = new Camera();
    cam.center = { x: center.x, y: center.y };
    cam.scale = pxPerM / k;
    cam.width = w / k;
    cam.height = h / k;
    const geometry = this.ctx.view.geometry;
    const spots = labelSpots(this.ctx, prim, cam.visibleBounds(), pxPerM / k);
    g.save();
    g.scale(k, k);
    drawLabels(g, doc, cam, palette, spots, (l, look) => this.ctx.view.dimensionText(l, look), (b) => geometry.blockPieces(b));
    g.restore();
  }
}
