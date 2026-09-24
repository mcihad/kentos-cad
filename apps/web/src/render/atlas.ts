import { cssColor, drawShape } from './canvasShapes';
import type { AtlasHit, AtlasImage, AtlasSource, AtlasUpload, TileMark } from './types';
import { TEXT_BOX } from './types';

/**
 * One texture page shared by the WebGL2 and WebGPU backends for
 * everything that is an image on the GPU: SVG and raster markers, text
 * markers and pattern tiles (MapLibre's sprite, in spirit).
 *
 * Images are drawn at the size they are shown, in power-of-two steps
 * (8 … 512 px), so they are sharp at every zoom and never shrink by more
 * than half: no mipmaps. Each new image is drawn into a small canvas and
 * handed to the active backend, which copies it into its texture; nothing
 * else is ever uploaded. A size still being made is stood in for by the
 * nearest size already there.
 *
 * The page fills with shelves; when it is full it starts over and the
 * frame asks again for what it draws. If one frame alone needs more than
 * the page, the largest step is halved for a while (a little blur rather
 * than a page redrawn every frame).
 */

const PAGE = 2048;
/** Transparent border around every image: linear filtering never reads a neighbour. */
const PAD = 2;
const MIN_STEP = 8;
const MAX_STEP = 512;
/** No side of one image is longer (a long text at a large step is scaled down). */
const MAX_SIDE = 1024;
/** Frames without an overflow before the largest step may grow again. */
const RELAX_FRAMES = 240;

interface Entry extends AtlasHit {
  step: number;
}

type Decoded = HTMLImageElement | 'loading' | 'failed';

const withXmlns = (svg: string) => (/\sxmlns\s*=/.test(svg) ? svg : svg.replace(/<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"'));

async function decodeSvg(svg: string): Promise<HTMLImageElement> {
  const url = URL.createObjectURL(new Blob([withXmlns(svg)], { type: 'image/svg+xml' }));
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return img;
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function decodeUrl(url: string): Promise<HTMLImageElement> {
  const img = new Image();
  img.src = url;
  await img.decode();
  return img;
}

/** The step for a wanted size: the next power of two, within the steps allowed now. */
function stepFor(px: number, cap: number): number {
  const p = Math.max(MIN_STEP, Math.min(cap, px));
  return Math.min(cap, 2 ** Math.ceil(Math.log2(p)));
}

export class Atlas implements AtlasSource {
  readonly size = PAGE;
  generation = 0;
  /** Called when an image finished decoding (the view redraws). */
  onChange: (() => void) | null = null;
  private upload: AtlasUpload | null = null;
  /** Placed images by image key and step. */
  private entries = new Map<string, Entry>();
  /** Steps placed per image key, for stand-ins. */
  private steps = new Map<string, number[]>();
  /** Decoded SVG and raster images by image key (kept across page resets). */
  private readonly decoded = new Map<string, Decoded>();
  private readonly scratch: HTMLCanvasElement;
  private readonly measure: CanvasRenderingContext2D;
  private shelfX = 0;
  private shelfY = 0;
  private shelfH = 0;
  private frame = 0;
  private resetFrame = -1;
  private cap = MAX_STEP;
  private lastOverflow = -Infinity;

  constructor() {
    this.scratch = document.createElement('canvas');
    this.measure = document.createElement('canvas').getContext('2d')!;
    // Text drawn before the UI font arrived used a fallback: draw everything again.
    document.fonts?.ready.then(() => {
      if (this.entries.size) {
        this.reset();
        this.onChange?.();
      }
    });
  }

  attach(upload: AtlasUpload | null): void {
    this.upload = upload;
    this.reset();
  }

  beginFrame(): void {
    this.frame++;
    if (this.cap < MAX_STEP && this.frame - this.lastOverflow > RELAX_FRAMES) {
      this.cap *= 2;
      this.lastOverflow = this.frame;
    }
  }

  lookup(image: AtlasImage, px: number): AtlasHit | null {
    if (!this.upload) return null;
    const step = stepFor(px, this.cap);
    const key = `${image.key}@${step}`;
    const hit = this.entries.get(key);
    if (hit) return hit;
    return this.make(image, step, key) ?? this.standIn(image.key, step);
  }

  /** The placed size nearest to `step` (larger first: shrinking looks better than growing). */
  private standIn(imageKey: string, step: number): AtlasHit | null {
    const have = this.steps.get(imageKey);
    if (!have?.length) return null;
    let best = have[0];
    for (const s of have) {
      if (s >= step) {
        if (best < step || s < best) best = s;
      } else if (best < step && s > best) best = s;
    }
    return this.entries.get(`${imageKey}@${best}`) ?? null;
  }

  private reset(): void {
    this.entries.clear();
    this.steps.clear();
    this.shelfX = this.shelfY = this.shelfH = 0;
    this.generation++;
  }

  /** A free spot for w×h px (shelf packing), starting the page over once per frame when full. */
  private alloc(w: number, h: number): [number, number] | null {
    const W = w + PAD * 2;
    const H = h + PAD * 2;
    if (W > PAGE || H > PAGE) return null;
    for (let attempt = 0; attempt < 2; attempt++) {
      if (this.shelfX + W > PAGE) {
        this.shelfY += this.shelfH;
        this.shelfX = 0;
        this.shelfH = 0;
      }
      if (this.shelfY + H <= PAGE) {
        const at: [number, number] = [this.shelfX, this.shelfY];
        this.shelfX += W;
        this.shelfH = Math.max(this.shelfH, H);
        return at;
      }
      if (this.resetFrame === this.frame) break;
      this.resetFrame = this.frame;
      this.reset();
    }
    // This frame needs more than a page: smaller steps for a while.
    this.cap = Math.max(32, this.cap / 2);
    this.lastOverflow = this.frame;
    return null;
  }

  /** Draws an image at a step and places it; null while its pixels are not available yet. */
  private make(image: AtlasImage, step: number, key: string): Entry | null {
    let w: number;
    let h: number;
    let paint: (g: CanvasRenderingContext2D) => void;
    switch (image.kind) {
      case 'text': {
        const size = step / TEXT_BOX;
        const font = `${image.italic ? 'italic ' : ''}${image.weight} ${size}px ${image.font}`;
        this.measure.font = font;
        const halo = image.halo ? image.halo.width * size : 0;
        const tw = this.measure.measureText(image.text).width + 2 * halo;
        const k = Math.min(1, MAX_SIDE / Math.max(tw, step));
        w = Math.max(1, Math.ceil(tw * k));
        h = Math.max(1, Math.ceil(step * k));
        paint = (g) => {
          g.scale(k, k);
          g.font = font;
          g.textBaseline = 'alphabetic';
          g.textAlign = 'left';
          const baseline = step * 0.78;
          if (image.halo) {
            g.lineJoin = 'round';
            g.lineWidth = halo * 2;
            g.strokeStyle = image.halo.color;
            g.strokeText(image.text, halo, baseline);
          }
          g.fillStyle = image.color;
          g.fillText(image.text, halo, baseline);
        };
        break;
      }
      case 'svg':
      case 'raster': {
        const img = this.decode(image);
        if (!img) return null;
        const aspect = image.height / Math.max(image.width, 1e-9);
        const k = Math.min(1, MAX_SIDE / Math.max(step, step * aspect));
        w = Math.max(1, Math.round(step * k));
        h = Math.max(1, Math.round(step * aspect * k));
        paint = (g) => g.drawImage(img, 0, 0, w, h);
        break;
      }
      case 'tile': {
        const decoded = new Map<string, HTMLImageElement>();
        let ready = true;
        for (const m of image.draw) {
          if (m.look.kind !== 'image' || m.look.image.kind === 'text' || m.look.image.kind === 'tile') continue;
          const img = this.decode(m.look.image);
          if (img) decoded.set(m.look.image.key, img);
          else ready = false;
        }
        if (!ready) return null;
        w = step;
        h = Math.max(2, Math.min(MAX_SIDE, Math.round(step * image.aspect)));
        paint = (g) => this.paintTile(g, image, w, h, decoded);
        break;
      }
    }
    const at = this.alloc(w, h);
    if (!at) return null;
    const c = this.scratch;
    c.width = w + PAD * 2;
    c.height = h + PAD * 2;
    const g = c.getContext('2d')!;
    g.save();
    g.translate(PAD, PAD);
    g.beginPath();
    g.rect(0, 0, w, h);
    g.clip();
    paint(g);
    g.restore();
    this.upload!(c, at[0], at[1]);
    const x = at[0] + PAD;
    const y = at[1] + PAD;
    const entry: Entry = { uv: [x / PAGE, y / PAGE, w / PAGE, h / PAGE], aspect: h / w, step };
    this.entries.set(key, entry);
    let list = this.steps.get(image.key);
    if (!list) this.steps.set(image.key, (list = []));
    list.push(step);
    return entry;
  }

  /** The decoded picture of an SVG or raster image, or null while it loads (the view redraws when it arrives). */
  private decode(image: Extract<AtlasImage, { kind: 'svg' | 'raster' }>): HTMLImageElement | null {
    const d = this.decoded.get(image.key);
    if (d instanceof HTMLImageElement) return d;
    if (d) return null;
    this.decoded.set(image.key, 'loading');
    (image.kind === 'svg' ? decodeSvg(image.svg) : decodeUrl(image.url))
      .then((img) => {
        this.decoded.set(image.key, img);
        this.onChange?.();
      })
      .catch(() => this.decoded.set(image.key, 'failed'));
    return null;
  }

  /** Pattern tile: each mark drawn at the cell centre (and half-shifted for staggered rows), wrapped at the edges so tiles join seamlessly. */
  private paintTile(g: CanvasRenderingContext2D, image: Extract<AtlasImage, { kind: 'tile' }>, W: number, H: number, decoded: Map<string, HTMLImageElement>): void {
    const centres: [number, number][] = image.stagger
      ? [
          [0.5, 0.25],
          [0, 0.75],
        ]
      : [[0.5, 0.5]];
    for (const m of image.draw)
      for (const [cx, cy] of centres)
        for (const dx of [-W, 0, W]) for (const dy of [-H, 0, H]) this.drawMark(g, m, cx * W + dx + m.offset[0] * W, cy * H + dy - m.offset[1] * W, W, decoded);
  }

  private drawMark(g: CanvasRenderingContext2D, m: TileMark, cx: number, cy: number, W: number, decoded: Map<string, HTMLImageElement>): void {
    const w = m.w * W;
    const h = (m.h || m.w) * W;
    g.save();
    g.translate(cx, cy);
    g.rotate(-m.rotation);
    g.scale(1, -1);
    if (m.look.kind === 'shape') drawShape(g, m.look, w, h, m.look.strokeWidth * W);
    else if (m.look.image.kind === 'text') {
      g.scale(1, -1);
      g.font = `${m.look.image.italic ? 'italic ' : ''}${m.look.image.weight} ${h}px ${m.look.image.font}`;
      g.textAlign = 'center';
      g.textBaseline = 'middle';
      g.fillStyle = m.look.image.color;
      g.fillText(m.look.image.text, 0, 0);
    } else {
      const img = decoded.get(m.look.image.key);
      if (img) {
        const ih = w * (img.naturalHeight / Math.max(img.naturalWidth, 1));
        g.scale(1, -1);
        g.drawImage(img, -w / 2, -ih / 2, w, ih);
      }
    }
    g.restore();
  }
}

export { cssColor };
