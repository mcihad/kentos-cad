import type { AppContext } from '../../app/context';
import { surveyPolar, type PolarPoint } from '../../model/geom/surveyCalc';
import type { Vec2 } from '../../model/geometry';
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

/**
 * Kutupsal alım (takeometri): points surveyed from a known station. The
 * instrument is oriented on a known back point by its reading to it; each
 * point has a horizontal direction reading and a distance, horizontal or
 * slope with its zenith angle (then a height too, from the station's height,
 * the instrument's and the target's). Curvature and refraction are not
 * applied.
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
      h('p', { class: 'io-field__hint' }, 'Okumalar saat yönündedir; bakılan noktanın okuması semtine eşlenir. Başucu açısı verilen uzunluk eğiktir (0 tam yukarı, çeyrek tur yatay); o zaman nokta kotu da hesaplanır. Yer eğriliği ve kırılma uygulanmaz.'),
      new Grid(model, recompute).el,
    );
    this.recompute();
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
    const station = known(state.station.text, 'Durulan nokta');
    const back = known(state.back.text, 'Bakılan nokta');
    const backReading = readNumber(state.backReading) ?? 0;
    const stationZ = readNumber(state.stationZ);
    const ih = readNumber(state.instrumentHeight);
    if (Number.isNaN(backReading)) errors.push('Bakılan noktanın okuması bir sayı değil.');
    const rows = state.rows.map((r, i) => ({ r, i })).filter(({ r }) => Object.values(r).some((v) => v?.trim()));
    if (!rows.length) errors.push('Tabloya en az bir nokta yazın.');
    const shots = rows.map(({ r, i }) => {
      const reading = readNumber(r.reading);
      const distance = readNumber(r.distance);
      const zenith = readNumber(r.zenith);
      const target = readNumber(r.target);
      if (reading === null) errors.push(`${i + 1}. satırda yatay açı okuması yok.`);
      else if (Number.isNaN(reading)) errors.push(`${i + 1}. satırda yatay açı okuması bir sayı değil.`);
      if (distance === null) errors.push(`${i + 1}. satırda uzunluk yok.`);
      else if (Number.isNaN(distance)) errors.push(`${i + 1}. satırda uzunluk bir sayı değil.`);
      if (Number.isNaN(zenith ?? 0) || Number.isNaN(target ?? 0)) errors.push(`${i + 1}. satırda bir değer sayı değil.`);
      return { reading: reading ?? Number.NaN, distance: distance ?? Number.NaN, zenith, targetHeight: target };
    });
    const names = rows.map(({ r, i }) => r.name?.trim() || `${i + 1}`);
    if (!errors.length && station && back) {
      try {
        const points = surveyPolar({
          unit: ctx.doc.settings.angleUnit.value,
          station,
          back,
          backReading,
          stationZ: stationZ !== null && !Number.isNaN(stationZ) ? stationZ : null,
          instrumentHeight: ih !== null && !Number.isNaN(ih) ? ih : null,
          shots,
        });
        this.result = { points, names };
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
    this.add.disabled = !res;
    this.copy.disabled = !res;
    if (!res) {
      summary(this.summaryBox, errors.slice(0, 6).map((e) => summaryLine('warn', e)));
      replaceChildren(this.results);
      return;
    }
    const withZ = res.points.filter((p) => p.z != null).length;
    summary(this.summaryBox, [
      summaryLine('ok', `${res.points.length} nokta hesaplandı${withZ ? `, ${withZ} noktanın kotu ile` : ''}.`),
      res.points.some((p) => p.dz != null) && readNumber(state.stationZ) === null ? summaryLine('info', 'İstasyon kotu verilmedi: yükseklik farkları hesaplandı, kotlar yazılmadı.') : null,
    ]);
    const rows = res.points.map((p, i) => [res.names[i], angleText(ctx, p.bearing), f.length(p.horizontal, false), f.coord(p.p.x), f.coord(p.p.y), p.z != null ? f.length(p.z, false) : p.dz != null ? `Δ ${f.length(p.dz, false)}` : '—']);
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Sonuç'),
      resultTable(['Nokta', 'Semt', 'Yatay uzunluk (m)', 'Y (sağa)', 'X (yukarı)', 'Z (m)'], rows, [false, true, true, true, true, true]),
    );
  }

  private points(): NewPoint[] {
    const res = this.result;
    return res ? res.points.map((p, i) => ({ name: res.names[i], p: p.p, z: p.z })) : [];
  }

  private addToDrawing(): void {
    const pts = this.points();
    if (!pts.length || !state.layer) return;
    const n = addPoints(this.ctx, state.layer, pts, 'Alım noktası', TITLE);
    if (n === null) return;
    this.ctx.log.success(`${TITLE}: ${n} nokta çizime eklendi (Ctrl+Z geri alır).`);
    this.dialog.close();
  }

  private copyReport(): void {
    const res = this.result;
    if (!res) return;
    const f = this.ctx.format;
    const lines: string[][] = [[TITLE], ['Nokta', 'Semt', 'Yatay uzunluk', 'Y', 'X', 'Z']];
    res.points.forEach((p, i) => lines.push([res.names[i], p.bearing.toFixed(4), f.length(p.horizontal, false), f.coord(p.p.x), f.coord(p.p.y), p.z != null ? f.length(p.z, false) : '']));
    copyReport(this.ctx, TITLE, lines);
  }
}
