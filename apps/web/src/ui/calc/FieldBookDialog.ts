import type { AppContext } from '../../app/context';
import type { FieldBookRead } from '../../contracts/generated/FieldBookRead';
import type { FieldCsvOptions } from '../../contracts/generated/FieldCsvOptions';
import type { FieldStation } from '../../contracts/generated/FieldStation';
import { fixed } from '../../core/displayNumber';
import { formats } from '../../io/client';
import { fieldPolar, fieldReduce, fieldTraverse, type FieldStation as CoreStation, type Reduction, type Tolerances, type TraverseTransfer } from '../../model/geom/surveyCalc';
import type { AngleUnit } from '../../model/projectSettings';
import { surveyTexts } from '../../model/surveyForm';
import { h, replaceChildren } from '../dom';
import { checkField, field, fileLine, select } from '../io/common';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { copyReport, Grid, readNumber, resolvePoint, summary, summaryLine, type GridModel, type Row } from './common';
import { openPolarWith } from './PolarDialog';
import { openTraverseWith } from './TraverseDialog';

/**
 * Karne editörü (docs/adr/0169 §2–§3, §6; the desktop's `calc/fieldbook/`): a field book opened from an instrument's
 * file (Leica GSI, told by its content) or a text book whose columns are mapped here; its stations, their observations as
 * the file has them, which may be left out (Kullan) and renamed; the station shown reduced as the shared core reduces it
 * (`fieldReduce`) with the project's k and tolerances: the faces paired and their differences, the horizontal distances
 * and the height differences, a difference above its tolerance in the warning colour. What is opened stays while the app
 * runs, as the other Hesap windows' fields do; the file itself is not changed.
 */
export function openFieldBook(ctx: AppContext, file?: File): void {
  const dialog = new FieldBookDialog(ctx);
  if (file) void dialog.open(file);
}

const TITLE = 'Karne editörü';
/** The largest field book read (an instrument's files are kilobytes). */
const LIMIT = 64 << 20;
/** A text book's columns as the mapping names them, in the CSV options' order. */
const MAPPED = ['İstasyon', 'Alet yüksekliği', 'Nokta', 'Yatay açı', 'Başucu açısı', 'Eğik uzunluk', 'Prizma yüksekliği', 'Kod'] as const;
const KEYS = ['station', 'instrumentHeight', 'target', 'hz', 'zenith', 'slope', 'targetHeight', 'code'] as const;
const TARGET = 2;
const HZ = 3;

/** A station's edits: the observations used, their names, its instrument height as typed. */
interface Edits {
  rows: Row[];
  height: string;
}

/** What is opened, kept while the app runs. */
const state = {
  file: null as string | null,
  bytes: null as Uint8Array | null,
  book: null as FieldBookRead | null,
  mapping: { columns: Array<number | null>(8).fill(null), header: true, unit: null as AngleUnit | null },
  station: 0,
  /** Each station's reduced row Kutupsal alım is oriented on (the first station's is the traverse's back sight too). */
  backs: [] as number[],
  /** The last station's reduced row the traverse ends oriented on. */
  fore: null as number | null,
  edits: [] as Edits[],
  error: null as string | null,
};

const editsOf = (s: FieldStation): Edits => ({
  rows: s.observations.map((o) => ({ use: '1', name: o.target })),
  height: s.instrumentHeight === undefined ? '' : String(s.instrumentHeight),
});

/** A station in the core's terms: the observations used, as named here, and where each came from in the file's list. */
function coreStation(at: number): { station: CoreStation; from: number[] } | null {
  const st = state.book?.stations[at];
  const edits = state.edits[at];
  if (!st || !edits) return null;
  const from: number[] = [];
  const observations = st.observations.flatMap((o, i) => {
    if (edits.rows[i]?.use === '0') return [];
    from.push(i);
    return [{ ...o, target: edits.rows[i]?.name ?? o.target }];
  });
  const ih = readNumber(edits.height);
  return { station: { station: st.station, ...(ih !== null && Number.isFinite(ih) ? { instrumentHeight: ih } : {}), observations }, from };
}

/** A text book's reader options; null until the point and the horizontal reading are mapped. */
function options(): FieldCsvOptions | null {
  const c = state.mapping.columns;
  if (c[TARGET] === null || c[HZ] === null) return null;
  const opt: FieldCsvOptions = { target: c[TARGET]!, hz: c[HZ]!, header: state.mapping.header };
  KEYS.forEach((k, i) => {
    if (i !== TARGET && i !== HZ && c[i] !== null) (opt as Record<string, unknown>)[k] = c[i];
  });
  return opt;
}

/** An angle's columns in gon or degrees, the differences in cc or seconds. */
const marks = (unit: AngleUnit) => (unit === 'grad' ? { mark: 'g', fine: 'cc', per: 10000 } : { mark: '°', fine: '″', per: 3600 });
const shown = (v: number | null | undefined, d: number): string => (v === null || v === undefined ? '' : fixed(v, d));
/**
 * A value Kutupsal alım is filled with: the display rule's `d` decimals (8 for angles, 6 for lengths: far below what an
 * instrument resolves), trailing zeros and a bare point dropped (the desktop's `fieldbook::exact`).
 */
function exact(v: number, d: number): string {
  let s = fixed(v, d);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  return s === '' || s === '-0' ? '0' : s;
}

class FieldBookDialog {
  private readonly ctx: AppContext;
  private readonly fileBox = h('div', { class: 'fieldbook-file' });
  private readonly mappingBox = h('div', { class: 'fieldbook-mapping' });
  private readonly stationBox = h('div', { class: 'fieldbook-station' });
  private readonly tableBox = h('div', { class: 'calc-section' });
  private readonly reducedBox = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly transfer = h('button', { class: 'btn btn--primary', type: 'button' }, "Kutupsal alım'a aktar");
  private readonly toTraverseButton = h('button', { class: 'btn', type: 'button' }, "Poligon hesabı'na aktar");
  /** Poligon: the stations in order, the fore sight, the legs (two stations or more). */
  private readonly traverseBox = h('div', { class: 'calc-section' });
  private traverse: TraverseTransfer | null = null;
  /** The last station's reduced rows (Bitişte bakılan), and the traverse's back and fore sights as last synced. */
  private lastRows: string[] = [];
  private ends: { back: string; fore: string | null } | null = null;
  /** Geri bakış: the reduced row Kutupsal alım is oriented on (filled when the station is reduced). */
  private readonly backBox = h('div', { class: 'fieldbook-back' });
  /** The station shown in the core's terms, as last reduced (Kutupsal alım'a aktar reduces it again). */
  private core: { station: CoreStation; tolerances: Tolerances | null } | null = null;
  private readonly dialog: Dialog;
  private reduction: Reduction | null = null;
  /** The observations the reduction was made of (the table's rows used, in order). */
  private from: number[] = [];

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: TITLE,
      width: 1040,
      className: 'dialog--io dialog--calc dialog--fieldbook',
      content: [this.fileBox, this.mappingBox, this.stationBox, this.tableBox, this.reducedBox, this.traverseBox, this.summaryBox],
      footer: [h('div', { class: 'dialog__spacer' }), this.copy, this.toTraverseButton, this.transfer, close],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.copy.addEventListener('click', () => this.copyReport());
    this.transfer.addEventListener('click', () => this.toPolar());
    this.toTraverseButton.addEventListener('click', () => this.toTraverse());
    this.render();
  }

  /** The book's angle unit: the file's, else the mapping's, else the project's. */
  private unit(): AngleUnit {
    const u = state.book?.unit;
    return u === 'grad' || u === 'deg' ? u : (state.mapping.unit ?? this.ctx.doc.settings.angleUnit.value);
  }

  private render(): void {
    this.renderFile();
    this.renderMapping();
    this.renderStation();
    this.sync();
  }

  private renderFile(): void {
    const input = h('input', { type: 'file', accept: '.gsi,.GSI,.txt,.TXT,.csv,.CSV,.dat,.DAT', hidden: true }) as HTMLInputElement;
    input.addEventListener('change', () => {
      const f = input.files?.[0];
      if (f) void this.open(f);
    });
    const open = h('button', { class: 'btn', type: 'button' }, 'Dosya aç…');
    open.addEventListener('click', () => input.click());
    const book = state.book;
    const meta = book
      ? `${book.format === 'gsi' ? 'Leica GSI' : 'Metin karne'} · ${this.unit() === 'grad' ? 'gon' : 'derece'} · ${book.stations.length} istasyon, ${book.stations.reduce((n, s) => n + s.observations.length, 0)} gözlem · ${book.encoding}`
      : '';
    replaceChildren(
      this.fileBox,
      open,
      input,
      state.file && book ? fileLine(state.file, meta) : h('span', { class: 'io-field__hint' }, 'Leica GSI dosyasını ya da sütunları eşlenecek bir CSV/TXT karneyi açın.'),
      state.error ? h('p', { class: 'note note--warn' }, state.error) : null,
    );
  }

  /** A file opened: an instrument's by its content, a text book with the mapping remembered (or its first line, to map). */
  async open(f: File): Promise<void> {
    if (f.size > LIMIT) {
      state.error = `“${f.name}” karne için çok büyük (${Math.floor(f.size / (1 << 20))} MiB); en çok 64 MiB okunur.`;
      this.render();
      return;
    }
    state.file = f.name;
    state.bytes = new Uint8Array(await f.arrayBuffer());
    state.error = null;
    await this.read();
  }

  /** The bytes read again (a text book's mapping changed); the edits start over. */
  private async read(): Promise<void> {
    if (!state.bytes) return;
    try {
      const book = await formats().readFieldBook(state.bytes, options());
      state.book = book;
      state.edits = book.stations.map(editsOf);
      state.backs = book.stations.map(() => 0);
      state.fore = null;
      state.station = Math.min(state.station, Math.max(0, book.stations.length - 1));
    } catch (e) {
      state.error = `Karne okunamadı: ${e instanceof Error ? e.message : String(e)}`;
    }
    this.render();
  }

  private renderMapping(): void {
    const book = state.book;
    if (!book || book.format !== 'csv') {
      replaceChildren(this.mappingBox);
      this.mappingBox.hidden = true;
      return;
    }
    this.mappingBox.hidden = false;
    const choices = [{ value: '', label: '—' }, ...book.firstLine.map((c, i) => ({ value: String(i), label: `${i + 1} · ${[...c].slice(0, 18).join('')}` }))];
    const fieldOf = (i: number) => {
      const c = state.mapping.columns[i];
      const value = c !== null && c < book.firstLine.length ? String(c) : '';
      return field(
        i === TARGET || i === HZ ? `${MAPPED[i]} *` : MAPPED[i],
        select(MAPPED[i], choices, value, (v) => {
          state.mapping.columns[i] = v === '' ? null : Number(v);
          void this.read();
        }),
      );
    };
    replaceChildren(
      this.mappingBox,
      h('h3', { class: 'calc-results__title' }, 'Sütunlar'),
      h('div', { class: 'fieldbook-mapping__grid' }, MAPPED.map((_, i) => fieldOf(i))),
      h(
        'div',
        { class: 'io-row' },
        checkField('Başlık', 'İlk satır başlık', state.mapping.header, (v) => ((state.mapping.header = v), void this.read()), 'header'),
        field(
          'Açı birimi',
          segmented<AngleUnit>({
            label: 'Açı birimi',
            value: this.unit(),
            options: [
              { value: 'grad', label: 'Grad' },
              { value: 'deg', label: 'Derece' },
            ],
            onChange: (v) => ((state.mapping.unit = v), this.render()),
          }),
        ),
      ),
      options() ? null : h('p', { class: 'io-field__hint' }, 'Nokta ve Yatay açı sütunlarını seçin; karne eşlenen sütunlarla okunur.'),
    );
  }

  private renderStation(): void {
    const book = state.book;
    if (!book || !book.stations.length) {
      replaceChildren(this.stationBox);
      return;
    }
    const st = book.stations[state.station]!;
    const stations = book.stations.map((s, i) => ({ value: String(i), label: `${s.station || `${i + 1}. istasyon (adsız)`} · ${s.observations.length} gözlem` }));
    const height = h('input', { class: 'field calc-num', value: state.edits[state.station]?.height ?? '', placeholder: '0', 'aria-label': 'Alet yüksekliği (m)' }) as HTMLInputElement;
    height.addEventListener('input', () => {
      const e = state.edits[state.station];
      if (e) e.height = height.value;
      this.sync();
    });
    const f = this.ctx.format;
    replaceChildren(
      this.stationBox,
      h(
        'div',
        { class: 'io-row' },
        field('İstasyon', select('İstasyon', stations, String(state.station), (v) => ((state.station = Number(v)), this.render()))),
        field('Alet yüksekliği (m)', height),
        this.backBox,
        st.east !== undefined && st.north !== undefined
          ? field('Dosyadaki koordinatlar', h('span', { class: 'num fieldbook-place' }, `${f.point({ x: st.east, y: st.north })}${st.height !== undefined ? `  Z ${fixed(st.height, 3)}` : ''}`))
          : null,
      ),
    );
  }

  /** The station shown again: its observations' table, its reduction with the project's k and tolerances (docs/adr/0169 §3). */
  private sync(): void {
    const book = state.book;
    const st = book?.stations[state.station];
    const edits = state.edits[state.station];
    this.reduction = null;
    this.from = [];
    if (!st || !edits) {
      replaceChildren(this.tableBox);
      replaceChildren(this.reducedBox);
      replaceChildren(this.backBox);
      replaceChildren(this.traverseBox);
      this.traverse = null;
      this.toTraverseButton.disabled = true;
      this.core = null;
      this.copy.disabled = true;
      this.transfer.disabled = true;
      this.showSummary();
      return;
    }
    const unit = this.unit();
    const settings = this.ctx.doc.settings;
    const survey = settings.survey.value;
    const { station, from } = coreStation(state.station)!;
    this.from = from;
    const tolerances = survey ? { faceHz: survey.faceHz, index: survey.index, faceSlope: survey.faceSlope, twoWay: survey.twoWay } : null;
    this.syncTraverse(unit, tolerances);
    this.reduction = fieldReduce(station, unit, settings.refraction, tolerances);
    this.core = { station, tolerances };
    // Kutupsal alım is oriented on a reduced row: the first, or the one chosen.
    const rowsReduced = this.reduction.rows;
    const back = Math.min(state.backs[state.station] ?? 0, Math.max(0, rowsReduced.length - 1));
    replaceChildren(
      this.backBox,
      rowsReduced.length
        ? field(
            'Geri bakış',
            select(
              'Geri bakış',
              rowsReduced.map((row, i) => ({ value: String(i), label: row.target })),
              String(back),
              (v) => ((state.backs[state.station] = Number(v)), this.sync()),
            ),
          )
        : null,
    );
    const polar = rowsReduced.length ? fieldPolar(station, unit, settings.refraction, tolerances, back, settings.angleUnit.value) : null;
    this.transfer.disabled = !polar?.shots.length;
    const faces = new Map<number, number | null>();
    this.from.forEach((i, k) => faces.set(i, this.reduction!.faces[k] ?? null));
    const { mark } = marks(unit);
    const rows = st.observations.map((o, i) => {
      const used = edits.rows[i]?.use !== '0';
      const face = !used ? '—' : (({ 1: 'I', 2: 'II', 0: 'Doğrultu' }) as Record<number, string>)[faces.get(i) ?? -1] ?? 'Geçersiz';
      Object.assign(edits.rows[i]!, { face, hz: fixed(o.hz, 5), zenith: shown(o.zenith, 5), slope: shown(o.slope, 4), th: shown(o.targetHeight, 3), code: o.code ?? '', line: String(o.line) });
      return edits.rows[i]!;
    });
    const model: GridModel = {
      columns: [
        { key: 'use', label: 'Kullan', check: true },
        { key: 'name', label: 'Nokta' },
        { key: 'face', label: 'Durum' },
        { key: 'hz', label: 'Yatay açı', unit: mark, numeric: true },
        { key: 'zenith', label: 'Başucu açısı', unit: mark, numeric: true },
        { key: 'slope', label: 'Eğik uzunluk', unit: 'm', numeric: true },
        { key: 'th', label: 'Prizma', unit: 'm', numeric: true },
        { key: 'code', label: 'Kod' },
        { key: 'line', label: 'Satır', numeric: true },
      ],
      rows: () => rows,
      addLabel: null,
      readonly: (_r, key) => key !== 'use' && key !== 'name',
      mark: (r) => (rows[r]?.use === '0' ? 'off' : null),
      canInsertAfter: () => false,
      insertAfter: () => {},
      canRemove: () => false,
      remove: () => {},
    };
    replaceChildren(this.tableBox, h('h3', { class: 'calc-results__title' }, 'Gözlemler'), new Grid(model, () => this.sync()).el);
    this.renderReduced(unit);
    this.showSummary();
  }

  /** The station's reduction: a row per target, the differences in cc or seconds and millimetres, one above its tolerance in the warning colour. */
  private renderReduced(unit: AngleUnit): void {
    const { mark, fine, per } = marks(unit);
    const r = this.reduction;
    const head = ['Nokta', 'Durum', `Yatay açı (${mark})`, `Fark (${fine})`, `Başucu açısı (${mark})`, `İndeks (${fine})`, 'Eğik uzunluk (m)', 'Fark (mm)', 'Yatay uzunluk (m)', 'Kot farkı (m)'];
    const numeric = [false, false, true, true, true, true, true, true, true, true];
    const rows = (r?.rows ?? []).map((row) => {
      const single = r!.faces[row.observations[0] ?? -1];
      const face = row.faces === 2 ? 'I + II' : single === 1 ? 'I' : single === 2 ? 'II' : 'Doğrultu';
      const over = (key: 'faceHz' | 'index' | 'faceSlope') => row.over.includes(key);
      return [
        [row.target, false],
        [face, false],
        [fixed(row.hz, 5), false],
        [shown(row.hzDiff === undefined || row.hzDiff === null ? null : row.hzDiff * per, 1), over('faceHz')],
        [shown(row.zenith, 5), false],
        [shown(row.index === undefined || row.index === null ? null : row.index * per, 1), over('index')],
        [shown(row.slope, 4), false],
        [shown(row.slopeDiff === undefined || row.slopeDiff === null ? null : row.slopeDiff * 1000, 1), over('faceSlope')],
        [shown(row.horizontal, 4), false],
        [shown(row.dh, 4), false],
      ] as [string, boolean][];
    });
    replaceChildren(
      this.reducedBox,
      h('h3', { class: 'calc-results__title' }, 'İndirgenmiş'),
      h(
        'div',
        { class: 'io-table-wrap calc-results' },
        h(
          'table',
          { class: 'io-table fieldbook-reduced' },
          h('thead', null, h('tr', null, head.map((c, i) => h('th', { class: numeric[i] ? 'num' : '' }, c)))),
          h('tbody', null, rows.map((cells) => h('tr', null, cells.map(([t, over], i) => h('td', { class: `${numeric[i] ? 'num' : ''}${over ? ' is-over' : ''}` }, t))))),
        ),
      ),
    );
    this.copy.disabled = !r?.rows.length;
  }

  /** What is said under the tables: the lines not read, the observations that are no face, the tolerances and what is above them, k. */
  private showSummary(): void {
    const book = state.book;
    const lines: HTMLElement[] = [];
    if (book) {
      for (const p of book.problems.slice(0, 6)) lines.push(summaryLine('warn', p.message));
      if (book.problems.length > 6) lines.push(summaryLine('warn', `… ${book.problems.length - 6} satır daha okunmadı.`));
    }
    const r = this.reduction;
    const st = book?.stations[state.station];
    if (r && st) {
      for (const u of r.problems) {
        const o = st.observations[this.from[u.observation] ?? -1];
        if (o) lines.push(summaryLine('warn', `Satır ${o.line}: ${o.target} noktasının başucu açısı (${shown(o.zenith, 5)}) bir durum değil; gözlem indirgenmedi.`));
      }
      const unit = this.unit();
      const { fine } = marks(unit);
      const settings = this.ctx.doc.settings;
      const [, hzText, indexText, slopeText, twoWayText] = surveyTexts(settings.survey.value, unit);
      const traverse = this.traverse;
      const given = [
        hzText ? `yatay fark ${hzText} ${fine}` : null,
        indexText ? `indeks ${indexText} ${fine}` : null,
        slopeText ? `uzunluk farkı ${slopeText} mm` : null,
        traverse && twoWayText ? `kenarın iki yönden farkı ${twoWayText} mm` : null,
      ].filter(Boolean);
      if (!given.length) lines.push(summaryLine('info', 'Tolerans verilmedi (Proje ayarları › Ölçme): farklar denetlenmedi.'));
      else {
        lines.push(summaryLine('info', `Toleranslar: ${given.join(', ')}.`));
        const over = r.rows.filter((row) => row.over.length).length;
        if (over) lines.push(summaryLine('warn', `${over} hedefte tolerans aşıldı.`));
        const legs = traverse?.legs.filter((l) => l.over).length ?? 0;
        if (legs) lines.push(summaryLine('warn', `${legs} kenarda iki yönden fark toleransı aşıldı.`));
      }
      for (const m of this.traverse?.missing.slice(0, 6) ?? []) lines.push(summaryLine('warn', `Poligon: ${m.station} istasyonunda ${m.target} gözlemi yok.`));
      lines.push(summaryLine('info', `Kot farkları yer eğriliği ve refraksiyonla, k = ${settings.refraction}.`));
    }
    summary(this.summaryBox, lines);
  }

  /**
   * Kutupsal alım'a aktar (docs/adr/0169 §3): its fields filled from the station shown (the station by its name when the
   * drawing has it, else by the file's coordinates), then it opens; what is opened here stays.
   */
  private toPolar(): void {
    const st = state.book?.stations[state.station];
    const core = this.core;
    if (!st || !core) return;
    const settings = this.ctx.doc.settings;
    const t = fieldPolar(core.station, this.unit(), settings.refraction, core.tolerances, state.backs[state.station] ?? 0, settings.angleUnit.value);
    if (!t?.shots.length) return;
    const known = resolvePoint(this.ctx, st.station);
    const station = known && !('error' in known) ? st.station : st.east !== undefined && st.north !== undefined ? `${st.east},${st.north}` : st.station;
    openPolarWith(this.ctx, {
      station,
      back: t.back,
      backReading: exact(t.backReading, 8),
      stationZ: st.height === undefined ? '' : exact(st.height, 6),
      instrumentHeight: state.edits[state.station]?.height ?? '',
      rows: t.shots.map((s) => ({ name: s.name, reading: exact(s.reading, 8), distance: exact(s.slope, 6), zenith: exact(s.zenith, 8), target: s.targetHeight === undefined ? '' : exact(s.targetHeight, 6) })),
    });
    this.ctx.log.success(`Karne editörü: ${t.shots.length} nokta Kutupsal alım'a aktarıldı (geri bakış ${t.back}).`);
    if (t.left.length) this.ctx.log.warn(`Uzunluğu ya da başucu açısı olmayan ${t.left.length} doğrultu aktarılmadı: ${t.left.join(', ')}.`);
    this.dialog.close();
  }

  /** The traverse through every station (two or more), its angles in the project's unit (docs/adr/0169 §3). */
  private syncTraverse(unit: AngleUnit, tolerances: Tolerances | null): void {
    this.traverse = null;
    this.lastRows = [];
    this.ends = null;
    const n = state.book?.stations.length ?? 0;
    const settings = this.ctx.doc.settings;
    const stations = Array.from({ length: n }, (_, i) => coreStation(i)?.station).filter((s): s is CoreStation => !!s);
    const first = stations[0] ? fieldReduce(stations[0], unit, settings.refraction, tolerances) : null;
    const last = n >= 2 && stations[n - 1] ? fieldReduce(stations[n - 1]!, unit, settings.refraction, tolerances) : null;
    const back = first?.rows[state.backs[0] ?? 0]?.target;
    if (n < 2 || stations.length !== n || !last || back === undefined) {
      replaceChildren(this.traverseBox);
      this.toTraverseButton.disabled = true;
      return;
    }
    this.lastRows = last.rows.map((r) => r.target);
    const fore = state.fore === null ? null : (last.rows[state.fore]?.target ?? null);
    this.traverse = fieldTraverse(stations, unit, settings.refraction, tolerances, back, fore, settings.angleUnit.value);
    this.ends = { back, fore };
    this.toTraverseButton.disabled = false;
    const legs = this.traverse.legs.map((l): [string, boolean][] => [
      [`${l.from} → ${l.to}`, false],
      [shown(l.forward, 4), false],
      [shown(l.backward, 4), false],
      [shown(l.mean, 4), false],
      [shown(l.diff === undefined ? null : l.diff * 1000, 1), l.over],
    ]);
    const numeric = [false, true, true, true, true];
    replaceChildren(
      this.traverseBox,
      h('h3', { class: 'calc-results__title' }, 'Poligon'),
      h(
        'div',
        { class: 'io-row' },
        field('İstasyonlar', h('span', { class: 'fieldbook-chain' }, this.traverse.stations.map((s) => s || '(adsız)').join(' → ')), undefined, 'grow'),
        field(
          'Bitişte bakılan',
          select(
            'Bitişte bakılan',
            [{ value: '', label: '—' }, ...this.lastRows.map((r, i) => ({ value: String(i), label: r }))],
            state.fore === null ? '' : String(state.fore),
            (v) => ((state.fore = v === '' ? null : Number(v)), this.sync()),
          ),
        ),
      ),
      h(
        'div',
        { class: 'io-table-wrap calc-results' },
        h(
          'table',
          { class: 'io-table fieldbook-legs' },
          h('thead', null, h('tr', null, ['Kenar', 'İleri (m)', 'Geri (m)', 'Ortalama (m)', 'Fark (mm)'].map((c, i) => h('th', { class: numeric[i] ? 'num' : '' }, c)))),
          h('tbody', null, legs.map((cells) => h('tr', null, cells.map(([c, over], i) => h('td', { class: `${numeric[i] ? 'num' : ''}${over ? ' is-over' : ''}` }, c))))),
        ),
      ),
    );
  }

  /**
   * Poligon hesabı'na aktar (docs/adr/0169 §3): a connected traverse through the stations, from the first (oriented on its
   * back sight) to the last (on the fore sight chosen, if any), each station's angle and each leg's mean distance; the
   * stations by their names when the drawing has them, else by the file's coordinates. Poligon hesabı opens.
   */
  private toTraverse(): void {
    const t = this.traverse;
    const ends = this.ends;
    const book = state.book;
    if (!t || !ends || !book) return;
    const n = t.stations.length;
    const place = (i: number): string => {
      const st = book.stations[i]!;
      const known = resolvePoint(this.ctx, st.station);
      return known && !('error' in known) ? st.station : st.east !== undefined && st.north !== undefined ? `${st.east},${st.north}` : st.station;
    };
    const angle = (i: number): string => {
      const a = t.angles[i];
      return a === null || a === undefined ? '' : exact(a, 8);
    };
    const leg = (i: number): string => {
      const m = t.legs[i]?.mean;
      return m === undefined ? '' : exact(m, 6);
    };
    openTraverseWith(this.ctx, {
      kind: 'connected',
      endOriented: ends.fore !== null,
      start: place(0),
      back: ends.back,
      end: place(n - 1),
      fore: ends.fore ?? '',
      first: { name: '', angle: angle(0), distance: leg(0) },
      rows: t.stations.slice(1, -1).map((name, k) => ({ name, angle: angle(k + 1), distance: leg(k + 1) })),
      last: { name: '', angle: ends.fore !== null ? angle(n - 1) : '', distance: '' },
    });
    this.ctx.log.success(`Karne editörü: ${n} istasyonlu poligon Poligon hesabı'na aktarıldı.`);
    if (t.missing.length)
      this.ctx.log.warn(`Poligonda bulunamayan gözlemler: ${t.missing.map((m) => `${m.station} istasyonunda ${m.target}`).join('; ')}. Açısı ya da kenarı olmayan satırları Poligon hesabı'nda yazın.`);
    this.dialog.close();
  }

  private copyReport(): void {
    const r = this.reduction;
    const st = state.book?.stations[state.station];
    if (!r || !st) return;
    const lines: string[][] = [[TITLE, state.file ?? ''], ['İstasyon', st.station, 'Alet yüksekliği', state.edits[state.station]?.height ?? '']];
    lines.push(['Nokta', 'Durum', 'Yatay açı', 'Fark', 'Başucu açısı', 'İndeks', 'Eğik uzunluk', 'Fark', 'Yatay uzunluk', 'Kot farkı']);
    for (const row of r.rows)
      lines.push([row.target, String(row.faces), fixed(row.hz, 5), shown(row.hzDiff, 5), shown(row.zenith, 5), shown(row.index, 5), shown(row.slope, 4), shown(row.slopeDiff, 4), shown(row.horizontal, 4), shown(row.dh, 4)]);
    copyReport(this.ctx, TITLE, lines);
  }
}
