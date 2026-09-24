import type { Pt } from '../../style/svg/pathData';
import { shapeId, type Guide } from '../../style/svg/svgModel';
import { h } from '../dom';
import { el, fmtNum, type CanvasView } from './svgView';

/**
 * Rulers along the top and left of the SVG editor's canvas, in drawing
 * units, and the guides pulled out of them (Inkscape's): drag from the top
 * ruler for a horizontal guide, from the left one for a vertical guide.
 * A guide is dragged to move it (snapping like a point) and dragged back
 * onto a ruler to delete it; a double click opens its position and angle,
 * with Sil. Guides are snap targets and live in the drawing (undo keeps
 * them), but the SVG file does not.
 */

export const RULER = 18;

type Op = { id: string; created: boolean; overRuler: boolean };

export class Rulers {
  private readonly view: CanvasView;
  private op: Op | null = null;
  private popup: HTMLElement | null = null;

  constructor(view: CanvasView) {
    this.view = view;
  }

  private get guides(): Guide[] {
    return this.view.host.doc.guides ?? [];
  }

  private screenXY(e: { clientX: number; clientY: number }): Pt {
    const r = this.view.stage.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  /** Which ruler a screen point is on. */
  private rulerAt(s: Pt): 'h' | 'v' | null {
    if (!this.view.host.options.rulers) return null;
    if (s[1] <= RULER && s[0] > RULER) return 'h';
    if (s[0] <= RULER && s[1] > RULER) return 'v';
    return null;
  }

  down(p: Pt, target: Element): boolean {
    const host = this.view.host;
    const guide = target.getAttribute('data-guide');
    const ruler = target.getAttribute('data-ruler') as 'h' | 'v' | null;
    if (!guide && !ruler) return false;
    this.closePopup();
    host.begin();
    if (guide) {
      this.op = { id: guide, created: false, overRuler: false };
      return true;
    }
    const id = shapeId();
    const q = this.view.snap(p, { guide: id });
    host.doc.guides = [...this.guides, { id, x: q[0], y: q[1], angle: ruler === 'h' ? 0 : 90 }];
    this.op = { id, created: true, overRuler: true };
    host.changed();
    return true;
  }

  move(e: PointerEvent, p: Pt): boolean {
    const op = this.op;
    if (!op) return false;
    const g = this.guides.find((x) => x.id === op.id);
    if (!g) return true;
    const q = this.view.snap(p, { guide: op.id });
    g.x = q[0];
    g.y = q[1];
    op.overRuler = !!this.rulerAt(this.screenXY(e));
    this.view.host.status(op.overRuler ? 'Bırakınca kılavuz silinir.' : `Kılavuz: ${g.angle === 0 ? `Y ${fmtNum(g.y)}` : g.angle === 90 ? `X ${fmtNum(g.x)}` : `${fmtNum(g.x)}, ${fmtNum(g.y)} · ${fmtNum(g.angle)}°`}`);
    this.view.host.changed();
    return true;
  }

  up(): boolean {
    const op = this.op;
    if (!op) return false;
    this.op = null;
    const host = this.view.host;
    if (op.overRuler) {
      host.doc.guides = this.guides.filter((g) => g.id !== op.id);
      if (!host.doc.guides.length) delete host.doc.guides;
    }
    host.commit(op.overRuler ? (op.created ? '' : 'Kılavuzu sil') : op.created ? 'Kılavuz ekle' : 'Kılavuzu taşı');
    host.status(op.overRuler || !op.created ? '' : 'Kılavuz eklendi: sürükleyerek taşıyın, cetvele geri bırakınca silinir, çift tık konumunu açar.');
    return true;
  }

  get busy(): boolean {
    return !!this.op;
  }

  /** Double click on a guide: its position and angle, typed. */
  dbl(e: MouseEvent, target: Element): boolean {
    const id = target.getAttribute('data-guide');
    const g = id ? this.guides.find((x) => x.id === id) : undefined;
    if (!g) return false;
    this.closePopup();
    const [sx, sy] = this.screenXY(e);
    const field = (label: string, value: number) => {
      const input = h('input', { class: 'field num', value: fmtNum(value), inputmode: 'decimal', 'aria-label': label, spellcheck: 'false' });
      return { input, row: h('label', { class: 'svge__gpfield' }, h('span', null, label), input) };
    };
    const x = field('X', g.x);
    const y = field('Y', g.y);
    const a = field('Açı (°)', g.angle);
    const apply = () => {
      const nums = [x, y, a].map((f) => Number(f.input.value.replace(',', '.')));
      if (nums.some((v) => !Number.isFinite(v))) return this.view.host.status('Kılavuz için sayı yazın (ondalık ayırıcı nokta).', 'warn');
      this.edit(g.id, (gg) => Object.assign(gg, { x: nums[0], y: nums[1], angle: ((nums[2] % 180) + 180) % 180 }), 'Kılavuz konumu');
      this.closePopup();
    };
    const ok = h('button', { class: 'btn btn--small btn--primary', type: 'button' }, 'Tamam');
    ok.addEventListener('click', apply);
    const del = h('button', { class: 'btn btn--small svge__danger', type: 'button' }, 'Sil');
    del.addEventListener('click', () => {
      this.remove(g.id);
      this.closePopup();
    });
    const pop = h('div', { class: 'svge__guidepop', role: 'dialog', 'aria-label': 'Kılavuz' }, h('div', { class: 'svge__gptitle' }, 'Kılavuz'), x.row, y.row, a.row, h('div', { class: 'svge__gpacts' }, del, ok));
    pop.style.left = `${Math.min(sx + 8, this.view.stage.clientWidth - 190)}px`;
    pop.style.top = `${Math.min(sy + 8, this.view.stage.clientHeight - 170)}px`;
    pop.addEventListener('keydown', (ev) => {
      ev.stopPropagation();
      if (ev.key === 'Enter') apply();
      if (ev.key === 'Escape') {
        ev.preventDefault();
        this.closePopup();
        this.view.stage.focus();
      }
    });
    pop.addEventListener('pointerdown', (ev) => ev.stopPropagation());
    this.view.stage.append(pop);
    this.popup = pop;
    // A horizontal guide is about its Y, a vertical one about its X.
    const first = (g.angle === 0 ? y : x).input;
    queueMicrotask(() => (first.focus(), first.select()));
    return true;
  }

  closePopup(): void {
    this.popup?.remove();
    this.popup = null;
  }

  private edit(id: string, fn: (g: Guide) => void, label: string): void {
    const host = this.view.host;
    const g = this.guides.find((x) => x.id === id);
    if (!g) return;
    host.begin();
    fn(g);
    host.commit(label);
  }

  remove(id: string): void {
    const host = this.view.host;
    host.begin();
    host.doc.guides = this.guides.filter((g) => g.id !== id);
    if (!host.doc.guides.length) delete host.doc.guides;
    host.commit('Kılavuzu sil');
  }

  clearAll(): void {
    const host = this.view.host;
    if (!this.guides.length) return;
    host.begin();
    delete host.doc.guides;
    host.commit('Kılavuzları sil');
  }

  // ── Drawing ──────────────────────────────────────────────────────────

  /** Guides across the whole stage (under the handles). */
  drawGuides(g: SVGElement): void {
    const w = this.view.stage.clientWidth;
    const ht = this.view.stage.clientHeight;
    const far = (w + ht) * 2;
    for (const gd of this.guides) {
      const [x, y] = this.view.toScreen([gd.x, gd.y]);
      const a = (gd.angle * Math.PI) / 180;
      const dx = Math.cos(a) * far;
      const dy = Math.sin(a) * far;
      const deleting = this.op?.id === gd.id && this.op.overRuler;
      g.append(el('line', { x1: x - dx, y1: y - dy, x2: x + dx, y2: y + dy, class: `svge__guide${deleting ? ' svge__guide--del' : ''}` }));
      g.append(el('line', { x1: x - dx, y1: y - dy, x2: x + dx, y2: y + dy, class: 'svge__guidehit', 'data-guide': gd.id }));
    }
  }

  /** The rulers (on top of everything but the menus). */
  drawRulers(g: SVGElement): void {
    if (!this.view.host.options.rulers) return;
    const v = this.view;
    const w = v.stage.clientWidth;
    const ht = v.stage.clientHeight;
    const z = v.scale;
    g.append(el('rect', { x: 0, y: 0, width: w, height: RULER, class: 'svge__ruler', 'data-ruler': 'h' }));
    g.append(el('rect', { x: 0, y: 0, width: RULER, height: ht, class: 'svge__ruler', 'data-ruler': 'v' }));
    g.append(el('rect', { x: 0, y: 0, width: RULER, height: RULER, class: 'svge__ruler svge__ruler--corner' }));
    // Majors at least 56 px apart on a 1-2-5 scale, minors a fifth (or half) of that.
    const major = niceStep(56 / z);
    const minor = major * z / 5 >= 5 ? major / 5 : major / 2;
    const o = v.toScreen([0, 0]);
    let d = '';
    const labels: SVGElement[] = [];
    const along = (from: number, to: number, origin: number, draw: (s: number, len: number, value: number) => void) => {
      const a = Math.ceil((from - origin) / z / minor);
      const b = Math.floor((to - origin) / z / minor);
      for (let k = a; k <= b && k - a < 2000; k++) {
        const value = k * minor;
        const isMajor = Math.abs(value / major - Math.round(value / major)) < 1e-6;
        draw(origin + value * z, isMajor ? RULER - 4 : 5, isMajor ? value : NaN);
      }
    };
    along(RULER, w, o[0], (s, len, value) => {
      d += `M${s.toFixed(1)} ${RULER}v${-len}`;
      if (Number.isFinite(value)) labels.push(textAt(s + 3, 9, fmtNum(value)));
    });
    along(RULER, ht, o[1], (s, len, value) => {
      d += `M${RULER} ${s.toFixed(1)}h${-len}`;
      if (Number.isFinite(value)) {
        const t = textAt(9, s - 3, fmtNum(value));
        t.setAttribute('transform', `rotate(-90 9 ${s - 3})`);
        labels.push(t);
      }
    });
    g.append(el('path', { d, class: 'svge__rtick' }));
    for (const l of labels) g.append(l);
    // The canvas's extent on the rulers.
    const [x1, y1] = v.toScreen([v.host.doc.width, v.host.doc.height]);
    g.append(el('path', { d: `M${Math.max(RULER, o[0])} ${RULER - 1}H${Math.max(RULER, x1)}M${RULER - 1} ${Math.max(RULER, o[1])}V${Math.max(RULER, y1)}`, class: 'svge__rextent' }));
  }
}

const textAt = (x: number, y: number, s: string) => {
  const t = el('text', { x, y, class: 'svge__rlabel' });
  t.textContent = s;
  return t;
};

/** The smallest 1-2-5 step not below `v`. */
export function niceStep(v: number): number {
  const e = 10 ** Math.floor(Math.log10(v));
  const m = v / e;
  return (m <= 1 ? 1 : m <= 2 ? 2 : m <= 5 ? 5 : 10) * e;
}
