import { rulerTicks } from './rulerTicks';
import type { PaperViewport } from './paperView';

/**
 * The workspace's millimetre rulers (docs/sheet/design.md §11), drawn as the
 * SVG editor's are (DESIGN.md §7.14): panel-head ground, tertiary marks and
 * numbers, the paper's extent as an accent line along the inner edge, the
 * chosen items' extent as a soft band and the pointer's place as a thin
 * accent line. The horizontal ruler lies over the drawing area and shares
 * its x, the vertical one beside it shares its y (its numbers read upwards).
 */

export interface RulerColors {
  ground: string;
  edge: string;
  mark: string;
  number: string;
  accent: string;
  band: string;
  font: string;
}

export function readRulerColors(el: Element): RulerColors {
  const cs = getComputedStyle(el);
  const v = (name: string, fallback: string) => cs.getPropertyValue(name).trim() || fallback;
  return {
    ground: v('--c-panel-head', '#252e38'),
    edge: v('--c-line', '#2d3641'),
    mark: v('--c-text-3', '#6d7988'),
    number: v('--c-text-3', '#6d7988'),
    accent: v('--canvas-accent', '#f2b632'),
    band: v('--c-accent-soft', 'rgba(242,182,50,0.15)'),
    font: v('--font-ui', 'system-ui, sans-serif'),
  };
}

export interface RulerScene {
  readonly axis: 'h' | 'v';
  /** The ruler's length and thickness, CSS px. */
  readonly length: number;
  readonly thickness: number;
  readonly view: PaperViewport;
  /** The paper's size along this ruler, mm. */
  readonly paperMm: number;
  /** The pointer's place along this ruler, mm; null when it is off the paper area. */
  readonly cursor: number | null;
  /** The chosen items' extent along this ruler, mm. */
  readonly band: { readonly from: number; readonly to: number } | null;
  /** The type size of its numbers, CSS px (the interface's --fs-2xs). */
  readonly fontPx: number;
}

export function paintRuler(g: CanvasRenderingContext2D, dpr: number, s: RulerScene, c: RulerColors): void {
  const horizontal = s.axis === 'h';
  const T = s.thickness;
  // Along the ruler and across it (0 at the outer edge, T at the edge towards the paper).
  const xy = (along: number, across: number): [number, number] => (horizontal ? [along, across] : [across, along]);
  const crisp = (v: number) => (Math.round(v * dpr) + 0.5) / dpr;
  const origin = horizontal ? s.view.x : s.view.y;
  const at = (mm: number) => origin + mm * s.view.scale;
  const line = (a0: number, c0: number, a1: number, c1: number) => {
    g.moveTo(...xy(a0, c0));
    g.lineTo(...xy(a1, c1));
  };

  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = c.ground;
  g.fillRect(0, 0, ...xy(s.length, T));

  if (s.band) {
    g.fillStyle = c.band;
    const [x, y] = xy(at(s.band.from), 0);
    g.fillRect(x, y, ...xy((s.band.to - s.band.from) * s.view.scale, T));
  }

  const ticks = rulerTicks((0 - origin) / s.view.scale, (s.length - origin) / s.view.scale, s.view.scale);
  g.strokeStyle = c.mark;
  g.lineWidth = 1 / dpr;
  g.beginPath();
  for (const t of ticks) {
    const a = crisp(at(t.at));
    line(a, T, a, T - (t.level === 2 ? T * 0.62 : t.level === 1 ? T * 0.38 : T * 0.22));
  }
  g.stroke();

  g.fillStyle = c.number;
  g.font = `400 ${s.fontPx}px ${c.font}`;
  g.textBaseline = 'top';
  for (const t of ticks) {
    if (t.label === undefined) continue;
    if (horizontal) g.fillText(t.label, at(t.at) + 3, 2);
    else {
      // Read upwards, just below its mark.
      g.save();
      g.translate(2, at(t.at) + 3 + g.measureText(t.label).width);
      g.rotate(-Math.PI / 2);
      g.fillText(t.label, 0, 0);
      g.restore();
    }
  }

  // The paper's extent along the inner edge, in the accent.
  g.strokeStyle = c.accent;
  g.globalAlpha = 0.7;
  g.lineWidth = 2;
  g.beginPath();
  line(at(0), T - 1, at(s.paperMm), T - 1);
  g.stroke();
  g.globalAlpha = 1;

  if (s.cursor !== null) {
    const a = crisp(at(s.cursor));
    g.strokeStyle = c.accent;
    g.lineWidth = 1;
    g.beginPath();
    line(a, 0, a, T);
    g.stroke();
  }
  // The edge towards the paper area.
  g.strokeStyle = c.edge;
  g.lineWidth = 1 / dpr;
  g.beginPath();
  const e = T - 0.5 / dpr;
  line(0, e, s.length, e);
  g.stroke();
}
