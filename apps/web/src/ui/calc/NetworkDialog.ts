import type { AppContext } from '../../app/context';
import type { EntityGeometry as EditGeometry } from '../../contracts/generated/EntityGeometry';
import type { NewObject } from '../../contracts/generated/NewObject';
import { fixed } from '../../core/displayNumber';
import type { PointEntity } from '../../model/entities';
import { levelAdjust, networkAdjust, type LevelInput, type LevelResult, type LevelRow, type NetRow, type NetworkInput, type NetworkResult, type ObservationResult, type Statistics } from '../../model/geom/networkAdjust';
import { gridNote, surveyGrid } from '../../model/groundMeasures';
import { followPoint } from '../../model/ops/pointEditor';
import { hasSigmas, surveySigmas, type AngleUnit } from '../../model/projectSettings';
import { elevatedPaths } from '../../product/elevation';
import { entitiesCreate } from '../../product/entitiesCreate';
import { entitiesEdit } from '../../product/entitiesEdit';
import { entitiesSet } from '../../product/entitiesSet';
import { inStep, withPaths } from '../bottom/pointEdit';
import { h, replaceChildren } from '../dom';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { copyReport, field, Grid, layerChoice, readNumber, resultTable, summary, summaryLine, type GridColumn, type GridModel, type Row } from './common';

/**
 * Yatay ağ dengelemesi and Kot ağı dengelemesi (docs/adr/0203 §7; the desktop's `calc/network/`): two Hesap windows,
 * each with the known points and the observations side by side, adjusted by the geometry core at every change
 * (model/geom/networkAdjust.ts). A known point typed by its name alone is the drawing's point of that name; a new point
 * the drawing has starts the approximations where it is. The summary says what does not hold, or the counts, m0, the
 * model test and the observation to look at first; then the points' and the observations' tables, a flagged
 * observation in the danger colour. Çizime yaz writes the new points only (docs/adr/0203 §8): Yatay ağ moves the
 * drawing's points of the same name to their adjusted places, the vertices on them with them, and adds the others on
 * the chosen layer; Kot ağı gives the drawing's points their adjusted heights. One undo step named after the window.
 * What is typed stays for the session.
 */
export const NETWORK_TITLE = 'Yatay ağ dengelemesi';
export const LEVEL_TITLE = 'Kot ağı dengelemesi';
/** The kind of the points Çizime yaz adds (their `Tür`). */
const POINT_KIND = 'Ağ noktası';

type LevelKind = LevelInput['kind'];

const KNOWN_KEYS = ['name', 'y', 'x', 'sigma'] as const;
const ROW_KEYS = ['station', 'target', 'direction', 'distance'] as const;
const HEIGHT_KEYS = ['name', 'h', 'sigma'] as const;
const LEVEL_KEYS = ['from', 'to', 'dh', 'length'] as const;
const empty = (keys: readonly string[]): Row => Object.fromEntries(keys.map((k) => [k, '']));

const state = {
  network: { known: [empty(KNOWN_KEYS)], rows: [empty(ROW_KEYS)], layer: null as string | null },
  level: { kind: 'geometric' as LevelKind, known: [empty(HEIGHT_KEYS)], rows: [empty(LEVEL_KEYS)] },
};

export function openNetwork(ctx: AppContext): void {
  new NetworkDialog(ctx, 'network');
}

export function openLevel(ctx: AppContext): void {
  new NetworkDialog(ctx, 'level');
}

/** Karne editörü's Ağ dengelemesine aktar (docs/adr/0203 §6): its rows as the observations (then one empty row); the known points stay. */
export function openNetworkWith(ctx: AppContext, rows: readonly Row[]): void {
  state.network.rows = [...rows.map((r) => ({ ...empty(ROW_KEYS), ...r })), empty(ROW_KEYS)];
  openNetwork(ctx);
}

/** Karne editörü's Kot ağına aktar: its height differences, the kind trigonometric. */
export function openLevelWith(ctx: AppContext, rows: readonly Row[]): void {
  state.level.rows = [...rows.map((r) => ({ ...empty(LEVEL_KEYS), ...r })), empty(LEVEL_KEYS)];
  state.level.kind = 'trigonometric';
  openLevel(ctx);
}

/** A name as names are compared: trimmed, in Turkish capitals. */
const key = (name: string) => name.trim().toLocaleUpperCase('tr-TR');

/** The drawing's points named `name` (their label, else their `Ad`; compared the Turkish way). */
function namedPoints(ctx: AppContext, name: string): PointEntity[] {
  const k = key(name);
  const out: PointEntity[] = [];
  for (const e of ctx.doc.all()) {
    if (e.kind !== 'point') continue;
    const n = e.label ?? e.attrs.Ad ?? '';
    if (n && key(n) === k) out.push(e);
  }
  return out;
}

const filled = (r: Row) => Object.values(r).some((v) => v?.trim());
const cell = (r: Row | undefined, k: string) => (r?.[k] ?? '').trim();

/** A small angle (radians) in the project's fine unit: cc in a gon project, ″ in a degree one. */
function fineAngle(v: number, unit: AngleUnit): string {
  return unit === 'grad' ? `${fixed((v * 2_000_000) / Math.PI, 1)} cc` : `${fixed((v * 648_000) / Math.PI, 1)}″`;
}

/** A small length (m) in millimetres. */
const mm = (v: number) => `${fixed(v * 1000, 1)} mm`;

/** A table of typed rows, any of which may be added after or removed (one stays). */
function rowsModel(columns: GridColumn[], rows: () => Row[], keys: readonly string[]): GridModel {
  return {
    columns,
    rows,
    readonly: () => false,
    canInsertAfter: () => true,
    insertAfter: (r) => rows().splice(r + 1, 0, empty(keys)),
    canRemove: () => rows().length > 1,
    remove: (r) => rows().splice(r, 1),
  };
}

type Line = ['ok' | 'warn' | 'error', string];

/** What the summary says of an adjustment: its counts, vᵀPv, m0 and the model test (the desktop's `summary_lines`). */
function summaryTexts(r: Statistics, worst: string | null): Line[] {
  const out: Line[] = [['ok', `${r.n} gözlem, ${r.u} bilinmeyen, serbestlik derecesi ${r.f}; ${r.iterations} yineleme.`]];
  if (r.m0 !== undefined && r.chi2 !== undefined && r.passed === true)
    out.push(['ok', `m₀ = ${fixed(r.m0, 3)} (önsel 1); model testi geçti: vᵀPv ${fixed(r.omega, 3)} ≤ χ²₀,₉₅(${r.f}) ${fixed(r.chi2, 3)}.`]);
  else if (r.m0 !== undefined && r.chi2 !== undefined)
    out.push(['warn', `m₀ = ${fixed(r.m0, 3)} (önsel 1); model testi kaldı: vᵀPv ${fixed(r.omega, 3)} > χ²₀,₉₅(${r.f}) ${fixed(r.chi2, 3)}. Önsel doğrulukları ya da ölçüleri denetleyin.`]);
  else out.push(['warn', 'Serbestlik derecesi 0: ölçüler denetlenemez; doğruluklar önsel ağırlıklarla.']);
  if (worst) out.push(['warn', worst]);
  return out;
}

/** The flagged observation with the greatest test value, said. */
const worstLine = (o: ObservationResult, what: string) => `Uyuşumsuz ölçü olabilir: ${what} (w ${fixed(o.w ?? 0, 2)}); önce bu ölçüyü denetleyin.`;

/** An observation's kind as the table names it. */
const KIND_WORD: Record<ObservationResult['kind'], string> = {
  direction: 'Doğrultu',
  distance: 'Kenar',
  y: 'Y (ağırlıklı)',
  x: 'X (ağırlıklı)',
  dh: 'Kot farkı',
  h: 'Kot (ağırlıklı)',
};

/** A results table whose `marked` rows read in the danger colour (an observation the test flags, docs/adr/0203 §4). */
function markedTable(head: string[], rows: string[][], numeric: readonly boolean[], marked: readonly boolean[]): HTMLElement {
  const el = resultTable(head, rows, numeric);
  el.querySelectorAll<HTMLElement>('tbody tr').forEach((tr, i) => {
    if (marked[i]) tr.dataset.flag = 'blunder';
  });
  return el;
}

/** A table under its title and what it takes. */
const titled = (title: string, note: HTMLElement | string, table: HTMLElement) =>
  h('div', { class: 'calc-tables__side' }, h('h3', { class: 'calc-results__title' }, title), typeof note === 'string' ? h('p', { class: 'io-field__hint' }, note) : note, table);

/** A full turn in the project's angle unit. */
const fullTurn = (unit: AngleUnit) => (unit === 'grad' ? 400 : 360);

class NetworkDialog {
  private readonly ctx: AppContext;
  private readonly kind: 'network' | 'level';
  private readonly results = h('div', { class: 'calc-section' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly add = h('button', { class: 'btn btn--primary', type: 'button' }, 'Çizime yaz');
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly dialog: Dialog;
  /** The last adjustment and, for each of its rows, the table's row it was typed on. */
  private network: { result: NetworkResult | null; lines: number[] } = { result: null, lines: [] };
  private level: { result: LevelResult | null; lines: number[] } = { result: null, lines: [] };

  constructor(ctx: AppContext, kind: 'network' | 'level') {
    this.ctx = ctx;
    this.kind = kind;
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    const content = kind === 'network' ? this.networkForm() : this.levelForm();
    const footer: HTMLElement[] = [h('div', { class: 'dialog__spacer' }), this.copy];
    if (kind === 'network') footer.push(h('span', { class: 'calc-foot-label' }, 'Katman'), layerChoice(ctx, state.network, 'poligon'));
    footer.push(this.add, close);
    this.dialog = new Dialog({
      title: kind === 'network' ? NETWORK_TITLE : LEVEL_TITLE,
      width: 1040,
      className: 'dialog--io dialog--calc dialog--network',
      content: [...content, this.summaryBox, this.results],
      footer,
    });
    close.addEventListener('click', () => this.dialog.close());
    this.add.addEventListener('click', () => (kind === 'network' ? this.writeNetwork() : this.writeLevels()));
    this.copy.addEventListener('click', () => this.copyReport());
    this.compute();
  }

  private networkForm(): HTMLElement[] {
    const unit = this.ctx.doc.settings.angleUnit.value;
    const known = new Grid(
      rowsModel(
        [
          { key: 'name', label: 'Ad' },
          { key: 'y', label: 'Y', unit: 'm', numeric: true, placeholder: () => 'çizimdeki' },
          { key: 'x', label: 'X', unit: 'm', numeric: true, placeholder: () => 'çizimdeki' },
          { key: 'sigma', label: 'σ', unit: 'mm', numeric: true, placeholder: () => 'sabit' },
        ],
        () => state.network.known,
        KNOWN_KEYS,
      ),
      () => this.compute(),
    );
    const rows = new Grid(
      rowsModel(
        [
          { key: 'station', label: 'Durulan' },
          { key: 'target', label: 'Bakılan' },
          { key: 'direction', label: 'Doğrultu', unit: unit === 'grad' ? 'g' : '°', numeric: true },
          { key: 'distance', label: 'Kenar', unit: 'm', numeric: true },
        ],
        () => state.network.rows,
        ROW_KEYS,
      ),
      () => this.compute(),
    );
    const grid = surveyGrid(this.ctx.doc.settings);
    return [
      h(
        'div',
        { class: 'calc-tables' },
        titled('Bilinen noktalar', 'Y ve X boşsa çizimdeki aynı adlı nokta; σ boşsa sabit, yazılırsa ağırlıklı.', known.el),
        titled('Gözlemler', 'Bir istasyonun bütün doğrultuları bir seridir; kenar yataydır. Elektronik tablodan satırlar yapıştırılabilir.', rows.el),
      ),
      h('p', { class: 'io-field__hint' }, [this.sigmaLine(null), ...(grid ? [gridNote(grid.height)] : [])].join(' ')),
    ];
  }

  private levelForm(): HTMLElement[] {
    const kindBox = h('div', { class: 'calc-network-kind' });
    const lengthNote = h('p', { class: 'io-field__hint' });
    const sigma = h('p', { class: 'io-field__hint' });
    const show = () => {
      replaceChildren(
        kindBox,
        segmented<LevelKind>({
          label: 'Ölçü türü',
          value: state.level.kind,
          options: [
            { value: 'geometric', label: 'Geometrik nivelman' },
            { value: 'trigonometric', label: 'Trigonometrik' },
          ],
          onChange: (v) => {
            state.level.kind = v;
            show();
            this.compute();
          },
        }),
      );
      lengthNote.textContent = state.level.kind === 'geometric' ? 'Uzunluk nivelman hattınındır.' : 'Uzunluk yatay uzunluktur.';
      sigma.textContent = this.sigmaLine(state.level.kind);
    };
    show();
    const known = new Grid(
      rowsModel(
        [
          { key: 'name', label: 'Ad' },
          { key: 'h', label: 'Kot', unit: 'm', numeric: true, placeholder: () => 'çizimdeki' },
          { key: 'sigma', label: 'σ', unit: 'mm', numeric: true, placeholder: () => 'sabit' },
        ],
        () => state.level.known,
        HEIGHT_KEYS,
      ),
      () => this.compute(),
    );
    const rows = new Grid(
      rowsModel(
        [
          { key: 'from', label: 'Başlangıç' },
          { key: 'to', label: 'Bitiş' },
          { key: 'dh', label: 'Kot farkı', unit: 'm', numeric: true },
          { key: 'length', label: 'Uzunluk', unit: 'm', numeric: true },
        ],
        () => state.level.rows,
        LEVEL_KEYS,
      ),
      () => this.compute(),
    );
    return [
      field('Ölçü türü', kindBox),
      h(
        'div',
        { class: 'calc-tables' },
        titled('Bilinen kotlar', 'Kot boşsa çizimdeki aynı adlı noktanın kotu; σ boşsa sabit, yazılırsa ağırlıklı.', known.el),
        titled('Kot farkları', lengthNote, rows.el),
      ),
      sigma,
    ];
  }

  /** The a priori standard deviations the adjustment weighs with (the project's, docs/adr/0203 §1). */
  private sigmaLine(level: LevelKind | null): string {
    const settings = this.ctx.doc.settings;
    const survey = settings.survey.value;
    const s = surveySigmas(survey);
    const unit = settings.angleUnit.value;
    const whose = hasSigmas(survey) ? 'projenin' : 'varsayılan';
    const what =
      level === null
        ? `doğrultu ${fineAngle(s.direction, unit)}, kenar ${mm(s.distance)} + ${fixed(s.ppm, 1)} ppm, merkezleme ${mm(s.centering)}`
        : level === 'geometric'
          ? `nivelman ${mm(s.levelling)} /√km`
          : `başucu açısı ${fineAngle(s.zenith, unit)}, kenar ${mm(s.distance)} + ${fixed(s.ppm, 1)} ppm`;
    return `Önsel doğruluklar (${whose}): ${what}. Proje ayarları › Ölçme'de değiştirilir.`;
  }

  /** The core's input from the tables and the drawing, or what does not hold (docs/adr/0203 §7). */
  private networkInput(): { input: NetworkInput | null; problems: string[]; lines: number[] } {
    const settings = this.ctx.doc.settings;
    const problems: string[] = [];
    const known: NetworkInput['known'] = [];
    state.network.known.forEach((r, i) => {
      if (!filled(r)) return;
      const name = cell(r, 'name');
      if (!name) return void problems.push(`${i + 1}. bilinen noktanın adı yok.`);
      const [y, x] = [readNumber(r.y), readNumber(r.x)];
      let p: [number, number];
      if (y === null && x === null) {
        const there = namedPoints(this.ctx, name)[0];
        if (!there) return void problems.push(`${name}: çizimde bu adla nokta yok; Y ve X yazın.`);
        p = [there.p.x, there.p.y];
      } else if (y !== null && x !== null) {
        if (!Number.isFinite(y) || !Number.isFinite(x)) return void problems.push(`${name}: Y ya da X bir sayı değil.`);
        p = [y, x];
      } else return void problems.push(`${name}: Y ve X birlikte yazılır.`);
      const s = readNumber(r.sigma);
      if (s !== null && !(Number.isFinite(s) && s > 0)) return void problems.push(`${name}: σ sıfırdan büyük bir sayı olmalı (mm).`);
      known.push({ name, y: p[0], x: p[1], sigma: s === null ? null : s / 1000 });
    });
    const rows: NetRow[] = [];
    const lines: number[] = [];
    state.network.rows.forEach((r, i) => {
      if (!filled(r)) return;
      const [station, target] = [cell(r, 'station'), cell(r, 'target')];
      if (!station || !target) return void problems.push(`${i + 1}. gözlemde durulan ya da bakılan yok.`);
      const [direction, distance] = [readNumber(r.direction), readNumber(r.distance)];
      if (direction === null && distance === null) return void problems.push(`${i + 1}. gözlemde doğrultu ya da kenar yok.`);
      if (direction !== null && !Number.isFinite(direction)) return void problems.push(`${i + 1}. gözlemde doğrultu bir sayı değil.`);
      if (distance !== null && !Number.isFinite(distance)) return void problems.push(`${i + 1}. gözlemde kenar bir sayı değil.`);
      rows.push({ station, target, direction, distance, line: i + 1 });
      lines.push(i);
    });
    if (problems.length) return { input: null, problems, lines };
    // The new points' places in the drawing, where it has them.
    const approx: NetworkInput['approx'] = [];
    for (const r of rows)
      for (const name of [r.station, r.target]) {
        if (approx.some((a) => a.name === name)) continue;
        const there = namedPoints(this.ctx, name)[0];
        if (there) approx.push({ name, y: there.p.x, x: there.p.y });
      }
    const input: NetworkInput = { unit: settings.angleUnit.value, sigma: surveySigmas(settings.survey.value), grid: surveyGrid(settings), known, approx, rows };
    return { input, problems, lines };
  }

  /** The core's input from the tables and the drawing, or what does not hold. */
  private levelInput(): { input: LevelInput | null; problems: string[]; lines: number[] } {
    const problems: string[] = [];
    const known: LevelInput['known'] = [];
    state.level.known.forEach((r, i) => {
      if (!filled(r)) return;
      const name = cell(r, 'name');
      if (!name) return void problems.push(`${i + 1}. bilinen noktanın adı yok.`);
      let height = readNumber(r.h);
      if (height === null) {
        const z = namedPoints(this.ctx, name).find((p) => p.z !== undefined)?.z;
        if (z === undefined) return void problems.push(`${name}: çizimde bu adla kotlu nokta yok; kotu yazın.`);
        height = z;
      } else if (!Number.isFinite(height)) return void problems.push(`${name}: kot bir sayı değil.`);
      const s = readNumber(r.sigma);
      if (s !== null && !(Number.isFinite(s) && s > 0)) return void problems.push(`${name}: σ sıfırdan büyük bir sayı olmalı (mm).`);
      known.push({ name, h: height, sigma: s === null ? null : s / 1000 });
    });
    const rows: LevelRow[] = [];
    const lines: number[] = [];
    state.level.rows.forEach((r, i) => {
      if (!filled(r)) return;
      const [from, to] = [cell(r, 'from'), cell(r, 'to')];
      if (!from || !to) return void problems.push(`${i + 1}. gözlemde başlangıç ya da bitiş yok.`);
      const [dh, length] = [readNumber(r.dh), readNumber(r.length)];
      if (dh === null || length === null) return void problems.push(`${i + 1}. gözlemde kot farkı ya da uzunluk yok.`);
      if (!Number.isFinite(dh) || !Number.isFinite(length)) return void problems.push(`${i + 1}. gözlemde kot farkı ya da uzunluk bir sayı değil.`);
      rows.push({ from, to, dh, length, line: i + 1 });
      lines.push(i);
    });
    if (problems.length) return { input: null, problems, lines };
    return { input: { kind: state.level.kind, sigma: surveySigmas(this.ctx.doc.settings.survey.value), known, rows }, problems, lines };
  }

  /** The tables adjusted again (at every change): the summary and the results. */
  private compute(): void {
    let lines: Line[] = [];
    replaceChildren(this.results);
    let done = false;
    const failed = (e: unknown): Line[] => [['error', e instanceof Error ? e.message : String(e)]];
    if (this.kind === 'network') {
      const { input, problems, lines: typed } = this.networkInput();
      this.network = { result: null, lines: typed };
      if (!input) lines = problems.slice(0, 6).map((p): Line => ['warn', p]);
      else
        try {
          const r = networkAdjust(input);
          this.network.result = r;
          done = true;
          lines = summaryTexts(r, r.worst !== undefined ? worstLine(r.observations[r.worst]!, this.observationName(r.observations[r.worst]!)) : null);
          this.showNetwork(r);
        } catch (e) {
          lines = failed(e);
        }
    } else {
      const { input, problems, lines: typed } = this.levelInput();
      this.level = { result: null, lines: typed };
      if (!input) lines = problems.slice(0, 6).map((p): Line => ['warn', p]);
      else
        try {
          const r = levelAdjust(input);
          this.level.result = r;
          done = true;
          lines = summaryTexts(r, r.worst !== undefined ? worstLine(r.observations[r.worst]!, this.observationName(r.observations[r.worst]!)) : null);
          this.showLevel(r);
        } catch (e) {
          lines = failed(e);
        }
    }
    summary(
      this.summaryBox,
      lines.map(([k, t]) => summaryLine(k, t)),
    );
    this.add.disabled = !done;
    this.copy.disabled = !done;
  }

  /** The names of the known points the table types (their rows' names). */
  private knownNames(): string[] {
    const rows = this.kind === 'network' ? state.network.known : state.level.known;
    return rows.filter(filled).map((r) => cell(r, 'name')).filter(Boolean);
  }

  /** The table row a given adjusted row was typed on. */
  private typedRow(row: number): Row | undefined {
    return this.kind === 'network' ? state.network.rows[this.network.lines[row] ?? -1] : state.level.rows[this.level.lines[row] ?? -1];
  }

  /** A given row's two names as typed. */
  private rowNames(row: number): [string, string] {
    const r = this.typedRow(row);
    return this.kind === 'network' ? [cell(r, 'station'), cell(r, 'target')] : [cell(r, 'from'), cell(r, 'to')];
  }

  /** An observation as the summary names it: “K2 → Y1 kenarı”, “N2 → R2 kot farkı”. */
  private observationName(o: ObservationResult): string {
    const names = this.knownNames();
    if (o.kind === 'y' || o.kind === 'x') return `${names[o.row] ?? ''} noktasının ${o.kind === 'y' ? 'Y' : 'X'}'i`;
    if (o.kind === 'h') return `${names[o.row] ?? ''} noktasının kotu`;
    const [a, b] = this.rowNames(o.row);
    return `${a} → ${b} ${o.kind === 'direction' ? 'doğrultusu' : o.kind === 'distance' ? 'kenarı' : 'kot farkı'}`;
  }

  /** The points' rows (name, place, standard deviations, error ellipse, the shift from the drawing's point) and the observations'. */
  private networkRows(r: NetworkResult): { points: string[][]; observations: string[][]; marked: boolean[] } {
    const fmt = this.ctx.format.metric();
    const unit = this.ctx.doc.settings.angleUnit.value;
    const points = r.points.map((p) => {
      const there = namedPoints(this.ctx, p.name)[0];
      const shift = there ? fixed(Math.sqrt((there.p.x - p.y) ** 2 + (there.p.y - p.x) ** 2) * 1000, 1) : 'yeni';
      const ellipse = [p.sy, p.sx, p.sp, p.a, p.b].map((v) => fixed(v * 1000, 1));
      return [p.name, fmt.coord(p.y), fmt.coord(p.x), ...ellipse, fixed((p.theta * fullTurn(unit)) / (2 * Math.PI), 2), shift];
    });
    const names = this.knownNames();
    const observations: string[][] = [];
    const marked: boolean[] = [];
    for (const o of r.observations) {
      const [a, b] = o.kind === 'y' || o.kind === 'x' ? [names[o.row] ?? '', '—'] : this.rowNames(o.row);
      const typed = this.typedRow(o.row);
      const [observed, v, sigma] =
        o.kind === 'direction' ? [cell(typed, 'direction'), fineAngle(o.v, unit), fineAngle(o.sigma, unit)] : o.kind === 'distance' ? [cell(typed, 'distance'), mm(o.v), mm(o.sigma)] : ['', mm(o.v), mm(o.sigma)];
      observations.push([a, b, KIND_WORD[o.kind], observed, v, sigma, fixed(o.r, 2), o.w === undefined ? '—' : fixed(o.w, 2)]);
      marked.push(o.flag === 'blunder');
    }
    return { points, observations, marked };
  }

  private showNetwork(r: NetworkResult): void {
    const { points, observations, marked } = this.networkRows(r);
    const theta = this.ctx.doc.settings.angleUnit.value === 'grad' ? 'θ (g)' : 'θ (°)';
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Noktalar'),
      resultTable(['Nokta', 'Y (sağa)', 'X (yukarı)', 'σY (mm)', 'σX (mm)', 'σP (mm)', 'a (mm)', 'b (mm)', theta, 'Kayma (mm)'], points, [false, ...Array<boolean>(9).fill(true)]),
      h('h3', { class: 'calc-results__title' }, 'Gözlemler'),
      markedTable(['Durulan', 'Bakılan', 'Tür', 'Ölçü', 'v', 'σ', 'r', 'w'], observations, [false, false, false, true, true, true, true, true], marked),
    );
  }

  /** The points' rows (name, height, its standard deviation, the change from the drawing's height) and the observations'. */
  private levelRows(r: LevelResult): { points: string[][]; observations: string[][]; marked: boolean[] } {
    const fmt = this.ctx.format.metric();
    const points = r.points.map((p) => {
      const z = namedPoints(this.ctx, p.name)[0]?.z;
      return [p.name, fmt.length(p.h, false), fixed(p.sh * 1000, 1), z === undefined ? '—' : fixed((p.h - z) * 1000, 1)];
    });
    const names = this.knownNames();
    const observations: string[][] = [];
    const marked: boolean[] = [];
    for (const o of r.observations) {
      const [a, b] = o.kind === 'h' ? [names[o.row] ?? '', '—'] : this.rowNames(o.row);
      const observed = o.kind === 'h' ? '' : cell(this.typedRow(o.row), 'dh');
      observations.push([a, b, observed, mm(o.v), mm(o.sigma), fixed(o.r, 2), o.w === undefined ? '—' : fixed(o.w, 2)]);
      marked.push(o.flag === 'blunder');
    }
    return { points, observations, marked };
  }

  private showLevel(r: LevelResult): void {
    const { points, observations, marked } = this.levelRows(r);
    replaceChildren(
      this.results,
      h('h3', { class: 'calc-results__title' }, 'Noktalar'),
      resultTable(['Nokta', 'Kot (m)', 'σH (mm)', 'Çizimdeki kottan (mm)'], points, [false, true, true, true]),
      h('h3', { class: 'calc-results__title' }, 'Gözlemler'),
      markedTable(['Başlangıç', 'Bitiş', 'Kot farkı (m)', 'v', 'σ', 'r', 'w'], observations, [false, false, true, true, true, true, true], marked),
    );
  }

  /** Raporu kopyala: the title, the summary, the points, the observations (Yatay ağ: the orientations too). */
  private copyReport(): void {
    if (this.kind === 'network') {
      const r = this.network.result;
      if (!r) return;
      const { points, observations } = this.networkRows(r);
      const full = fullTurn(this.ctx.doc.settings.angleUnit.value);
      const worst = r.worst !== undefined ? worstLine(r.observations[r.worst]!, this.observationName(r.observations[r.worst]!)) : null;
      copyReport(this.ctx, NETWORK_TITLE, [
        [NETWORK_TITLE],
        ...summaryTexts(r, worst).map(([, t]) => [t]),
        [],
        ['Nokta', 'Y (sağa)', 'X (yukarı)', 'σY (mm)', 'σX (mm)', 'σP (mm)', 'a (mm)', 'b (mm)', 'θ', 'Kayma (mm)'],
        ...points,
        [],
        ['Durulan', 'Bakılan', 'Tür', 'Ölçü', 'v', 'σ', 'r', 'w'],
        ...observations,
        [],
        ['İstasyon', 'Yöneltme'],
        ...r.orientations.map((o) => [o.station, fixed((o.z * full) / (2 * Math.PI), 5)]),
      ]);
      return;
    }
    const r = this.level.result;
    if (!r) return;
    const { points, observations } = this.levelRows(r);
    const worst = r.worst !== undefined ? worstLine(r.observations[r.worst]!, this.observationName(r.observations[r.worst]!)) : null;
    copyReport(this.ctx, LEVEL_TITLE, [
      [LEVEL_TITLE, state.level.kind === 'geometric' ? 'Geometrik nivelman' : 'Trigonometrik'],
      ...summaryTexts(r, worst).map(([, t]) => [t]),
      [],
      ['Nokta', 'Kot (m)', 'σH (mm)', 'Çizimdeki kottan (mm)'],
      ...points,
      [],
      ['Başlangıç', 'Bitiş', 'Kot farkı (m)', 'v', 'σ', 'r', 'w'],
      ...observations,
    ]);
  }

  /** Whether `name` is one of the known points' (compared the Turkish way). */
  private known(name: string): boolean {
    const k = key(name);
    return this.knownNames().some((n) => key(n) === k);
  }

  /** Yatay ağ dengelemesi's Çizime yaz: the new points moved or added (docs/adr/0203 §8; the desktop's `network_write`). */
  private writeNetwork(): void {
    const { ctx } = this;
    const doc = ctx.doc;
    const r = this.network.result;
    if (!r) return;
    const moves: Change[] = [];
    const adds: { name: string; p: { x: number; y: number } }[] = [];
    for (const p of r.points.filter((p) => !this.known(p.name))) {
      const there = namedPoints(ctx, p.name);
      if (!there.length) adds.push({ name: p.name, p: { x: p.y, y: p.x } });
      for (const e of there) moves.push({ e, to: { x: p.y, y: p.x }, z: e.z ?? null });
    }
    if (!moves.length && !adds.length) return;
    const layer = state.network.layer;
    const written: number[] = [];
    const refused = inStep(doc, NETWORK_TITLE, () => {
      if (moves.length) {
        const res = entitiesEdit.execute({ doc }, { operation: 'networkAdjust', changes: edits(ctx, moves, false) });
        if (res.status !== 'completed') return 'error' in res ? res.error.message : null;
        written.push(...moves.map((m) => m.e.id));
      }
      if (adds.length) {
        if (!layer) return 'Yeni noktalar için katman seçin.';
        const objects: NewObject[] = adds.map((a) => ({ geometry: { kind: 'point', p: a.p }, label: a.name, attrs: { Ad: a.name, Tür: POINT_KIND } }));
        const res = entitiesCreate.execute({ doc }, { layerId: layer, objects, operation: 'networkAdjust' });
        if (res.status !== 'completed') return 'error' in res ? res.error.message : null;
        written.push(...res.output.ids);
      }
      return null;
    });
    if (refused) {
      ctx.log.warn(refused);
      return;
    }
    ctx.selection.set(written);
    ctx.log.success(`${NETWORK_TITLE}: ${moves.length} nokta dengelenmiş yerine taşındı, ${adds.length} nokta eklendi (Ctrl+Z geri alır).`);
    this.dialog.close();
  }

  /** Kot ağı dengelemesi's Çizime yaz: the new points' heights, their `Z (m)` and the vertices on them (the desktop's `level_write`). */
  private writeLevels(): void {
    const { ctx } = this;
    const doc = ctx.doc;
    const r = this.level.result;
    if (!r) return;
    const changes: Change[] = [];
    const absent: string[] = [];
    for (const p of r.points.filter((p) => !this.known(p.name))) {
      const there = namedPoints(ctx, p.name);
      if (!there.length) absent.push(p.name);
      for (const e of there) changes.push({ e, to: e.p, z: p.h });
    }
    const notThere = () => {
      if (absent.length) ctx.log.warn(`${LEVEL_TITLE}: ${absent.join(', ')} çizimde yok; yazılmadı.`);
    };
    if (!changes.length) return notThere();
    const refused = inStep(doc, LEVEL_TITLE, () => {
      const res = entitiesEdit.execute({ doc }, { operation: 'levelAdjust', changes: edits(ctx, changes, true) });
      if (res.status !== 'completed') return 'error' in res ? res.error.message : null;
      // The Z (m) the Hesap windows write beside it follows (three decimals).
      for (const c of changes) {
        if (!('Z (m)' in c.e.attrs) || c.z === null) continue;
        const set = entitiesSet.execute({ doc }, { uids: [doc.uidOf(c.e.id) ?? ''], attrs: { 'Z (m)': fixed(c.z, 3) }, operation: 'attributes' });
        if (set.status !== 'completed') return 'error' in set ? set.error.message : null;
      }
      return null;
    });
    if (refused) {
      ctx.log.warn(refused);
      return;
    }
    ctx.selection.set(changes.map((c) => c.e.id));
    ctx.log.success(`${LEVEL_TITLE}: ${changes.length} noktanın kotu yazıldı (Ctrl+Z geri alır).`);
    notThere();
    this.dialog.close();
  }
}

/** A point to move, raise or lower: what it becomes. */
interface Change {
  e: PointEntity;
  to: { x: number; y: number };
  z: number | null;
}

/** The edits of `changes`: each point and the vertices of lines, polylines and areas on it (the point editor's rule, docs/adr/0153 §3). */
function edits(ctx: AppContext, changes: readonly Change[], setZ: boolean) {
  const doc = ctx.doc;
  const out: { kind: 'update'; uid: string; geometry: EditGeometry }[] = [];
  for (const c of changes) {
    const from = c.e.p;
    // A multi-point object keeps its other points (docs/adr/0174).
    out.push({ kind: 'update', uid: doc.uidOf(c.e.id) ?? '', geometry: { kind: 'point', p: c.to, ...(c.z !== null && { z: c.z }), ...(c.e.parts && { parts: c.e.parts }) } as EditGeometry });
    for (const other of doc.all()) {
      if (other.kind !== 'line' && other.kind !== 'polyline' && other.kind !== 'polygon') continue;
      const moved = followPoint(elevatedPaths(other), from, c.to, setZ, setZ ? c.z : null);
      const geometry = moved && withPaths(other, moved);
      if (geometry) out.push({ kind: 'update', uid: doc.uidOf(other.id) ?? '', geometry });
    }
  }
  return out;
}
