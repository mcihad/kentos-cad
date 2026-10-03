import type { ArcSeg } from '../../contracts/generated/sheet/ArcSeg';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { MapPrim } from '../../contracts/generated/sheet/MapPrim';
import type { Prim } from '../../contracts/generated/sheet/Prim';
import type { RectUm } from '../../contracts/generated/sheet/RectUm';
import type { Seg } from '../../contracts/generated/sheet/Seg';
import type { Stroke } from '../../contracts/generated/sheet/Stroke';
import type { TextPrim } from '../../contracts/generated/sheet/TextPrim';

/**
 * Paints a sheet's display list with Canvas2D (docs/sheet/design.md §8): the
 * engine has laid out every line of text, every grid line and label, the
 * scale bar's parts, the north arrow, the tables and the title block; this
 * paints them as the SVG writer of the engine writes them (crates/shared/
 * sheet/src/svg.rs is the reference): a rectangle turned about its own
 * centre, a text line from the start of its baseline turned about it, arcs
 * clockwise on the paper, clips and group opacities nested. Two things are
 * the host's: the pictures' bytes (`image`) and a map frame's content
 * (`map`: the project's drawing as the render pipeline draws it at the map's
 * view, already turned by the map's own rotation; it fills the content's
 * rectangle and turns with the frame).
 *
 * Text is set in the drawing's typefaces at the width the engine measured
 * (`TextPrim.width`, without kerning): the browser's own width is pulled to
 * it, so a right-aligned cell ends where the engine put its end.
 */

export interface PaintSources {
  /** A picture by its SHA-256, decoded; null while it loads or when this device lacks it (a grey box stands in). */
  image(sha: string): CanvasImageSource | null;
  /**
   * A map frame's content, at least `pxPerUm` device pixels per micrometre of
   * paper; null while it is being drawn (a light box stands in).
   */
  map(prim: MapPrim, pxPerUm: number): CanvasImageSource | null;
  /** A `DRAWING_FONTS` id as a CSS font family list. */
  family(font: string): string;
}

export interface PaintTarget {
  /** Device pixels per micrometre of paper. */
  readonly scale: number;
  /** Where the paper's top left corner is, device pixels. */
  readonly x: number;
  readonly y: number;
  /**
   * The thinnest a line is drawn, device pixels: on the screen a hairline stays
   * visible at any zoom (0.75); an export draws every width as it is (0).
   */
  readonly minLinePx: number;
  /** Text smaller than this (device pixels) is left out on the screen; 0 draws all. */
  readonly minTextPx: number;
  /** Placeholder words in the engine's colours (“harita”, a missing picture's box). */
  readonly placeholderFont: string;
}

const RAD = Math.PI / 180_000;

/** Paints the paper and every primitive of the list. The caller's transform and state are kept. */
export function paintList(g: CanvasRenderingContext2D, list: DisplayList, t: PaintTarget, src: PaintSources): void {
  g.save();
  setUm(g, t);
  g.fillStyle = list.paper;
  g.fillRect(0, 0, list.size.width, list.size.height);
  let depth = 0;
  for (const p of list.prims) {
    switch (p.type) {
      case 'rect':
        paintRect(g, p, t);
        break;
      case 'path':
        paintPath(g, p.segments, p.fill, p.stroke, p.evenOdd, t);
        break;
      case 'text':
        paintText(g, p, t, src);
        setUm(g, t);
        break;
      case 'image':
        paintImage(g, p.rect, p.rotation, p.opacity, src.image(p.asset), t);
        break;
      case 'map':
        paintMap(g, p, t, src);
        break;
      case 'pushClip':
        g.save();
        depth++;
        // The path is made turned (a path keeps the transform it was made under), the clip under plain paper units.
        turnAbout(g, p.rect, p.rotation);
        g.beginPath();
        g.rect(p.rect.left, p.rect.top, p.rect.width, p.rect.height);
        setUm(g, t);
        g.clip();
        break;
      case 'pushGroup':
        g.save();
        depth++;
        g.globalAlpha *= Math.max(0, Math.min(100, p.opacity)) / 100;
        break;
      case 'popClip':
      case 'popGroup':
        if (depth > 0) {
          g.restore();
          depth--;
        }
        break;
    }
  }
  while (depth-- > 0) g.restore();
  g.restore();
}

/** Micrometres of paper as the drawing's units. */
function setUm(g: CanvasRenderingContext2D, t: PaintTarget): void {
  g.setTransform(t.scale, 0, 0, t.scale, t.x, t.y);
}

/** Turns about a rectangle's centre (millidegrees, clockwise on the paper). */
function turnAbout(g: CanvasRenderingContext2D, r: RectUm, rotation: number): void {
  if (!(rotation % 360_000)) return;
  const cx = r.left + r.width / 2;
  const cy = r.top + r.height / 2;
  g.translate(cx, cy);
  g.rotate(rotation * RAD);
  g.translate(-cx, -cy);
}

function applyStroke(g: CanvasRenderingContext2D, s: Stroke, t: PaintTarget): void {
  g.strokeStyle = s.color;
  g.lineWidth = Math.max(Math.max(1, s.width), t.minLinePx / t.scale);
  g.setLineDash(s.dash.length ? s.dash : []);
  g.lineCap = s.cap;
  g.lineJoin = s.join;
}

function paintRect(g: CanvasRenderingContext2D, p: Extract<Prim, { type: 'rect' }>, t: PaintTarget): void {
  if (!p.fill && !p.stroke) return;
  g.save();
  turnAbout(g, p.rect, p.rotation);
  g.beginPath();
  const r = p.rect;
  if (p.radius > 0) g.roundRect(r.left, r.top, r.width, r.height, Math.min(p.radius, r.width / 2, r.height / 2));
  else g.rect(r.left, r.top, r.width, r.height);
  if (p.fill) {
    g.fillStyle = p.fill;
    g.fill();
  }
  if (p.stroke) {
    applyStroke(g, p.stroke, t);
    g.stroke();
  }
  g.restore();
}

function arc(g: CanvasRenderingContext2D, a: ArcSeg): void {
  const start = a.start * RAD;
  const sweep = Math.max(-360_000, Math.min(360_000, a.sweep)) * RAD;
  g.ellipse(a.center[0], a.center[1], Math.max(0, a.radius[0]), Math.max(0, a.radius[1]), a.rotation * RAD, start, start + sweep, sweep < 0);
}

function tracePath(g: CanvasRenderingContext2D, segs: readonly Seg[]): void {
  g.beginPath();
  for (const s of segs) {
    if (s === 'z') g.closePath();
    else if ('m' in s) g.moveTo(s.m[0], s.m[1]);
    else if ('l' in s) g.lineTo(s.l[0], s.l[1]);
    else arc(g, s.a);
  }
}

function paintPath(g: CanvasRenderingContext2D, segs: readonly Seg[], fill: string | undefined, stroke: Stroke | undefined, evenOdd: boolean, t: PaintTarget): void {
  if (!fill && !stroke) return;
  tracePath(g, segs);
  if (fill) {
    g.fillStyle = fill;
    g.fill(evenOdd ? 'evenodd' : 'nonzero');
  }
  if (stroke) {
    applyStroke(g, stroke, t);
    g.stroke();
  }
}

/** A text line: device pixels around its baseline's start, so letters are set at the size they show. */
function paintText(g: CanvasRenderingContext2D, p: TextPrim, t: PaintTarget, src: PaintSources): void {
  const px = p.size * t.scale;
  if (!p.text || px < t.minTextPx) return;
  const x = t.x + p.at[0] * t.scale;
  const y = t.y + p.at[1] * t.scale;
  g.setTransform(1, 0, 0, 1, x, y);
  if (p.rotation % 360_000) g.rotate(p.rotation * RAD);
  g.font = `${p.italic ? 'italic ' : ''}${p.weight} ${px}px ${src.family(p.font)}`;
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  g.fontKerning = 'none';
  // The engine's measure, without kerning: the browser's own is pulled to it (a few per cent at most).
  const want = p.width * t.scale;
  const have = want > 0 ? g.measureText(p.text).width : 0;
  const k = have > 0 ? want / have : 1;
  if (k > 0.8 && k < 1.25 && Math.abs(k - 1) > 0.002) g.scale(k, 1);
  if (p.halo) {
    g.strokeStyle = p.halo;
    g.lineWidth = Math.max(1, px / 5);
    g.lineJoin = 'round';
    g.setLineDash([]);
    g.strokeText(p.text, 0, 0);
  }
  g.fillStyle = p.color;
  g.fillText(p.text, 0, 0);
}

function paintImage(g: CanvasRenderingContext2D, r: RectUm, rotation: number, opacity: number, img: CanvasImageSource | null, t: PaintTarget): void {
  g.save();
  turnAbout(g, r, rotation);
  if (img) {
    g.globalAlpha *= Math.max(0, Math.min(100, opacity)) / 100;
    g.imageSmoothingQuality = 'high';
    g.drawImage(img, r.left, r.top, r.width, r.height);
  } else {
    // A picture this device lacks (or still decodes): the engine's grey box (svg.rs).
    g.fillStyle = '#f2f2f2';
    g.fillRect(r.left, r.top, r.width, r.height);
    g.strokeStyle = '#b4b4b4';
    g.lineWidth = Math.max(180, t.minLinePx / t.scale);
    g.setLineDash([]);
    g.strokeRect(r.left, r.top, r.width, r.height);
  }
  g.restore();
}

function paintMap(g: CanvasRenderingContext2D, p: MapPrim, t: PaintTarget, src: PaintSources): void {
  const r = p.clip;
  if (r.width <= 0 || r.height <= 0) return;
  const img = p.view.center ? src.map(p, t.scale) : null;
  g.save();
  turnAbout(g, r, p.rotation);
  if (img) {
    g.imageSmoothingQuality = 'high';
    g.drawImage(img, r.left, r.top, r.width, r.height);
    g.restore();
    return;
  }
  // No picture yet, or no place chosen: the engine's grey box with “harita” (svg.rs placeholders).
  g.fillStyle = '#ececec';
  g.fillRect(r.left, r.top, r.width, r.height);
  const size = Math.max(2000, Math.min(12_000, Math.min(r.width, r.height) / 12));
  if (size * t.scale >= Math.max(4, t.minTextPx)) {
    g.fillStyle = '#a0a0a0';
    g.font = `${size}px ${t.placeholderFont}`;
    g.textAlign = 'center';
    g.textBaseline = 'alphabetic';
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2 + size / 3;
    g.fillText('harita', cx, cy);
    if (!p.view.center) {
      g.font = `${size * 0.55}px ${t.placeholderFont}`;
      g.fillText('yeri seçilmedi', cx, cy + size * 0.9);
    }
  }
  g.restore();
}

/** The device pixels a map frame's content needs at a paint's scale (whole, at least one). */
export function mapPixels(p: MapPrim, pxPerUm: number): { width: number; height: number } {
  return { width: Math.max(1, Math.round(p.clip.width * pxPerUm)), height: Math.max(1, Math.round(p.clip.height * pxPerUm)) };
}

/** The fonts a list sets its text in (`weight italic family`), to load before painting. */
export function fontsOf(list: DisplayList, family: (font: string) => string): string[] {
  const out = new Set<string>();
  for (const p of list.prims) if (p.type === 'text') out.add(`${p.italic ? 'italic ' : ''}${p.weight} 16px ${family(p.font)}`);
  return [...out];
}
