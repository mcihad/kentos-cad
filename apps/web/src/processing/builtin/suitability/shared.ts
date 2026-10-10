import { fixed } from '../../../core/displayNumber';
import type { NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import type { Feedback, FeatureSet, RunContext, RunResult, TableOutput, TargetLayer } from '../../types';
import { trimmed } from '../hydrology/shared';
import { analyze, rastersFor, specOf } from '../rasterOps/shared';
import { count, stemOf, withTif } from '../surface/shared';

/**
 * What Uygunluk analizi's tools share (docs/adr/0237; the desktop's `builtin/suitability/mod.rs`): the runs of the
 * raster core's operation job in the page's analysis worker (processing/rasterHost.ts), a raster result kept beside the
 * first input with its object right above its layer, İkili karşılaştırma's weights and ROC's curve as tables, the notes
 * for the summaries.
 */

export const SUITABILITY = 'suitability';

/** Above this the comparisons are inconsistent. */
const CONSISTENT = 0.1;

/** İkili karşılaştırma's weights and consistency (the WASM module's notes' `suit.pairwise`). */
export interface PairwiseNotes {
  weights: number[];
  lambda: number;
  ci: number;
  ri: number;
  cr: number;
}

/** What a suitability run met (the WASM module's notes' `suit`). */
export interface SuitNotes {
  unmatched: number;
  outside: number;
  restricted: number;
  invalid: number;
  pairwise: PairwiseNotes | null;
}

/** ROC ile doğrulama's figures (the WASM module's `roc`). */
export interface RocFigures {
  presence: number;
  background: number;
  allCells: boolean;
  auc: number;
  rows: [number, number, number][];
  best: number | null;
  skipped: number;
  outside: number;
  both: number;
}

/** The values a suitability tool reads. */
export interface SuitValues {
  input?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

function notesOf(text: string): { suit: SuitNotes; emptyCells: number } {
  const n = JSON.parse(text) as { suit?: SuitNotes; emptyCells?: number };
  return { suit: n.suit ?? { unmatched: 0, outside: 0, restricted: 0, invalid: 0, pairwise: null }, emptyCells: n.emptyCells ?? 0 };
}

/** “n raster birleşti.” */
export const joined = (n: number): string => ` ${count(n)} raster birleşti.`;

/** Bulanık çakıştırma's note: the cells with a membership outside 0–1. */
export function sayInvalid(n: SuitNotes, feedback: Feedback): void {
  if (n.invalid > 0) feedback.warn(`${count(n.invalid)} hücrede üyelik 0–1 aralığının dışında; o hücreler değersiz bırakıldı.`);
}

/** Ağırlıklı çakıştırma's notes: the cells no rule held, off the scale, restricted. */
export function sayOverlay(n: SuitNotes, feedback: Feedback): string {
  if (n.unmatched > 0) feedback.warn(`Sınıf tablosunun hiçbir kuralının tutmadığı ${count(n.unmatched)} hücre değersiz bırakıldı.`);
  if (n.outside > 0) feedback.warn(`Ölçeğin dışında (ya da tam sayı olmayan) değerli ${count(n.outside)} hücre değersiz bırakıldı.`);
  return n.restricted > 0 ? ` Kısıtlı ${count(n.restricted)} hücre.` : '';
}

/** İkili karşılaştırma's figures as the summary writes them. */
export function consistency(p: PairwiseNotes): string {
  return `λ ${fixed(p.lambda, 4)}; CI ${fixed(p.ci, 4)}; RI ${fixed(p.ri, 2)}; CR ${fixed(p.cr, 4)} (${p.cr > CONSISTENT ? 'tutarsız' : 'tutarlı'}).`;
}

/** The weights' table: Ölçüt, Ağırlık, Yüzde. */
export function weightsTable(names: readonly string[], p: PairwiseNotes): TableOutput {
  return { columns: ['Ölçüt', 'Ağırlık', 'Yüzde (%)'], rows: names.map((n, k) => [n, fixed(p.weights[k], 6), fixed(100 * p.weights[k], 2)]) };
}

/** The warning of inconsistent comparisons. */
export function warnInconsistent(p: PairwiseNotes, feedback: Feedback): void {
  if (p.cr > CONSISTENT) feedback.warn(`Karşılaştırmalar tutarsız (CR ${fixed(p.cr, 2)} > 0.10); en çelişkili çiftleri yeniden gözden geçirin.`);
}

/**
 * Runs a tool whose result is a raster (§2): kept beside the first input (or named), its object right above the first
 * input's layer; `said` writes the summary's tail from the notes and the inputs' count, the warnings through the feedback.
 */
export async function runSuitRaster(
  v: SuitValues,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  [suffix, label]: readonly [string, string],
  one: boolean,
  said: (n: SuitNotes, inputs: number, feedback: Feedback) => string,
): Promise<RunResult & { notes?: SuitNotes }> {
  const got = rastersFor(v.input, ctx, one);
  if ('refused' in got) return got;
  const { list, names } = got;
  const ran = await analyze(list, specOf(tool, list, names, ctx), [], feedback, label);
  if (!ran.ok) return ran.end;
  const r = ran.result;
  if (!r.bytes) return { refused: 'Çözümleme raster vermedi.' };
  const first = list[0];
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOf(first)}${suffix}.tif`;
  const [width, height] = [r.grid[6], r.grid[7]];
  const kept = await ran.host.keep(r.bytes, name, width, height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const notes = notesOf(r.notes);
  if (notes.emptyCells === width * height) feedback.warn('Sonucun hiçbir hücresinde değer yok.');
  const add: NewEntity[] = [];
  if (v.add !== false && v.layer) {
    add.push({
      kind: 'raster',
      layerId: v.layer.id,
      attrs: {},
      affine: [r.grid[0], r.grid[1], r.grid[2], r.grid[3], r.grid[4], r.grid[5]],
      width,
      height,
      bands: r.bands,
      sample: r.sample as RasterSample,
      ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
      srid: first.srid,
      style: JSON.parse(r.style) as RasterStyle,
    } satisfies Omit<RasterEntity, 'id' | 'uid'>);
  }
  const summary = `${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.${said(notes.suit, list.length, feedback)}`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary, above: first.layerId, notes: notes.suit };
}

/** Runs İkili karşılaştırma: the weights' table and, when asked, the weighted sum's raster. */
export async function runPairwise(
  v: SuitValues & { comparisons?: readonly (readonly [string, string, number])[] | null; band?: number | null; write?: boolean | null; sample?: string | null },
  ctx: RunContext,
  feedback: Feedback,
): Promise<RunResult> {
  const write = v.write !== false;
  const tool = { kind: 'pairwise', band: v.band ?? 1, pairs: v.comparisons ?? [], write, sample: v.sample ?? 'f32' };
  if (write) {
    let table: TableOutput | null = null;
    const got = rastersFor(v.input, ctx, false);
    const names = 'refused' in got ? [] : got.names;
    const result = await runSuitRaster(v, ctx, feedback, tool, ['-ahp', 'Ağırlıklı toplam hesaplanıyor'], false, (n, _k, fb) => {
      if (!n.pairwise) return '';
      warnInconsistent(n.pairwise, fb);
      table = weightsTable(names, n.pairwise);
      return ` Ağırlıklar tabloda; ${consistency(n.pairwise)}`;
    });
    const { notes: _notes, ...rest } = result;
    return table ? { ...rest, outputs: { ...rest.outputs, table } } : rest;
  }
  const got = rastersFor(v.input, ctx, false);
  if ('refused' in got) return got;
  const { list, names } = got;
  const label = 'Ağırlıklar hesaplanıyor';
  const ran = await analyze(list, specOf(tool, list, names, ctx), [], feedback, label);
  if (!ran.ok) return ran.end;
  feedback.progress(1, label);
  const p = notesOf(ran.result.notes).suit.pairwise;
  if (!p) return { refused: 'Ağırlıklar hesaplanamadı.' };
  warnInconsistent(p, feedback);
  return { outputs: { table: weightsTable(names, p) }, summary: `${count(names.length)} ölçütün ağırlıkları tabloda; ${consistency(p)}` };
}

/** The curve's table (§8). */
export function rocTable(c: RocFigures): TableOutput {
  const [fpLabel, fpShare] = c.allCells ? ['Hücre', 'Alan oranı (%)'] : ['Yanlış pozitif', 'Yanlış pozitif oranı (%)'];
  return {
    columns: ['Eşik', 'Doğru pozitif', 'Doğru pozitif oranı (%)', fpLabel, fpShare],
    rows: c.rows.map(([t, tp, fp]) => [trimmed(t, 6), String(tp), fixed((100 * tp) / c.presence, 2), String(fp), fixed((100 * fp) / c.background, 2)]),
  };
}

/** ROC's summary and warnings (§8). */
export function rocSummary(c: RocFigures, feedback: Feedback): string {
  if (c.skipped > 0) feedback.warn(`Değersiz hücreye düşen ${count(c.skipped)} örnek atlandı.`);
  if (c.outside > 0) feedback.warn(`Rasterin dışında kalan ${count(c.outside)} nokta atlandı.`);
  if (c.both > 0) feedback.warn(`${count(c.both)} hücre hem varlık hem yokluk; ikisinde de sayıldı.`);
  let s = `AUC ${fixed(c.auc, 4)} (${count(c.presence)} varlık hücresi, ${count(c.background)} ${c.allCells ? 'değerli hücre' : 'yokluk hücresi'}).`;
  const b = c.best !== null ? c.rows[c.best] : undefined;
  if (b) {
    const share = c.allCells ? 'alan' : 'yanlış pozitif';
    s += ` En iyi eşik ${trimmed(b[0], 6)} (doğru pozitif %${fixed((100 * b[1]) / c.presence, 1)}, ${share} %${fixed((100 * b[2]) / c.background, 1)}).`;
  }
  return s;
}

/** Runs ROC ile doğrulama: the curve's table. */
export async function runRoc(
  v: { input?: FeatureSet | null; band?: number | null; presence?: FeatureSet | null; background?: string | null; absence?: FeatureSet | null; higher?: boolean | null },
  ctx: RunContext,
  feedback: Feedback,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const absence = v.background === 'absence';
  const presence = v.presence?.entities ?? [];
  const shapes = absence ? [...presence, ...(v.absence?.entities ?? [])] : [...presence];
  const tool = { kind: 'roc', band: v.band ?? 1, first: presence.length, absence, higher: v.higher !== false };
  const label = 'Doğrulanıyor';
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes, feedback, label);
  if (!ran.ok) return ran.end;
  feedback.progress(1, label);
  if (!ran.result.roc) return { refused: 'Doğrulama sonuç vermedi.' };
  const c = JSON.parse(ran.result.roc) as RocFigures;
  return { outputs: { table: rocTable(c), auc: c.auc }, summary: rocSummary(c, feedback) };
}
