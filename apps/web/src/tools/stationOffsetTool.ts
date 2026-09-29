import type { Formatter } from '../app/format';
import { bearingGrad, dist, type Vec2 } from '../model/geometry';
import { sideOffsets } from '../model/geom/survey';
import type { ViewTransform } from '../viewport/Camera';
import { dotMark, indexMark, lineReach, rightAngleMark, ringMark } from './constructPreview';
import { PointInputTool } from './drawTools';
import { drawTag, strokePath } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Dik ayak ölç (docs/adr/0141; Netcad's Prizma): the foot and the offset of points against a line,
 * as a surveyor reads them off. The line is two clicks, A then B; each point clicked after that is
 * measured against it. The foot (dik ayak) is measured from A towards B, the offset (dik boy) square
 * to the line, right of A→B positive: the rule of Nokta hesapla's yan nokta, and `sideOffsets`, the
 * shared core's, computes both. Every click writes its reading to the log; nothing is written to the
 * drawing.
 */

/** An offset shorter than this (m) is on the line. */
export const ON_LINE = 0.0005;

export type Side = 'right' | 'left' | 'on';

export interface StationReading {
  /** Dik ayak: from A towards B (negative behind A). */
  foot: number;
  /** Dik boy: right of A→B positive. */
  offset: number;
  side: Side;
  /** The foot of the perpendicular, on the line. */
  footPoint: Vec2;
}

/** The reading of `p` against the line A→B; null when A and B are one point. */
export function readStation(a: Vec2, b: Vec2, p: Vec2): StationReading | null {
  const o = sideOffsets(a, b, p);
  const length = dist(a, b);
  if (!o || length === 0) return null;
  const footPoint = { x: a.x + ((b.x - a.x) / length) * o.absis, y: a.y + ((b.y - a.y) / length) * o.absis };
  return { foot: o.absis, offset: o.ordinat, side: Math.abs(o.ordinat) < ON_LINE ? 'on' : o.ordinat > 0 ? 'right' : 'left', footPoint };
}

const SIDE_WORD: Record<Side, string> = { right: 'sağda', left: 'solda', on: 'hat üzerinde' };

/** A foot as shown: never “-0.000”. */
const footOf = (r: StationReading) => (Math.abs(r.foot) < ON_LINE ? 0 : r.foot);

/** The log line of a reading, the offset unsigned and its side in words: `Dik ayak 30.000 m, dik boy 5.000 m (solda)`. */
export function stationLine(f: Pick<Formatter, 'length'>, r: StationReading): string {
  return `Dik ayak ${f.length(footOf(r))}, dik boy ${f.length(Math.abs(r.offset))} (${SIDE_WORD[r.side]})`;
}

/** The tag's two lines: `Ayak 30.000 m` and `Boy −5.000 m`, the offset signed (+ right, − left, U+2212; none on the line). */
export function stationTag(f: Pick<Formatter, 'length'>, r: StationReading): string[] {
  const sign = r.side === 'right' ? '+' : r.side === 'left' ? '−' : '';
  return [`Ayak ${f.length(footOf(r))}`, `Boy ${sign}${f.length(Math.abs(r.offset))}`];
}

/** How many measured points stay marked on the drawing: the latest ones. */
const MARKED = 60;

export class StationOffsetTool extends PointInputTool {
  readonly id = 'stationOffset';
  protected readonly label = 'Dik ayak ölç';
  /** The points measured against the line, marked until the line changes. */
  private measured: { p: Vec2; reading: StationReading }[] = [];

  protected promptFor(n: number): string {
    if (n === 0) return 'hattın başına tıklayın (A)';
    if (n === 1) return 'hattın sonuna tıklayın (B)';
    return 'ölçülecek noktaya tıklayın [Başka hat (H) / Bitir (Enter)]';
  }

  protected onPoint(p: Vec2): void {
    const [a, b] = this.pts;
    if (!a) return void this.pts.push(p);
    if (!b) {
      if (dist(a, p) <= 1e-9) return void this.ctx.log.warn('B noktası A ile çakışıyor; hattın sonu için başka bir nokta gösterin.');
      return void this.pts.push(p);
    }
    const reading = readStation(a, b, p);
    if (!reading) return;
    this.measured = [...this.measured.slice(1 - MARKED), { p, reading }];
    this.ctx.log.info(stationLine(this.ctx.format, reading));
  }

  protected override option(key: string): boolean {
    if (key !== 'H' || this.pts.length < 2) return false;
    this.reset();
    return true;
  }

  protected override reset(): void {
    this.measured = [];
    super.reset();
  }

  /** The points to measure are taken as shown (and snapped): ortho and polar tracking belong to the line's two ends. */
  protected override constrain(p: ToolPointer): Vec2 {
    if (this.pts.length < 2) return super.constrain(p);
    this.tracking = null;
    return p.world;
  }

  /** Perpendicular snaps refer to the line's ends only. */
  override snapFrom(): Vec2 | null {
    return this.pts.length < 2 ? super.snapFrom() : null;
  }

  /** Right click or Enter ends the tool, whichever step it is at. */
  override confirm(): void {
    this.pts = [];
    this.ctx.tools.exit();
  }

  /** Esc steps back: the points to the line's start (a new line), B to A, A leaves. */
  cancel(): boolean {
    if (!this.pts.length) return false;
    this.reset();
    return true;
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const [a, b] = this.pts;
    const at = this.hover;
    if (!a) return this.drawTracking(g, view);
    if (!b) {
      ringMark(g, view, a, pal.snap);
      indexMark(g, view, a, 'A', pal.snap, pal.labelHalo);
      if (at && dist(a, at) > 1e-9) {
        strokePath(g, view, [a, at], { color: pal.accent });
        drawTag(g, view.worldToScreen(at), [f.length(dist(a, at)), `Semt ${f.bearing(bearingGrad(a, at))}`], pal.accent, pal.labelHalo);
      }
      return this.drawTracking(g, view);
    }
    // The line, and its extension across the view: the foot may lie beyond either end.
    const length = dist(a, b);
    const u = { x: (b.x - a.x) / length, y: (b.y - a.y) / length };
    const reach = lineReach(this.ctx);
    strokePath(g, view, [{ x: a.x - u.x * reach, y: a.y - u.y * reach }, a], { color: pal.snap, dash: [6, 4] });
    strokePath(g, view, [b, { x: b.x + u.x * reach, y: b.y + u.y * reach }], { color: pal.snap, dash: [6, 4] });
    strokePath(g, view, [a, b], { color: pal.accent, width: 1.5 });
    ringMark(g, view, a, pal.snap);
    ringMark(g, view, b, pal.snap);
    indexMark(g, view, a, 'A', pal.snap, pal.labelHalo);
    indexMark(g, view, b, 'B', pal.snap, pal.labelHalo);
    for (const m of this.measured) {
      this.drawReading(g, view, u, m.p, m.reading, pal.snap);
      ringMark(g, view, m.p, pal.snap, 4);
    }
    const live = at ? readStation(a, b, at) : null;
    if (at && live) {
      this.drawReading(g, view, u, at, live, pal.accent);
      drawTag(g, view.worldToScreen(at), stationTag(f, live), pal.accent, pal.labelHalo);
    }
  }

  /** One point against the line: its foot, the dashed perpendicular to it and the square corner between them. */
  private drawReading(g: CanvasRenderingContext2D, view: ViewTransform, along: Vec2, p: Vec2, r: StationReading, color: string): void {
    const halo = this.ctx.view.palette.labelHalo;
    const gap = dist(r.footPoint, p);
    if (r.side !== 'on' && gap > 1e-9) {
      strokePath(g, view, [r.footPoint, p], { color, dash: [4, 3] });
      rightAngleMark(g, view, r.footPoint, along, { x: (p.x - r.footPoint.x) / gap, y: (p.y - r.footPoint.y) / gap }, color);
    }
    dotMark(g, view, r.footPoint, color, halo);
  }
}
