import type { AppContext } from '../../app/context';
import type { PolarPoint } from '../../model/geom/surveyCalc';
import { h, replaceChildren } from '../dom';
import { textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import {
  addPoints,
  angleText,
  copyReport,
  field,
  Grid,
  knownField,
  layerChoice,
  readNumber,
  resolvePoint,
  resultTable,
  summary,
  summaryLine,
  type GridModel,
  type NewPoint,
  type Picker,
  type Row,
} from './common';
import { readPolar } from './read';
import { fixed } from '../../core/displayNumber';
import { gridNote, surveyGrid } from '../../model/groundMeasures';

/**
 * Kutupsal alım (takeometri): points surveyed from a known station. The
 * instrument is oriented on a known back point by its reading to it; each
 * point has a horizontal direction reading and a distance, horizontal or
 * slope with its zenith angle (then a height too, from the station's height,
 * the instrument's and the target's, with the earth's curvature and
 * refraction by the project's k, docs/adr/0169 §3).
 */
export function openPolar(ctx: AppContext): void {
  new PolarDialog(ctx);
}

const state = {
  station: { text: '' },
  back: { text: '' },
  backReading: '0',
  stationZ: '',
  instrumentHeight: '',
  rows: [{}, {}, {}] as Row[],
  layer: null as string | null,
};

const TITLE = 'Kutupsal alım';

/** Kutupsal alım's fields as another window fills them (Karne editörü, docs/adr/0169 §3): the table's rows replaced. */
export interface PolarFill {
  station: string;
  back: string;
  backReading: string;
  stationZ: string;
  instrumentHeight: string;
  rows: Row[];
}

/** Fills Kutupsal alım's fields, then opens it. */
export function openPolarWith(ctx: AppContext, fill: PolarFill): void {
  state.station.text = fill.station;
  state.back.text = fill.back;
  state.backReading = fill.backReading;
  state.stationZ = fill.stationZ;
  state.instrumentHeight = fill.instrumentHeight;
  state.rows = fill.rows.length ? fill.rows : [{}, {}, {}];
  openPolar(ctx);
}

class PolarDialog implements Picker {
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
  private result: { points: PolarPoint[]; names: string[] } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    const layer = layerChoice(ctx, state, ctx.doc.layers.active.value);
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
    openPolar(this.ctx);
  }

  private render(): void {
    const recompute = () => this.recompute();
    const unit = this.ctx.format.angleUnitLabel === '°' ? '°' : 'g';
    const num = (label: string, key: 'backReading' | 'stationZ' | 'instrumentHeight', placeholder = '') => {
      const f = textField({ label, value: state[key], placeholder, onChange: (v) => ((state[key] = v), recompute()) });
      f.classList.add('calc-num');
      return field(label, f);
    };
    replaceChildren(
      this.form,
      h(
        'div',
        { class: 'calc-knowns' },
        knownField(this, 'Durulan nokta (istasyon)', state.station, recompute, 'station'),
        knownField(this, 'Bakılan nokta', state.back, recompute, 'back', 'Alet bu noktaya yöneltilir'),
      ),
      h('div', { class: 'io-row' }, num(`Bakılan noktanın okuması (${unit})`, 'backReading'), num('İstasyon kotu (m)', 'stationZ', 'kot yoksa boş'), num('Alet yüksekliği (m)', 'instrumentHeight', '0')),
    );
    const model: GridModel = {
      columns: [
        { key: 'name', label: 'Nokta', placeholder: (r) => `${r + 1}` },
        { key: 'reading', label: 'Yatay açı okuması', unit, numeric: true },
        { key: 'distance', label: 'Uzunluk', unit: 'm', numeric: true },
        { key: 'zenith', label: 'Başucu açısı', unit, numeric: true, placeholder: () => 'yatay uzunluksa boş' },
        { key: 'target', label: 'Reflektör yüksekliği', unit: 'm', numeric: true },
      ],
      rows: () => state.rows,
      readonly: () => false,
      canInsertAfter: () => true,
      insertAfter: (r) => state.rows.splice(r + 1, 0, {}),
      canRemove: () => state.rows.length > 1,
      remove: (r) => state.rows.splice(r, 1),
    };
    replaceChildren(
      this.table,
      h(
        'p',
        { class: 'io-field__hint' },
        `Okumalar saat yönündedir; bakılan noktanın okuması semtine eşlenir. Başucu açısı verilen uzunluk eğiktir (0 tam yukarı, çeyrek tur yatay); o zaman nokta kotu da hesaplanır, yer eğriliği ve refraksiyonla: (1 − k)·D²/2R, k = ${this.ctx.doc.settings.refraction} (Proje ayarları › Ölçme).`,
      ),
      new Grid(model, recompute).el,
    );
    this.recompute();
  }

  /** The fields read and computed (read.ts); the result, or the errors shown. */
  private recompute(): void {
    const { ctx } = this;
    const form = { station: state.station.text, back: state.back.text, backReading: state.backReading, stationZ: state.stationZ, instrumentHeight: state.instrumentHeight, rows: state.rows };
    const read = readPolar(form, (text) => resolvePoint(ctx, text), ctx.doc.settings.angleUnit.value, ctx.doc.settings.refraction, surveyGrid(ctx.doc.settings));
    this.result = read.points ? { points: read.points, names: read.names } : null;
    this.show(read.errors);
  }

  private show(errors: string[]): void {
    const { ctx } = this;
    const f = ctx.format;
    const res = this.result;
    this.add.disabled = !res;
    this.copy.disabled = !res;
    if (!res) {
      summary(this.summaryBox, errors.slice(0, 6).map((e) => summaryLine('warn', e)));
      replaceChildren(this.results);
      return;
    }
    const withZ = res.points.filter((p) => p.z != null).length;
    // With the project's grid the length on it and the line's factor come too (docs/adr/0171 §4).
    const reduced = res.points.some((p) => p.grid !== undefined);
    const height = ctx.doc.settings.groundHeight;
    summary(this.summaryBox, [
      summaryLine('ok', `${res.points.length} nokta hesaplandı${withZ ? `, ${withZ} noktanın kotu ile` : ''}.`),
      reduced && height !== null ? summaryLine('info', gridNote(height)) : null,
      res.points.some((p) => p.dz != null) && readNumber(state.stationZ) === null ? summaryLine('info', 'İstasyon kotu verilmedi: yükseklik farkları hesaplandı, kotlar yazılmadı.') : null,
    ]);
    const rows = res.points.map((p, i) => [
      res.names[i],
      angleText(ctx, p.bearing),
      f.length(p.horizontal, false),
      ...(reduced ? [p.grid !== undefined ? f.length(p.grid, false) : '', p.scale !== undefined && p.heightFactor !== undefined ? fixed(p.scale * p.heightFactor, 8) : ''] : []),
      f.coord(p.p.x),
      f.coord(p.p.y),
      p.z != null ? f.length(p.z, false) : p.dz != null ? `Δ ${f.length(p.dz, false)}` : '—',
    ]);
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Sonuç'),
      reduced
        ? resultTable(['Nokta', 'Semt', 'Zeminde (m)', 'Düzlemde (m)', 'Çarpan', 'Y (sağa)', 'X (yukarı)', 'Z (m)'], rows, [false, true, true, true, true, true, true, true])
        : resultTable(['Nokta', 'Semt', 'Yatay uzunluk (m)', 'Y (sağa)', 'X (yukarı)', 'Z (m)'], rows, [false, true, true, true, true, true]),
    );
  }

  private points(): NewPoint[] {
    const res = this.result;
    return res ? res.points.map((p, i) => ({ name: res.names[i], p: p.p, z: p.z })) : [];
  }

  private addToDrawing(): void {
    const pts = this.points();
    if (!pts.length || !state.layer) return;
    const n = addPoints(this.ctx, state.layer, pts, 'Alım noktası', 'polarSurvey');
    if (n === null) return;
    this.ctx.log.success(`${TITLE}: ${n} nokta çizime eklendi (Ctrl+Z geri alır).`);
    this.dialog.close();
  }

  private copyReport(): void {
    const res = this.result;
    if (!res) return;
    const f = this.ctx.format;
    // With the project's grid: the length on it and the line's factors (docs/adr/0171 §4).
    const reduced = res.points.some((p) => p.grid !== undefined);
    const lines: string[][] = [
      [TITLE],
      ['Nokta', 'Semt', 'Yatay uzunluk', ...(reduced ? ['Düzlemde'] : []), 'Y', 'X', 'Z', ...(reduced ? ['Ölçek', 'Yükseklik çarpanı'] : [])],
    ];
    res.points.forEach((p, i) =>
      lines.push([
        res.names[i],
        fixed(p.bearing, 4),
        f.length(p.horizontal, false),
        ...(reduced ? [p.grid !== undefined ? f.length(p.grid, false) : ''] : []),
        f.coord(p.p.x),
        f.coord(p.p.y),
        p.z != null ? f.length(p.z, false) : '',
        ...(reduced ? [p.scale !== undefined ? fixed(p.scale, 8) : '', p.heightFactor !== undefined ? fixed(p.heightFactor, 8) : ''] : []),
      ]),
    );
    copyReport(this.ctx, TITLE, lines);
  }
}
