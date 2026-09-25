import type { AppContext } from '../../app/context';
import { surveyStakeout, type Stake } from '../../model/geom/surveyCalc';
import type { Vec2 } from '../../model/geometry';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import {
  angleText,
  copyReport,
  Grid,
  knownField,
  resolvePoint,
  resultTable,
  summary,
  summaryLine,
  type GridModel,
  type Picker,
  type Row,
} from './common';

/**
 * Aplikasyon: the values to set out known points from a station: bearing
 * (semt) and horizontal distance to each (the second fundamental task) and,
 * with a back point, the angle to turn clockwise from it. Nothing is added
 * to the drawing; the report is copied for the field.
 */
export function openStakeout(ctx: AppContext): void {
  new StakeoutDialog(ctx);
}

const state = {
  station: { text: '' },
  back: { text: '' },
  rows: [{}, {}, {}] as Row[],
};

const TITLE = 'Aplikasyon';

class StakeoutDialog implements Picker {
  readonly ctx: AppContext;
  readonly title = TITLE;
  private readonly form = h('div', { class: 'calc-form' });
  private readonly table = h('div', { class: 'calc-section' });
  private readonly results = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly copy = h('button', { class: 'btn btn--primary', type: 'button' }, 'Raporu kopyala');
  private readonly dialog: Dialog;
  private grid: Grid | null = null;
  private result: { stakes: Stake[]; names: string[] } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: TITLE,
      width: 860,
      className: 'dialog--io dialog--calc',
      content: [this.form, this.table, this.summaryBox, this.results],
      footer: [h('span', { class: 'io-status' }), this.copy, close],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.copy.addEventListener('click', () => this.copyReport());
    this.render();
  }

  close(): void {
    this.dialog.close();
  }

  reopen(): void {
    openStakeout(this.ctx);
  }

  private render(): void {
    const recompute = () => this.recompute();
    replaceChildren(
      this.form,
      h(
        'div',
        { class: 'calc-knowns' },
        knownField(this, 'Durulan nokta (istasyon)', state.station, recompute, 'station'),
        knownField(this, 'Bakılan nokta', state.back, recompute, 'back', 'Verilirse ondan dönülecek açı da hesaplanır'),
      ),
    );
    const model: GridModel = {
      columns: [{ key: 'point', label: 'Aplike edilecek nokta (ad ya da Y,X)' }],
      rows: () => state.rows,
      readonly: () => false,
      canInsertAfter: () => true,
      insertAfter: (r) => state.rows.splice(r + 1, 0, {}),
      canRemove: () => state.rows.length > 1,
      remove: (r) => state.rows.splice(r, 1),
    };
    this.grid = new Grid(model, recompute);
    const fromSelection = h('button', { class: 'btn', type: 'button' }, icon('select', 14), 'Seçili noktaları ekle');
    fromSelection.addEventListener('click', () => {
      const picked = [...this.ctx.selection.ids.value]
        .map((id) => this.ctx.doc.get(id))
        .filter((e) => e?.kind === 'point')
        .map((e) => (e?.kind === 'point' ? (e.label ?? `${e.p.x},${e.p.y}`) : ''));
      if (!picked.length) return this.ctx.log.warn('Seçili nokta yok: aplike edilecek noktaları seçip pencereyi yeniden açın.');
      const kept = state.rows.filter((r) => r.point?.trim());
      state.rows.splice(0, state.rows.length, ...kept, ...picked.map((point) => ({ point })));
      this.render();
    });
    replaceChildren(this.table, h('div', { class: 'calc-table-head' }, h('p', { class: 'io-field__hint' }, 'Noktaları adlarıyla ya da Y,X olarak yazın; çizimde seçili noktaları da ekleyebilirsiniz.'), fromSelection), this.grid.el);
    this.recompute();
  }

  private recompute(): void {
    const { ctx } = this;
    this.result = null;
    const errors: string[] = [];
    const st = resolvePoint(ctx, state.station.text);
    const bk = resolvePoint(ctx, state.back.text);
    if (!st) errors.push('Durulan nokta verilmedi.');
    else if ('error' in st) errors.push(`Durulan nokta: ${st.error}`);
    if (bk && 'error' in bk) errors.push(`Bakılan nokta: ${bk.error}`);
    const targets: Vec2[] = [];
    const names: string[] = [];
    state.rows.forEach((r, i) => {
      const t = resolvePoint(ctx, r.point ?? '');
      if (!t) return;
      if ('error' in t) errors.push(`${i + 1}. satır: ${t.error}`);
      else {
        targets.push(t.p);
        names.push(t.name || (r.point ?? '').trim());
      }
    });
    if (!targets.length && !errors.length) errors.push('Tabloya aplike edilecek en az bir nokta yazın.');
    if (!errors.length && st && 'p' in st) {
      try {
        const stakes = surveyStakeout({ unit: ctx.doc.settings.angleUnit.value, station: st.p, back: bk && 'p' in bk ? bk.p : null, targets });
        this.result = { stakes, names };
      } catch (e) {
        errors.push((e as Error).message);
      }
    }
    const res = this.result;
    this.copy.disabled = !res;
    if (!res) {
      summary(this.summaryBox, errors.slice(0, 6).map((e) => summaryLine('warn', e)));
      replaceChildren(this.results);
      return;
    }
    const f = ctx.format;
    summary(this.summaryBox, [
      summaryLine('ok', `${res.stakes.length} nokta için semt ve uzunluk hesaplandı.`),
      bk ? null : summaryLine('info', 'Bakılan nokta verilmedi: dönülecek açılar yok, aleti semte göre yöneltin.'),
    ]);
    const rows = res.stakes.map((s, i) => [res.names[i], angleText(ctx, s.bearing), f.length(s.distance, false), s.angle != null ? angleText(ctx, s.angle) : '—']);
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Aplikasyon değerleri'),
      resultTable(['Nokta', 'Semt', 'Yatay uzunluk (m)', 'Bakılan noktadan açı'], rows, [false, true, true, true]),
    );
  }

  private copyReport(): void {
    const res = this.result;
    if (!res) return;
    const f = this.ctx.format;
    const lines: string[][] = [[TITLE], ['Nokta', 'Semt', 'Yatay uzunluk', 'Açı']];
    res.stakes.forEach((s, i) => lines.push([res.names[i], s.bearing.toFixed(4), f.length(s.distance, false), s.angle != null ? s.angle.toFixed(4) : '']));
    copyReport(this.ctx, TITLE, lines);
  }
}
