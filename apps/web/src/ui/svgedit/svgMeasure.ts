import { bez, segmentCount, segmentCubic, segmentLength } from '../../style/svg/bezier';
import type { Pt } from '../../style/svg/pathData';
import { toPath } from '../../style/svg/svgModel';
import { el, fmtNum, tag, type CanvasView } from './svgView';

/**
 * The measure tool (M) of the SVG editor: between two snapped points
 * (drag, or click and click) it shows the distance and the angle, in
 * drawing units and in millimetres at the symbol's size (the document's
 * "1 birim = … mm"). With nothing being measured, the path under the
 * pointer shows the length of each of its segments.
 */

type State = 'idle' | 'placing' | 'done';

export class Measure {
  private readonly view: CanvasView;
  private state: State = 'idle';
  private pressed = false;
  private a: Pt | null = null;
  private b: Pt | null = null;
  private hoverId: string | null = null;

  constructor(view: CanvasView) {
    this.view = view;
  }

  reset(): void {
    this.state = 'idle';
    this.a = this.b = null;
    this.hoverId = null;
  }

  down(p: Pt): void {
    if (this.state === 'placing' && this.a) {
      this.b = this.view.snap(p, { from: this.a, noGrid: true });
      this.state = 'done';
    } else {
      this.a = this.view.snap(p, { noGrid: true });
      this.b = this.a;
      this.state = 'placing';
    }
    this.pressed = true;
    this.report();
    this.view.render();
  }

  move(p: Pt, target: Element): void {
    if (this.state === 'placing' && this.a) {
      this.b = this.view.snap(p, { from: this.a, noGrid: true });
      this.report();
    } else if (this.state !== 'done') {
      const id = target.closest('[data-id]')?.getAttribute('data-id') ?? null;
      if (id === this.hoverId) return;
      this.hoverId = id;
    }
    this.view.render();
  }

  up(): void {
    // A drag measures on release; a click waits for the second click.
    if (this.pressed && this.state === 'placing' && this.a && this.b && Math.hypot(this.b[0] - this.a[0], this.b[1] - this.a[1]) * this.view.scale > 4) this.state = 'done';
    this.pressed = false;
  }

  cancel(): boolean {
    if (this.state === 'idle') return false;
    this.reset();
    this.view.host.status('');
    this.view.render();
    return true;
  }

  private text(): { main: string; more: string } | null {
    const { a, b } = this;
    if (!a || !b) return null;
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const d = Math.hypot(dx, dy);
    // Angle as on paper: counter-clockwise from the right (the drawing's y runs down).
    const ang = ((Math.atan2(-dy, dx) * 180) / Math.PI + 360) % 360;
    const mm = this.mm(d);
    return {
      main: `${fmtNum(d)} birim${mm ? ` · ${mm}` : ''} · ${fmtNum(ang, 2)}°`,
      more: `ΔX ${fmtNum(dx)}, ΔY ${fmtNum(dy)}`,
    };
  }

  /** A length at the symbol's size, or nothing when the drawing has no size in mm. */
  private mm(units: number): string {
    const doc = this.view.host.doc;
    return doc.sizeMm ? `${fmtNum((units * doc.sizeMm) / doc.width, 2)} mm` : '';
  }

  private report(): void {
    const t = this.text();
    if (!t) return;
    const hint = this.view.host.doc.sizeMm ? '' : ' · mm için Dosya → Belge özellikleri’nden sembol boyunu verin';
    this.view.host.status(`Ölçü: ${t.main} · ${t.more}${hint}`);
  }

  draw(g: SVGElement): void {
    const v = this.view;
    const t = (p: Pt) => v.toScreen(p);
    if (this.a && this.b && this.state !== 'idle') {
      const [x1, y1] = t(this.a);
      const [x2, y2] = t(this.b);
      g.append(el('line', { x1, y1, x2, y2, class: 'svge__measure' }));
      for (const [x, y] of [
        [x1, y1],
        [x2, y2],
      ])
        g.append(el('circle', { cx: x, cy: y, r: 3, class: 'svge__measurept' }));
      const txt = this.text();
      if (txt) {
        const mx = (x1 + x2) / 2 + 10;
        const my = (y1 + y2) / 2 - 10;
        g.append(tag(mx, my, txt.main, 'svge__tag svge__tag--measure'));
        g.append(tag(mx, my + 15, txt.more, 'svge__tag svge__tag--measure'));
      }
      return;
    }
    // The hovered path's segment lengths.
    const s = this.hoverId ? v.host.doc.shapes.find((x) => x.id === this.hoverId) : undefined;
    const p = s ? toPath(s) : null;
    if (!p || p.kind !== 'path') return;
    let count = 0;
    let total = 0;
    for (const sp of p.subs)
      for (let i = 0; i < segmentCount(sp); i++) {
        const len = segmentLength(sp, i);
        total += len;
        if (count++ > 80) continue;
        const c = segmentCubic(sp, i);
        const [x, y] = t(bez(c, 0.5));
        if (len * v.scale < 24) continue;
        g.append(tag(x + 4, y - 4, fmtNum(len, 2), 'svge__tag svge__tag--seg'));
      }
    const mm = this.mm(total);
    v.host.status(`Yol uzunluğu ${fmtNum(total)} birim${mm ? ` (${mm})` : ''}, ${count} parça. İki noktayı ölçmek için tıklayıp sürükleyin.`);
  }
}
