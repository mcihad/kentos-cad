import type { AppContext } from '../../app/context';
import { surveyForward, surveyResection } from '../../model/geom/surveyCalc';
import type { Vec2 } from '../../model/geometry';
import { h, replaceChildren } from '../dom';
import { segmented, textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { addPoints, copyReport, field, knownField, layerChoice, readNumber, resolvePoint, resultTable, summary, summaryLine, type Picker } from './common';

/**
 * Önden ve geriden kestirme. Önden: angles measured at two known points A
 * and B towards the new point (α at A clockwise from B to P, β at B
 * clockwise from P to A: P lies right of A → B). Geriden: at the new point,
 * angles towards three known points seen left to right (α from A to B, β
 * from B to C). A sketch shows which angle is which.
 */
export type IntersectionKind = 'forward' | 'resection';

export function openIntersection(ctx: AppContext, kind: IntersectionKind): void {
  state.kind = kind;
  new IntersectionDialog(ctx);
}

const state = {
  kind: 'forward' as IntersectionKind,
  a: { text: '' },
  b: { text: '' },
  c: { text: '' },
  alpha: '',
  beta: '',
  name: '',
  layer: null as string | null,
};

const TITLE: Record<IntersectionKind, string> = { forward: 'Önden kestirme', resection: 'Geriden kestirme' };

/** The angles drawn: which point is where and which angle is which (not to scale). */
const SKETCH: Record<IntersectionKind, string> = {
  forward: `<svg viewBox="0 0 200 118" font-size="13" aria-hidden="true"><path d="M30 24 170 24M30 24 104 96M170 24 104 96" fill="none" stroke="currentColor" stroke-width="1.3"/>
<path d="M54 24A24 24 0 0 1 47.2 40.7" fill="none" stroke="var(--c-accent)" stroke-width="1.6"/><path d="M146 24A24 24 0 0 0 153.6 41.4" fill="none" stroke="var(--c-accent)" stroke-width="1.6"/>
<rect x="27" y="21" width="6" height="6" fill="currentColor"/><rect x="167" y="21" width="6" height="6" fill="currentColor"/><circle cx="104" cy="96" r="4" fill="none" stroke="currentColor" stroke-width="1.4"/>
<text x="16" y="18">A</text><text x="178" y="18">B</text><text x="112" y="112">P</text><text x="58" y="44" fill="var(--c-accent)">α</text><text x="132" y="46" fill="var(--c-accent)">β</text></svg>`,
  resection: `<svg viewBox="0 0 200 118" font-size="13" aria-hidden="true"><path d="M100 100 30 22M100 100 100 14M100 100 170 22" fill="none" stroke="currentColor" stroke-width="1.3"/>
<path d="M86.6 85.1A20 20 0 0 1 100 80" fill="none" stroke="var(--c-accent)" stroke-width="1.6"/><path d="M100 76A24 24 0 0 1 116 82.1" fill="none" stroke="var(--c-accent)" stroke-width="1.6"/>
<rect x="27" y="19" width="6" height="6" fill="currentColor"/><rect x="97" y="11" width="6" height="6" fill="currentColor"/><rect x="167" y="19" width="6" height="6" fill="currentColor"/><circle cx="100" cy="100" r="4" fill="none" stroke="currentColor" stroke-width="1.4"/>
<text x="16" y="16">A</text><text x="106" y="12">B</text><text x="178" y="16">C</text><text x="108" y="114">P</text><text x="80" y="76" fill="var(--c-accent)">α</text><text x="112" y="74" fill="var(--c-accent)">β</text></svg>`,
};

class IntersectionDialog implements Picker {
  readonly ctx: AppContext;
  private readonly form = h('div', { class: 'calc-form' });
  private readonly results = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly add = h('button', { class: 'btn btn--primary', type: 'button' }, 'Çizime ekle');
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly dialog: Dialog;
  private result: { p: Vec2; name: string; strength: number | null } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    const layer = layerChoice(ctx, state, 'poligon');
    this.dialog = new Dialog({
      title: 'Kestirme',
      width: 820,
      className: 'dialog--io dialog--calc',
      content: [this.form, this.summaryBox, this.results],
      footer: [h('span', { class: 'io-status' }), this.copy, h('span', { class: 'calc-foot-label' }, 'Katman'), layer, this.add, close],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.add.addEventListener('click', () => this.addToDrawing());
    this.copy.addEventListener('click', () => this.copyReport());
    this.render();
  }

  get title(): string {
    return TITLE[state.kind];
  }

  close(): void {
    this.dialog.close();
  }

  reopen(): void {
    openIntersection(this.ctx, state.kind);
  }

  private render(): void {
    const recompute = () => this.recompute();
    const unit = this.ctx.format.angleUnitLabel === '°' ? '°' : 'g';
    const fwd = state.kind === 'forward';
    const kind = segmented<IntersectionKind>({
      label: 'Kestirme türü',
      options: [
        { value: 'forward', label: 'Önden', hint: 'İki bilinen noktada yeni noktaya açı ölçüldü.' },
        { value: 'resection', label: 'Geriden', hint: 'Yeni noktada üç bilinen noktaya açı ölçüldü.' },
      ],
      value: state.kind,
      onChange: (k) => ((state.kind = k), this.render()),
    });
    const num = (label: string, key: 'alpha' | 'beta' | 'name', placeholder = '') => {
      const f = textField({ label, value: state[key], placeholder, onChange: (v) => ((state[key] = v), recompute()) });
      if (key !== 'name') f.classList.add('calc-num');
      return field(label, f);
    };
    const sketch = h('div', { class: 'calc-sketch' });
    sketch.innerHTML = SKETCH[state.kind];
    replaceChildren(
      this.form,
      h('div', { class: 'io-row' }, field('Kestirme türü', kind)),
      h(
        'div',
        { class: 'calc-split' },
        h(
          'div',
          { class: 'calc-knowns calc-knowns--col' },
          knownField(this, 'A noktası', state.a, recompute, 'a'),
          knownField(this, 'B noktası', state.b, recompute, 'b'),
          fwd ? null : knownField(this, 'C noktası', state.c, recompute, 'c'),
          h(
            'div',
            { class: 'io-row' },
            num(fwd ? `α: A'da B'den P'ye (${unit})` : `α: P'de A'dan B'ye (${unit})`, 'alpha'),
            num(fwd ? `β: B'de P'den A'ya (${unit})` : `β: P'de B'den C'ye (${unit})`, 'beta'),
            num('Yeni noktanın adı', 'name', 'P'),
          ),
        ),
        h(
          'div',
          { class: 'calc-sketch-box' },
          sketch,
          h('p', { class: 'io-field__hint' }, fwd ? 'Açılar saat yönünde ölçülür; P, A\'dan B\'ye bakınca sağdadır.' : 'Bilinen noktalar P\'den bakınca soldan sağa A, B, C sırasındadır; açılar saat yönündedir.'),
        ),
      ),
    );
    this.recompute();
  }

  private recompute(): void {
    const { ctx } = this;
    this.result = null;
    const errors: string[] = [];
    const known = (s: { text: string }, label: string): Vec2 | null => {
      const r = resolvePoint(ctx, s.text);
      if (!r) errors.push(`${label} verilmedi.`);
      else if ('error' in r) errors.push(`${label}: ${r.error}`);
      else return r.p;
      return null;
    };
    const fwd = state.kind === 'forward';
    const a = known(state.a, 'A noktası');
    const b = known(state.b, 'B noktası');
    const c = fwd ? null : known(state.c, 'C noktası');
    const alpha = readNumber(state.alpha);
    const beta = readNumber(state.beta);
    if (alpha === null || Number.isNaN(alpha)) errors.push('α açısını yazın.');
    if (beta === null || Number.isNaN(beta)) errors.push('β açısını yazın.');
    const name = state.name.trim() || 'P';
    const unit = ctx.doc.settings.angleUnit.value;
    if (!errors.length && a && b && alpha !== null && beta !== null) {
      try {
        if (fwd) this.result = { p: surveyForward(unit, a, b, alpha, beta), name, strength: null };
        else if (c) {
          const r = surveyResection(unit, a, b, c, alpha, beta);
          this.result = { p: r.p, name, strength: r.strength };
        }
      } catch (e) {
        errors.push((e as Error).message);
      }
    }
    const res = this.result;
    this.add.disabled = !res;
    this.copy.disabled = !res;
    if (!res) {
      summary(this.summaryBox, errors.slice(0, 6).map((e) => summaryLine('warn', e)));
      replaceChildren(this.results);
      return;
    }
    const f = ctx.format;
    summary(this.summaryBox, [
      summaryLine('ok', `${name} noktası hesaplandı.`),
      res.strength !== null && res.strength < 0.05
        ? summaryLine('warn', 'Nokta tehlike dairesine yakın (A, B, C ve durulan nokta neredeyse aynı çember üzerinde): küçük açı hataları konumu çok değiştirir. Başka bir bilinen noktayla denetleyin.')
        : null,
    ]);
    replaceChildren(this.results, resultTable(['Nokta', 'Y (sağa)', 'X (yukarı)'], [[name, f.coord(res.p.x), f.coord(res.p.y)]], [false, true, true]));
  }

  private addToDrawing(): void {
    const res = this.result;
    if (!res || !state.layer) return;
    const n = addPoints(this.ctx, state.layer, [{ name: res.name, p: res.p }], 'Kestirme noktası', this.title);
    if (n === null) return;
    this.ctx.log.success(`${this.title}: ${res.name} noktası çizime eklendi (Ctrl+Z geri alır).`);
    this.dialog.close();
  }

  private copyReport(): void {
    const res = this.result;
    if (!res) return;
    const f = this.ctx.format;
    copyReport(this.ctx, this.title, [[this.title], ['Nokta', 'Y', 'X'], [res.name, f.coord(res.p.x), f.coord(res.p.y)], ['α', state.alpha, 'β', state.beta]]);
  }
}
