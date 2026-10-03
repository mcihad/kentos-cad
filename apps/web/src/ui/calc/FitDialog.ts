import type { AppContext } from '../../app/context';
import type { Transform } from '../../contracts/generated/Transform';
import type { Vec2 } from '../../model/geometry';
import { FIT_NEED, fitTransform, type Fit, type FitFailure, type FitKind, type FitPair } from '../../model/ops/fit';
import { entitiesTransform } from '../../product/entitiesTransform';
import { PickPointTool } from '../../tools/pickPointTool';
import { fixed } from '../../core/displayNumber';
import { h, replaceChildren } from '../dom';
import { checkField, select } from '../io/common';
import { segmented, textField } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { copyReport, field, Grid, knownField, mmText, nameAt, readNumber, summary, summaryLine, type GridModel, type Row } from './common';
import { METHOD_HINT, METHODS, PARAMS, PARAMS_HINT, paramLabel, paramsReport, paramsSummary, paramsTransform, readParams, type Method, type Params } from './fitParams';
import { linksOf, RUBBER_HINT, rubberReport, rubberSummary, type Corrections, type Links } from './fitRubber';

/**
 * Vektör oturtma (docs/adr/0156 §7): a drawing or a layer fitted to another
 * system by control points. Each row is a pair: where the point is in the
 * drawing (source) and where it is to go (target), typed, pasted from a
 * spreadsheet or shown on the drawing; Kullan leaves a pair out of the
 * solution (its residual is still worked out). The solution is the geometry
 * core's (`fitTransform`): Helmert, affine or projective by least squares,
 * every pair's residual and m0, worked out again at every change. Adla eşle
 * fills the table with the points of the same name on two layers. Uygula
 * writes the transform through `cad.entities.transform` (one undo step,
 * Oturt) to the selected objects, a layer or the whole drawing, or their
 * copies. Parametrelerle gives the transform by its numbers instead
 * (fitParams.ts). Kauçuk levha (fitRubber.ts, docs/adr/0158) takes the used
 * pairs as links the sheet meets exactly; the table shows Helmert's
 * residuals, the local corrections, and Sabit makes a row a fixed point.
 * What is typed stays for the session. The desktop's is
 * `apps/desktop/src/calc/fit/`.
 */
export function openFit(ctx: AppContext): void {
  new FitDialog(ctx);
}

type Scope = 'selection' | 'layer' | 'all';
/** The window's transforms: the core's three least-squares ones and Kauçuk levha. */
type Kind = FitKind | 'rubber';
/** The least-squares transform the table is solved by: Kauçuk levha's residuals are Helmert's. */
const solvedBy = (kind: Kind): FitKind => (kind === 'rubber' ? 'helmert' : kind);

const state = {
  method: 'points' as Method,
  kind: 'helmert' as Kind,
  rows: [{}, {}, {}, {}] as Row[],
  /** Adla eşle's layers: the source points' and the target points'. */
  source: null as string | null,
  target: null as string | null,
  scope: 'selection' as Scope,
  layer: null as string | null,
  copy: false,
  /** Parametrelerle: the base point (a name or Y,X) and the numbers as typed. */
  base: { text: '' },
  scaleY: '1',
  scaleX: '1',
  rotation: '0',
  shiftY: '0',
  shiftX: '0',
};

const TITLE = 'Vektör oturtma';
const KINDS: { value: Kind; label: string }[] = [
  { value: 'helmert', label: 'Helmert' },
  { value: 'affine', label: 'Afin' },
  { value: 'projective', label: 'Projektif' },
  { value: 'rubber', label: 'Kauçuk levha' },
];
/** A kind as a sentence names it. */
const KIND_NAME: Record<Kind, string> = { helmert: 'Helmert', affine: 'afin', projective: 'projektif', rubber: 'kauçuk levha' };
const KIND_HINT: Record<Kind, string> = {
  helmert: 'Benzerlik: öteleme, dönüklük ve tek ölçek; en az 2 çift.',
  affine: "X ve Y'ye ayrı ölçek ve kayma; en az 3 çift, bir doğru üstünde olmayan. Daireler ve yaylar elips olur.",
  projective: "Perspektif; en az 4 çift, üçü bir doğru üstünde olmayan. Eğriler 0,1 mm'lik köşelere açılır.",
  rubber: RUBBER_HINT,
};
const FAILURE: Record<FitFailure['error'], string> = {
  too_few: '',
  coincident: 'Kaynak noktaların hepsi aynı yerde; dönüşüm bulunamaz.',
  collinear: 'Kaynak noktalar bir doğru üstünde; afin dönüşüm bulunamaz. Doğrunun dışında bir çift ekleyin.',
  singular: 'Denklemlerin tek çözümü yok: kaynak noktaların üçü bir doğru üstünde olmamalı.',
};
const COLUMNS = [
  { key: 'use', label: 'Kullan', check: true },
  { key: 'name', label: 'Ad' },
  { key: 'sy', label: 'Kaynak Y', numeric: true },
  { key: 'sx', label: 'Kaynak X', numeric: true },
  { key: 'ty', label: 'Hedef Y', numeric: true },
  { key: 'tx', label: 'Hedef X', numeric: true },
  { key: 'vy', label: 'vY', unit: 'mm', numeric: true },
  { key: 'vx', label: 'vX', unit: 'mm', numeric: true },
  { key: 'v', label: 'v', unit: 'mm', numeric: true },
];
const RESIDUALS = new Set(['vy', 'vx', 'v']);

/** A row's pair, when its four coordinates are numbers. */
function pairOf(row: Row): FitPair | null {
  const n = (k: string) => {
    const v = readNumber(row[k] ?? '');
    return v === null || Number.isNaN(v) ? null : v;
  };
  const [sy, sx, ty, tx] = [n('sy'), n('sx'), n('ty'), n('tx')];
  if (sy === null || sx === null || ty === null || tx === null) return null;
  return { source: { x: sy, y: sx }, target: { x: ty, y: tx }, used: row.use !== '0' };
}

class FitDialog {
  private readonly ctx: AppContext;
  private readonly methodBox = h('div', { class: 'io-row' });
  private readonly kindBox = h('div');
  private readonly paramsBox = h('div', { class: 'calc-section' });
  private readonly matchBox = h('div', { class: 'io-row' });
  private readonly applyBox = h('div', { class: 'io-row' });
  private readonly summaryBox = h('div', { class: 'io-summary' });
  private readonly status = h('span', { class: 'io-status', role: 'status' });
  private readonly apply = h('button', { class: 'btn btn--primary', type: 'button' }, 'Uygula');
  private readonly copy = h('button', { class: 'btn', type: 'button' }, 'Raporu kopyala');
  private readonly grid: Grid;
  private readonly gridBox: HTMLElement;
  private readonly dialog: Dialog;
  /** The solution now, and the rows its pairs came from. */
  private fit: Fit | FitFailure | null = null;
  private rowsOfPairs: number[] = [];
  /** Kauçuk levha now: the used pairs as links and whether they give a sheet. */
  private links: Links | null = null;
  /** Parametrelerle now: the numbers read, or what is wrong with them. */
  private params: Params | HTMLElement[] = [];

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    const model: GridModel = {
      columns: COLUMNS,
      rows: () => state.rows,
      addLabel: 'Çift ekle',
      readonly: (_r, key) => RESIDUALS.has(key),
      mark: (r) => this.mark(r),
      actions: (r) => [
        { icon: 'target', label: `${r + 1}. satırın kaynağını çizimden seç`, tip: 'Çizimdeki yerini gösterin; bir noktaya kenetlenirse adı da gelir.', run: () => this.pick(r, 'source') },
        { icon: 'pin', label: `${r + 1}. satırın hedefini çizimden seç`, tip: 'Ülke sistemindeki noktası çizimdeyse onu gösterin.', run: () => this.pick(r, 'target') },
        ...(state.kind === 'rubber'
          ? [{ icon: 'lock', label: `${r + 1}. satırı sabit yap`, tip: 'Hedefe kaynağı yazar: nokta yerinde kalır, levhayı orada tutar.', run: () => this.fix(r) }]
          : []),
      ],
      canInsertAfter: () => true,
      insertAfter: (r) => state.rows.splice(r + 1, 0, {}),
      canRemove: () => state.rows.length > 1,
      remove: (r) => state.rows.splice(r, 1),
    };
    this.grid = new Grid(model, () => this.solve());
    this.gridBox = h('div', { class: 'calc-section' }, h('h3', { class: 'calc-results__title' }, 'Kontrol noktaları'), this.grid.el);
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.dialog = new Dialog({
      title: TITLE,
      width: 980,
      className: 'dialog--io dialog--calc dialog--fit',
      content: [this.methodBox, this.kindBox, this.matchBox, this.gridBox, this.paramsBox, this.summaryBox, this.applyBox],
      footer: [this.status, this.copy, close, this.apply],
    });
    close.addEventListener('click', () => this.dialog.close());
    this.apply.addEventListener('click', () => this.write());
    this.copy.addEventListener('click', () => this.report());
    this.renderControls();
    this.solve();
  }

  private renderControls(): void {
    const { ctx } = this;
    const layers = ctx.doc.layers;
    const method = segmented<Method>({ label: 'Yöntem', options: METHODS, value: state.method, onChange: (v) => ((state.method = v), this.renderControls(), this.solve()) });
    replaceChildren(this.methodBox, field('Yöntem', method, METHOD_HINT[state.method], 'grow'));
    const points = state.method === 'points';
    for (const box of [this.kindBox, this.matchBox, this.gridBox]) box.hidden = !points;
    this.paramsBox.hidden = points;
    if (!points) this.renderParams();
    // A kind change builds the table again: Kauçuk levha's rows have Sabit.
    const kind = segmented<Kind>({ label: 'Dönüşüm', options: KINDS, value: state.kind, onChange: (v) => ((state.kind = v), this.renderControls(), this.grid.render(), this.solve()) });
    replaceChildren(this.kindBox, h('div', { class: 'io-row' }, field('Dönüşüm', kind, KIND_HINT[state.kind], 'grow')));
    // Adla eşle: the layers holding named points, each with its count.
    const named = new Map<string, number>();
    for (const e of ctx.doc.all()) if (e.kind === 'point' && e.label?.trim()) named.set(e.layerId, (named.get(e.layerId) ?? 0) + 1);
    const choices = layers.leaves().filter((l) => named.has(l.id)).map((l) => ({ value: l.id, label: `${layers.path(l.id)} (${named.get(l.id)})` }));
    if (!state.source || !named.has(state.source)) state.source = choices[0]?.value ?? null;
    if (!state.target || !named.has(state.target)) state.target = choices[1]?.value ?? choices[0]?.value ?? null;
    const none = [{ value: '', label: 'Adlı nokta yok' }];
    const from = select('Kaynak katmanı', choices.length ? choices : none, state.source ?? '', (v) => (state.source = v || null), 'source');
    const to = select('Hedef katmanı', choices.length ? choices : none, state.target ?? '', (v) => (state.target = v || null), 'target');
    const match = h('button', { class: 'btn', type: 'button', disabled: choices.length < 2 }, 'Eşle');
    match.addEventListener('click', () => this.matchByName());
    replaceChildren(
      this.matchBox,
      field('Kaynak noktaları', from, 'Çizimdeki adlı noktalar', 'grow'),
      field('Hedef noktaları', to, 'Oturtulacakları sistemdeki adlı noktalar', 'grow'),
      field('Adla eşle', match, 'Aynı adlılar çift olur'),
    );
    // Uygula: which objects, and whether copies.
    if (!state.layer || !layers.get(state.layer)) state.layer = layers.active.value;
    const counts = { selection: ctx.selection.size, all: ctx.doc.size };
    const scope = segmented<Scope>({
      label: 'Nesneler',
      options: [
        { value: 'selection', label: `Seçili (${counts.selection})`, disabled: !counts.selection },
        { value: 'layer', label: 'Katman' },
        { value: 'all', label: `Tümü (${counts.all})` },
      ],
      value: state.scope === 'selection' && !counts.selection ? 'all' : state.scope,
      onChange: (v) => ((state.scope = v), this.renderControls(), this.updateButton()),
    });
    if (state.scope === 'selection' && !counts.selection) state.scope = 'all';
    const layer =
      state.scope === 'layer'
        ? select(
            'Katman',
            layers.leaves().map((l) => ({ value: l.id, label: layers.path(l.id) })),
            state.layer,
            (v) => ((state.layer = v), this.updateButton()),
            'layer',
          )
        : null;
    const copy = checkField('Kopya', 'Kopya olarak: asıllar yerinde kalır', state.copy, (on) => (state.copy = on), 'copy');
    replaceChildren(this.applyBox, field('Uygulanacak nesneler', scope), layer ? field('Katman', layer, null, 'wide') : null, copy);
  }

  /** The solution again from what is typed: the table's pairs, or the parameters. */
  private solve(): void {
    if (state.method === 'parameters') this.solveParams();
    else this.solvePoints();
    this.updateButton();
  }

  /** Parametrelerle's fields: the base point (shown on the drawing too) and the five numbers. */
  private renderParams(): void {
    const { ctx } = this;
    const picker = { ctx, title: TITLE, close: () => this.dialog.close(), reopen: () => openFit(ctx) };
    const num = (p: (typeof PARAMS)[number]) => {
      const label = paramLabel(ctx, p);
      const f = textField({ label, value: state[p.key], placeholder: String(p.empty), onChange: (v) => ((state[p.key] = v), this.solve()) });
      f.classList.add('calc-num');
      f.dataset.key = p.key;
      return field(label, f);
    };
    replaceChildren(
      this.paramsBox,
      h('div', { class: 'calc-knowns' }, knownField(picker, 'Taban noktası', state.base, () => this.solve(), 'base', 'Ölçek ve dönüklük bu noktanın çevresinde')),
      h('div', { class: 'io-row' }, PARAMS.map(num)),
      h('p', { class: 'io-field__hint' }, PARAMS_HINT),
    );
  }

  /** Parametrelerle's summary: the numbers, where the base lands, and what the objects become; or what is wrong. */
  private solveParams(): void {
    const read = readParams(this.ctx, state);
    this.params = read;
    summary(this.summaryBox, Array.isArray(read) ? read.slice(0, 6) : paramsSummary(this.ctx, read));
  }

  /** The table's pairs solved again; the residuals into their cells, the summary under the table. */
  private solvePoints(): void {
    const kind = state.kind;
    const pairs: FitPair[] = [];
    this.rowsOfPairs = [];
    state.rows.forEach((row, r) => {
      const pair = pairOf(row);
      row.vy = row.vx = row.v = '';
      if (pair) {
        pairs.push(pair);
        this.rowsOfPairs.push(r);
      }
    });
    const used = pairs.filter((p) => p.used).length;
    this.fit = pairs.length ? fitTransform(pairs, solvedBy(kind)) : null;
    const fit = this.fit;
    if (fit && !('error' in fit))
      fit.residuals.forEach(([vx, vy, v], i) => {
        const row = state.rows[this.rowsOfPairs[i]];
        row.vy = fixed(vx * 1000, 1);
        row.vx = fixed(vy * 1000, 1);
        row.v = fixed(v * 1000, 1);
      });
    this.grid.refresh();
    if (kind === 'rubber') {
      this.links = linksOf(pairs);
      summary(this.summaryBox, rubberSummary(this.links, fit, this.corrections()));
      return;
    }
    this.links = null;
    const lines: HTMLElement[] = [];
    if (!fit || 'error' in fit) {
      const need = FIT_NEED[kind];
      if (!fit || fit.error === 'too_few') lines.push(summaryLine('info', `${KIND_NAME[kind][0].toLocaleUpperCase('tr-TR')}${KIND_NAME[kind].slice(1)} için en az ${need} kullanılan çift gerekir; şimdi ${used}. Koordinatları yazın, yapıştırın ya da çizimden seçin.`));
      else lines.push(summaryLine('warn', FAILURE[fit.error]));
    } else {
      const dof = 2 * used - 2 * FIT_NEED[kind];
      lines.push(
        summaryLine(
          'ok',
          fit.m0 === null ? `${used} çift tam geçer; m0 için en az bir fazla çift gerekir (serbestlik 0).` : `m0 = ±${mmText(fit.m0)} (${used} çift, serbestlik ${dof}).`,
        ),
      );
      lines.push(summaryLine('info', this.parameters(fit)));
      const worst = this.worst();
      if (worst !== null && fit.m0 !== null) {
        const row = state.rows[this.rowsOfPairs[worst]];
        lines.push(summaryLine('info', `En büyük artık ${mmText(fit.residuals[worst][2])}: ${row.name?.trim() || `${this.rowsOfPairs[worst] + 1}. satır`}. Kötü bir çifti Kullan'dan çıkarın; çözüm hemen yenilenir.`));
      }
    }
    summary(this.summaryBox, lines);
  }

  /**
   * The parameters in words: Helmert's scale and turn (Kauçuk levha's too), the affine's scales, turn and shear, the
   * projective's numbers. The affine's scales by the surveyor's axes (CLAUDE.md §5): Y is east (the core's x scale), X north.
   */
  private parameters(fit: Fit): string {
    // The Hesap windows are in metres, as typed (docs/adr/0165 §2).
    const format = this.ctx.format.metric();
    if (solvedBy(state.kind) === 'helmert' && fit.scale !== undefined && fit.rotation !== undefined)
      return `Ölçek ${fixed(fit.scale, 8)} (${fixed((fit.scale - 1) * 1e6, 1)} ppm), dönüklük ${format.angle(fit.rotation)}.`;
    if (state.kind === 'affine' && fit.scaleX !== undefined && fit.scaleY !== undefined && fit.rotation !== undefined && fit.shear !== undefined)
      return `Y ölçeği ${fixed(fit.scaleX, 8)}, X ölçeği ${fixed(fit.scaleY, 8)}, dönüklük ${format.angle(fit.rotation)}, kayma ${format.angle(fit.shear)}.`;
    return `Merkezli sayılar (kaynak merkezi ${format.point(fit.from)}): ${fit.params.map((p) => fixed(p, 9)).join('; ')}.`;
  }

  /** The used pair with the largest residual (its index among the pairs). */
  private worst(): number | null {
    const fit = this.fit;
    if (!fit || 'error' in fit) return null;
    let best: number | null = null;
    this.rowsOfPairs.forEach((r, i) => {
      if (state.rows[r].use === '0') return;
      if (best === null || fit.residuals[i][2] > fit.residuals[best][2]) best = i;
    });
    return best;
  }

  /** Kauçuk levha's local corrections: Helmert's largest residual among the used pairs and its row's name, and their mean. */
  private corrections(): Corrections | null {
    const fit = this.fit;
    const worst = this.worst();
    if (!fit || 'error' in fit || worst === null) return null;
    let sum = 0;
    let n = 0;
    this.rowsOfPairs.forEach((r, i) => {
      if (state.rows[r].use === '0') return;
      sum += fit.residuals[i][2];
      n++;
    });
    const r = this.rowsOfPairs[worst];
    return { worst: fit.residuals[worst][2], who: state.rows[r].name?.trim() || `${r + 1}. satır`, mean: sum / n };
  }

  /** Sabit: the row's target is its source, a fixed point; a row without its source says so and keeps its target. */
  private fix(r: number): void {
    const row = state.rows[r];
    if (!row) return;
    if (readNumber(row.sy ?? '') === null || readNumber(row.sx ?? '') === null) {
      this.status.textContent = `${r + 1}. satırın kaynağı eksik; önce kaynağını yazın ya da çizimden seçin.`;
      this.status.dataset.kind = 'error';
      return;
    }
    [row.ty, row.tx] = [row.sy ?? '', row.sx ?? ''];
    this.status.textContent = '';
    this.grid.render();
    this.solve();
  }

  private mark(r: number): string | null {
    if (state.rows[r]?.use === '0') return 'off';
    const worst = this.worst();
    return worst !== null && this.rowsOfPairs[worst] === r && this.fit && !('error' in this.fit) && this.fit.m0 !== null ? 'worst' : null;
  }

  /** The objects Uygula takes: the selection, a layer's or every object, by their persistent ids. */
  private targets(): string[] {
    const { doc, selection } = this.ctx;
    if (state.scope === 'selection') return [...selection.ids.value].flatMap((id) => doc.uidOf(id) ?? []);
    return [...doc.all()].filter((e) => state.scope === 'all' || e.layerId === state.layer).flatMap((e) => doc.uidOf(e.id) ?? []);
  }

  private updateButton(): void {
    const ready = this.plan() !== null;
    this.apply.disabled = !ready || !this.targets().length;
    this.copy.disabled = state.method === 'parameters' && !ready;
  }

  /** What Uygula writes: the transform (`cad.entities.transform`'s centred form) and how it is said. */
  private plan(): { transform: Transform; how: string; m0: string } | null {
    if (state.method === 'parameters') {
      const p = this.params;
      return Array.isArray(p) ? null : { transform: paramsTransform(p), how: 'parametrelerle', m0: '' };
    }
    if (state.kind === 'rubber') {
      const l = this.links;
      return l && !l.error ? { transform: { kind: 'rubbersheet', links: l.links }, how: 'kauçuk levhayla', m0: '' } : null;
    }
    const fit = this.fit;
    if (!fit || 'error' in fit) return null;
    return { transform: this.transform(fit), how: `${KIND_NAME[state.kind]} dönüşümle`, m0: fit.m0 === null ? '' : ` (m0 ±${mmText(fit.m0)})` };
  }

  /** The solution as `cad.entities.transform`'s transform (its centred form). */
  private transform(fit: Fit): Transform {
    const { from, to, params } = fit;
    if (state.kind === 'helmert') return { kind: 'similarity', from, to, a: params[0], b: params[1] };
    if (state.kind === 'affine') return { kind: 'affine', from, to, m: [params[0], params[1], params[2], params[3]] };
    return { kind: 'projective', from, to, h: [params[0], params[1], params[2], params[3], params[4], params[5], params[6], params[7]] };
  }

  private write(): void {
    const plan = this.plan();
    if (this.apply.disabled || !plan) return;
    const { ctx } = this;
    const uids = this.targets();
    const result = entitiesTransform.execute({ doc: ctx.doc }, { uids, transform: plan.transform, ...(state.copy ? { copy: true } : {}) });
    if (result.status !== 'completed') {
      this.status.textContent = 'error' in result ? result.error.message : 'Yazılamadı.';
      this.status.dataset.kind = 'error';
      return;
    }
    const n = state.copy ? result.output.created.length : result.output.changed.length;
    ctx.log.success(`${TITLE}: ${n} nesne${state.copy ? 'nin kopyası' : ''} ${plan.how} oturtuldu${plan.m0}. Ctrl+Z geri alır.`);
    for (const w of result.warnings) ctx.log.warn(w.message);
    if (state.copy) ctx.selection.set(result.output.created.flatMap((uid) => ctx.doc.byUid(uid)?.id ?? []));
    this.dialog.close();
  }

  /** Adla eşle: the points of the same name on the two layers, as pairs; the names on one layer more than once are left out and said. */
  private matchByName(): void {
    const { ctx } = this;
    if (!state.source || !state.target) return;
    const byName = (layer: string) => {
      const out = new Map<string, Vec2[]>();
      for (const e of ctx.doc.all()) {
        const name = e.kind === 'point' ? e.label?.trim() : '';
        if (e.kind === 'point' && name && e.layerId === layer) out.set(name, [...(out.get(name) ?? []), e.p]);
      }
      return out;
    };
    const from = byName(state.source);
    const to = byName(state.target);
    const rows: Row[] = [];
    let twice = 0;
    for (const [name, src] of from) {
      const dst = to.get(name);
      if (!dst) continue;
      if (src.length > 1 || dst.length > 1) {
        twice++;
        continue;
      }
      rows.push({ use: '1', name, sy: String(src[0].x), sx: String(src[0].y), ty: String(dst[0].x), tx: String(dst[0].y) });
    }
    if (!rows.length) {
      ctx.log.warn(`${TITLE}: iki katmanda aynı adlı nokta yok${twice ? `; ${twice} ad bir katmanda birden çok noktada` : ''}.`);
      return;
    }
    state.rows = rows;
    this.grid.render();
    this.solve();
    ctx.log.info(`${TITLE}: ${rows.length} çift adla eşlendi${twice ? `; ${twice} ad bir katmanda birden çok noktada olduğu için alınmadı` : ''}.`);
  }

  /** A row's source or target shown on the drawing: the window closes, the pick tool runs, the window comes back. */
  private pick(r: number, side: 'source' | 'target'): void {
    const { ctx } = this;
    const label = `${r + 1}. çiftin ${side === 'source' ? 'kaynağı' : 'hedefi'}`;
    this.dialog.close();
    ctx.tools.run(
      new PickPointTool(ctx, `${TITLE}: ${label}`, (p) => {
        const row = state.rows[r];
        if (p && row) {
          if (side === 'source') [row.sy, row.sx] = [String(p.x), String(p.y)];
          else [row.ty, row.tx] = [String(p.x), String(p.y)];
          const name = nameAt(ctx, p);
          if (name && !row.name?.trim()) row.name = name;
        }
        queueMicrotask(() => openFit(ctx));
      }),
      `${TITLE}: ${label}`,
    );
  }

  /** The report: the transform, the pairs with their residuals, m0 and the parameters, tab-separated. */
  private report(): void {
    if (state.method === 'parameters') return this.reportParams();
    if (state.kind === 'rubber') return this.reportRubber();
    const fit = this.fit;
    const lines: string[][] = [[TITLE, KIND_NAME[state.kind]], ['Kullan', 'Ad', 'Kaynak Y', 'Kaynak X', 'Hedef Y', 'Hedef X', 'vY (mm)', 'vX (mm)', 'v (mm)']];
    for (const row of state.rows) {
      if (!pairOf(row)) continue;
      lines.push([row.use === '0' ? 'hayır' : 'evet', row.name ?? '', row.sy ?? '', row.sx ?? '', row.ty ?? '', row.tx ?? '', row.vy ?? '', row.vx ?? '', row.v ?? '']);
    }
    if (fit && !('error' in fit)) {
      lines.push(['m0 (mm)', fit.m0 === null ? '—' : fixed(fit.m0 * 1000, 2)]);
      lines.push(['Parametreler', this.parameters(fit)]);
      lines.push(['Kaynak merkezi Y', String(fit.from.x), 'Kaynak merkezi X', String(fit.from.y)]);
      lines.push(['Hedef merkezi Y', String(fit.to.x), 'Hedef merkezi X', String(fit.to.y)]);
      lines.push(['Merkezli sayılar', ...fit.params.map(String)]);
    }
    copyReport(this.ctx, TITLE, lines);
  }

  /** Kauçuk levha's report: the method, the pairs with their local corrections, the links and Helmert's numbers. */
  private reportRubber(): void {
    const rows = state.rows
      .filter((row) => pairOf(row))
      .map((row) => [row.use === '0' ? 'hayır' : 'evet', row.name ?? '', row.sy ?? '', row.sx ?? '', row.ty ?? '', row.tx ?? '', row.vy ?? '', row.vx ?? '', row.v ?? '']);
    const fit = this.fit;
    const parameters = fit && !('error' in fit) ? this.parameters(fit) : null;
    copyReport(this.ctx, TITLE, rubberReport(TITLE, rows, this.links ?? linksOf([]), fit, this.corrections(), parameters));
  }

  /** Parametrelerle's report: the base, the numbers as read and the linear part, tab-separated. */
  private reportParams(): void {
    const p = this.params;
    if (!Array.isArray(p)) copyReport(this.ctx, TITLE, paramsReport(this.ctx, TITLE, p, state));
  }
}
