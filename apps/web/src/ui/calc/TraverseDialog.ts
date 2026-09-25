import type { AppContext } from '../../app/context';
import { surveyTraverse, type TraverseResult } from '../../model/geom/surveyCalc';
import type { Vec2 } from '../../model/geometry';
import { h, replaceChildren } from '../dom';
import { segmented, toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import {
  addPoints,
  angleText,
  copyReport,
  field,
  Grid,
  knownField,
  layerChoice,
  mmText,
  readNumber,
  resolvePoint,
  resultTable,
  smallAngleText,
  summary,
  summaryLine,
  type GridModel,
  type NewPoint,
  type Picker,
  type Row,
} from './common';

/**
 * Poligon hesabı: from a known point oriented on a known back point, through
 * the measured angles (kırılma açısı: at each station clockwise from the
 * previous point to the next) and horizontal leg lengths, to the new points.
 * Bağlı: closed on a known end point, oriented on a known fore point if the
 * angle there was measured; kapalı: back on the start; açık: no closure.
 * The misclosures are shown and taken off (angles equally, coordinates by
 * the compass rule); no tolerance is applied: the surveyor judges them.
 */
export function openTraverse(ctx: AppContext): void {
  new TraverseDialog(ctx);
}

type Kind = 'connected' | 'closed' | 'open';

const state = {
  kind: 'connected' as Kind,
  endOriented: true,
  start: { text: '' },
  back: { text: '' },
  end: { text: '' },
  fore: { text: '' },
  first: { name: '', angle: '', distance: '' } as Row,
  rows: [{}, {}] as Row[],
  last: { name: '', angle: '', distance: '' } as Row,
  layer: null as string | null,
};

const TITLE = 'Poligon hesabı';

class TraverseDialog implements Picker {
  readonly ctx: AppContext;
  readonly title = TITLE;
  private readonly form = h('div', { class: 'calc-form' });
  private readonly table = h('div', { class: 'calc-section' });
  private readonly results = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly add = h('button', { class: 'btn btn--primary', type: 'button' }, 'Çizime ekle');
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly dialog: Dialog;
  private grid: Grid | null = null;
  private result: { r: TraverseResult; names: string[]; endName: string | null; start: Vec2 } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    const layer = layerChoice(ctx, state, 'poligon');
    this.dialog = new Dialog({
      title: TITLE,
      width: 940,
      className: 'dialog--io dialog--calc',
      content: [this.form, this.table, this.summaryBox, this.results],
      footer: [this.status, this.copy, h('span', { class: 'calc-foot-label' }, 'Katman'), layer, this.add, close],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.add.addEventListener('click', () => this.addToDrawing());
    this.copy.addEventListener('click', () => this.copyReport());
    this.render();
  }

  close(): void {
    this.dialog.close();
  }

  reopen(): void {
    openTraverse(this.ctx);
  }

  private render(): void {
    const { ctx } = this;
    const recompute = () => this.recompute();
    // The start and end name the table's first and last rows.
    const renamed = () => (this.grid?.render(), this.recompute());
    const kind = segmented<Kind>({
      label: 'Poligon türü',
      options: [
        { value: 'connected', label: 'Bağlı', hint: 'Bilinen bir noktadan başlar, bilinen başka bir noktada biter.' },
        { value: 'closed', label: 'Kapalı', hint: 'Başladığı noktaya döner.' },
        { value: 'open', label: 'Açık', hint: 'Bilinen bir noktada bitmez: kapanma denetimi ve dengeleme yok.' },
      ],
      value: state.kind,
      onChange: (k) => ((state.kind = k), this.render()),
    });
    const oriented = toggleSwitch({ label: 'Bitişte yöneltme açısı ölçüldü', checked: state.endOriented, onChange: (v) => ((state.endOriented = v), this.render()) });
    replaceChildren(
      this.form,
      h('div', { class: 'io-row' }, field('Poligon türü', kind), state.kind === 'connected' ? field('Bitiş', h('div', { class: 'io-check' }, oriented, 'Bitişte yöneltme açısı ölçüldü')) : null),
      h(
        'div',
        { class: 'calc-knowns' },
        knownField(this, 'Başlangıç noktası (A)', state.start, renamed, 'start'),
        knownField(this, 'Başlangıçta bakılan nokta', state.back, recompute, 'back', 'İlk açı bu noktadan ölçülür'),
        state.kind === 'connected' ? knownField(this, 'Bitiş noktası (B)', state.end, renamed, 'end') : null,
        state.kind === 'connected' && state.endOriented ? knownField(this, 'Bitişte bakılan nokta', state.fore, recompute, 'fore', 'Son açı bu noktaya ölçülür') : null,
      ),
    );
    const unit = ctx.format.angleUnitLabel === '°' ? '°' : 'g';
    const model: GridModel = {
      columns: [
        { key: 'name', label: 'Nokta', placeholder: (r) => `P${r}` },
        { key: 'angle', label: 'Kırılma açısı', unit, numeric: true },
        { key: 'distance', label: 'Sonraki noktaya kenar', unit: 'm', numeric: true },
      ],
      rows: () => this.viewRows(),
      readonly: (r, key) => {
        const rows = this.viewRows();
        const endRow = this.hasEndRow() && r === rows.length - 1;
        if (key === 'name') return r === 0 || endRow;
        if (endRow) return key === 'distance' || !this.endAngle();
        // The last new point of an open traverse has neither an angle nor a leg after it.
        return state.kind === 'open' && r === rows.length - 1 && r > 0;
      },
      canInsertAfter: (r) => !(this.hasEndRow() && r === this.viewRows().length - 1),
      insertAfter: (r) => state.rows.splice(r, 0, {}),
      canRemove: (r) => r > 0 && r <= state.rows.length,
      remove: (r) => state.rows.splice(r - 1, 1),
    };
    this.grid = new Grid(model, recompute);
    replaceChildren(
      this.table,
      h('p', { class: 'io-field__hint' }, 'Kırılma açısı her istasyonda önceki noktadan sonraki noktaya saat yönünde ölçülür (başlangıçta bakılan noktadan). Kenarlar yataydır. Elektronik tablodan satırları yapıştırabilirsiniz.'),
      this.grid.el,
    );
    this.recompute();
  }

  private hasEndRow(): boolean {
    return state.kind !== 'open';
  }

  /** Whether the end station has an angle (to the fore point, or to the back point on a closed traverse). */
  private endAngle(): boolean {
    return state.kind === 'closed' || (state.kind === 'connected' && state.endOriented);
  }

  /** Start, the new points and the end station, as the table shows them. */
  private viewRows(): Row[] {
    const startName = this.nameOf(state.start.text, 'A');
    state.first.name = startName;
    const rows = [state.first, ...state.rows];
    if (!this.hasEndRow()) return rows;
    state.last.name = state.kind === 'closed' ? startName : this.nameOf(state.end.text, 'B');
    state.last.distance = '';
    return [...rows, state.last];
  }

  private nameOf(text: string, fallback: string): string {
    const r = resolvePoint(this.ctx, text);
    return r && 'p' in r && r.name ? r.name : fallback;
  }

  private recompute(): void {
    const { ctx } = this;
    this.result = null;
    const errors: string[] = [];
    const known = (text: string, label: string): Vec2 | null => {
      const r = resolvePoint(ctx, text);
      if (!r) errors.push(`${label} verilmedi.`);
      else if ('error' in r) errors.push(`${label}: ${r.error}`);
      else return r.p;
      return null;
    };
    const start = known(state.start.text, 'Başlangıç noktası');
    const back = known(state.back.text, 'Başlangıçta bakılan nokta');
    let end: Vec2 | null = null;
    let fore: Vec2 | null = null;
    if (state.kind === 'connected') {
      end = known(state.end.text, 'Bitiş noktası');
      if (state.endOriented) fore = known(state.fore.text, 'Bitişte bakılan nokta');
    } else if (state.kind === 'closed') {
      end = start;
      fore = back;
    }
    // New points with nothing typed at the end of the table are left out.
    const rows = [...state.rows];
    while (rows.length && !Object.values(rows[rows.length - 1]).some((v) => v?.trim())) rows.pop();
    const stations = [state.first, ...(state.kind === 'open' ? rows.slice(0, -1) : rows)];
    const angles: number[] = [];
    const distances: number[] = [];
    stations.forEach((row, i) => {
      const a = readNumber(row.angle);
      const d = readNumber(row.distance);
      const where = i === 0 ? 'Başlangıç satırında' : `${i + 1}. satırda`;
      if (a === null) errors.push(`${where} kırılma açısı yok.`);
      else if (Number.isNaN(a)) errors.push(`${where} kırılma açısı bir sayı değil.`);
      if (d === null) errors.push(`${where} kenar uzunluğu yok.`);
      else if (Number.isNaN(d)) errors.push(`${where} kenar uzunluğu bir sayı değil.`);
      angles.push(a ?? Number.NaN);
      distances.push(d ?? Number.NaN);
    });
    if (fore) {
      const a = readNumber(state.last.angle);
      if (a === null || Number.isNaN(a)) errors.push(state.kind === 'closed' ? 'Son satırda (başlangıca dönüşte) kırılma açısı yok.' : 'Bitiş satırında kırılma açısı yok.');
      angles.push(a ?? Number.NaN);
    }
    if (state.kind === 'open' && !rows.length) errors.push('Açık poligonda en az bir yeni nokta olmalı.');
    const names = rows.map((r, i) => r.name?.trim() || `P${i + 1}`);
    if (!errors.length && start && back) {
      try {
        const r = surveyTraverse({ unit: ctx.doc.settings.angleUnit.value, start, back, end, fore, angles, distances });
        const endName = state.kind === 'closed' ? this.nameOf(state.start.text, 'A') : this.nameOf(state.end.text, 'B');
        this.result = { r, names, endName: this.hasEndRow() ? endName : null, start };
      } catch (e) {
        errors.push((e as Error).message);
      }
    }
    this.show(errors);
  }

  private show(errors: string[]): void {
    const { ctx } = this;
    const f = ctx.format;
    const res = this.result;
    this.add.disabled = !res || !res.r.points.length;
    this.copy.disabled = !res;
    this.status.textContent = '';
    if (!res) {
      summary(this.summaryBox, errors.slice(0, 6).map((e) => summaryLine('warn', e)));
      replaceChildren(this.results);
      return;
    }
    const { r } = res;
    const ratio = r.linearMisclosure ? Math.round(r.length / r.linearMisclosure) : null;
    summary(this.summaryBox, [
      r.angleMisclosure != null
        ? summaryLine('info', `Açı kapanma hatası fβ = ${smallAngleText(ctx, r.angleMisclosure)}; her açıya ${smallAngleText(ctx, r.angleCorrection ?? 0)} düzeltme verildi.`)
        : state.kind === 'connected'
          ? summaryLine('info', 'Bitişte yöneltme yok: açı kapanması denetlenmedi.')
          : null,
      r.linearMisclosure != null
        ? summaryLine(
            'info',
            `Koordinat kapanma hatası fy = ${mmText(r.fy ?? 0)}, fx = ${mmText(r.fx ?? 0)}, fs = ${mmText(r.linearMisclosure)}; kenarlara uzunluklarıyla orantılı dağıtıldı (toplam ${f.length(r.length)}${ratio ? `, 1/${ratio}` : ''}).`,
          )
        : summaryLine('info', `Açık poligon: kapanma denetimi ve dengeleme yok (toplam ${f.length(r.length)}).`),
      summaryLine('info', 'Hata sınırı uygulanmaz: kapanma hatalarını ölçü sınıfınızın sınırlarıyla karşılaştırın.'),
    ]);
    const endPts = [...r.points, ...(res.endName !== null ? [null] : [])];
    const rows = r.legs.map((leg, i) => {
      const name = i < res.names.length ? res.names[i] : (res.endName ?? '');
      const p = endPts[i];
      return [name, angleText(ctx, leg.bearing), f.length(leg.distance, false), f.length(leg.dy, false), f.length(leg.dx, false), p ? f.coord(p.x) : 'bilinen', p ? f.coord(p.y) : 'bilinen'];
    });
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Sonuç'),
      resultTable(['Nokta', 'Semt', 'Kenar (m)', 'ΔY (m)', 'ΔX (m)', 'Y (sağa)', 'X (yukarı)'], rows, [false, true, true, true, true, true, true]),
    );
  }

  private points(): NewPoint[] {
    const res = this.result;
    if (!res) return [];
    return res.r.points.map((p, i) => ({ name: res.names[i], p }));
  }

  private addToDrawing(): void {
    const pts = this.points();
    if (!pts.length || !state.layer) return;
    const n = addPoints(this.ctx, state.layer, pts, 'Poligon noktası', TITLE);
    if (n === null) return;
    this.ctx.log.success(`${TITLE}: ${n} poligon noktası çizime eklendi (Ctrl+Z geri alır).`);
    this.dialog.close();
  }

  private copyReport(): void {
    const res = this.result;
    if (!res) return;
    const { ctx } = this;
    const f = ctx.format;
    const { r } = res;
    const lines: string[][] = [[TITLE], ['Nokta', 'Semt', 'Kenar', 'ΔY', 'ΔX', 'vY', 'vX', 'Y', 'X']];
    const endPts = [...r.points, ...(res.endName !== null ? [null] : [])];
    r.legs.forEach((leg, i) => {
      const p = endPts[i];
      lines.push([
        i < res.names.length ? res.names[i] : (res.endName ?? ''),
        leg.bearing.toFixed(4),
        f.length(leg.distance, false),
        f.length(leg.dy, false),
        f.length(leg.dx, false),
        leg.vy.toFixed(4),
        leg.vx.toFixed(4),
        p ? f.coord(p.x) : '',
        p ? f.coord(p.y) : '',
      ]);
    });
    if (r.angleMisclosure != null) lines.push(['Açı kapanma hatası', smallAngleText(ctx, r.angleMisclosure), 'Düzeltme', smallAngleText(ctx, r.angleCorrection ?? 0)]);
    if (r.linearMisclosure != null) lines.push(['fy', mmText(r.fy ?? 0), 'fx', mmText(r.fx ?? 0), 'fs', mmText(r.linearMisclosure), 'Toplam', f.length(r.length)]);
    copyReport(ctx, TITLE, lines);
  }
}
