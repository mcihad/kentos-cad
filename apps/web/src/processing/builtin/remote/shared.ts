import type { OpsResult } from '../../../io/rasterAnalysisProtocol';
import type { Entity, NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import { rasterRun } from '../../features';
import type { Feedback, FeatureSet, RunContext, RunResult, TableOutput, TargetLayer } from '../../types';
import { analyze, rastersFor, specOf } from '../rasterOps/shared';
import { count, stemOf, withTif } from '../surface/shared';

/**
 * What Uzaktan algılama's tools share (docs/adr/0242; the desktop's `builtin/remote/mod.rs`): the runs of the raster
 * core's operation job in the page's analysis worker (processing/rasterHost.ts), a raster result kept beside the first
 * input with its object right above its layer, Bantlara ayır's run a band, Doğruluk analizi's table. The core writes
 * the tables, the summaries' tails and the warnings; here they are passed on.
 */

export const REMOTE = 'remoteSensing';

/** What a remote sensing run says (the WASM module's notes' `remote`). */
export interface RemoteNotes {
  table: TableOutput | null;
  tail: string;
  warnings: string[];
  overall: number | null;
  kappa: number | null;
}

/** The values a remote sensing tool reads. */
export interface RemoteValues {
  input?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

function notesOf(text: string): { remote: RemoteNotes; emptyCells: number } {
  const n = JSON.parse(text) as { remote?: RemoteNotes; emptyCells?: number };
  return { remote: n.remote ?? { table: null, tail: '', warnings: [], overall: null, kappa: null }, emptyCells: n.emptyCells ?? 0 };
}

/** The notes' warnings through the feedback; the tail with a space before it (none when empty). */
function said(n: RemoteNotes, feedback: Feedback): string {
  for (const w of n.warnings) feedback.warn(w);
  return n.tail ? ` ${n.tail}` : '';
}

/** The one raster of a features parameter, with its name, or why not. */
function oneRaster(set: FeatureSet | null | undefined, ctx: RunContext, label: string): { raster: RasterEntity; name: string } | { refused: string } {
  const list = (set?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
  if (!list.length) return { refused: `${label} seçin: bu araç raster ister.` };
  if (list.length > 1) return { refused: `${list.length} raster seçili; ${label} için tek raster seçin.` };
  const run = rasterRun(list, { layerIndex: (id) => ctx.layerIndex(id), byLayer: (id) => ctx.doc.byLayer(id), layerName: (id) => ctx.layerName(id) });
  return { raster: list[0], name: run[0]?.name ?? '' };
}

/** The objects of a features parameter and the text of `field` on each (null where it is missing). */
export function objectsWith(set: FeatureSet | null | undefined, field: string | null | undefined): { shapes: Entity[]; texts: (string | null)[] } {
  const shapes = [...(set?.entities ?? [])];
  const name = field ?? '';
  return { shapes, texts: shapes.map((e) => (name && Object.hasOwn(e.attrs, name) ? e.attrs[name] : null)) };
}

/** The raster object of a kept result on `layer`. */
function rasterObject(r: OpsResult, kept: { asset?: string; file?: string }, name: string, srid: number, layerId: string): NewEntity {
  return {
    kind: 'raster',
    layerId,
    attrs: {},
    affine: [r.grid[0], r.grid[1], r.grid[2], r.grid[3], r.grid[4], r.grid[5]],
    width: r.grid[6],
    height: r.grid[7],
    bands: r.bands,
    sample: r.sample as RasterSample,
    ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
    srid,
    style: JSON.parse(r.style) as RasterStyle,
  } satisfies Omit<RasterEntity, 'id' | 'uid'>;
}

/**
 * Runs a tool whose result is a raster: the input (`one` raster or several) and the second parameter's raster, the
 * objects `shapes`; kept beside the first input (or named), its object right above the first input's layer.
 */
export async function runRemoteRaster(
  v: RemoteValues & Record<string, unknown>,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  [one, second]: readonly [boolean, readonly [string, string] | null],
  shapes: readonly Entity[],
  [suffix, label]: readonly [string, string],
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, one);
  if ('refused' in got) return got;
  const list = [...got.list];
  const names = [...got.names];
  if (second) {
    const s = oneRaster(v[second[0]] as FeatureSet | null | undefined, ctx, second[1]);
    if ('refused' in s) return s;
    list.push(s.raster);
    names.push(s.name);
  }
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes, feedback, label);
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
  const add: NewEntity[] = v.add !== false && v.layer ? [rasterObject(r, kept, name, first.srid, v.layer.id)] : [];
  const summary = `${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.${said(notes.remote, feedback)}`;
  const outputs: Record<string, unknown> = { file: name };
  if (notes.remote.table) outputs.table = notes.remote.table;
  return { changes: add.length ? { add } : undefined, outputs, summary, above: first.layerId };
}

/**
 * Bantlara ayır (§4): each band its own GeoTIFF, `…-b1.tif`, `…-b2.tif` …, after the raster's name (or the name
 * asked); the objects in the bands' order on a new layer right above the raster's.
 */
export async function runSplit(v: RemoteValues, ctx: RunContext, feedback: Feedback): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const first = list[0];
  const asked = v.output?.trim() ?? '';
  const stem = asked ? `${withTif(asked).replace(/\.tif$/i, '')}-b` : `${stemOf(first)}-b`;
  const label = 'Bantlar ayrılıyor';
  const add: NewEntity[] = [];
  const rows: string[][] = [];
  let size: [number, number] = [0, 0];
  // The file's bands; an alpha band (the last) the core refuses, and the bands end there.
  for (let b = 1; b <= Math.max(1, first.bands); b++) {
    const ran = await analyze(list, specOf({ kind: 'band', band: b }, list, names, ctx), [], feedback, label);
    if (!ran.ok) {
      const end = ran.end;
      if (b > 1 && 'refused' in end && typeof end.refused === 'string' && end.refused.includes('bant yok')) break;
      return end;
    }
    const r = ran.result;
    if (!r.bytes) return { refused: 'Çözümleme raster vermedi.' };
    const name = `${stem}${b}.tif`;
    size = [r.grid[6], r.grid[7]];
    const kept = await ran.host.keep(r.bytes, name, size[0], size[1]);
    if (kept.note) feedback.info(kept.note);
    feedback.progress(b / Math.max(1, first.bands), label);
    if (v.add !== false && v.layer) add.push(rasterObject(r, kept, name, first.srid, v.layer.id));
    rows.push([String(b), name]);
  }
  const shown = rows.length === 1 ? `“${rows[0][1]}”` : rows.length ? `“${rows[0][1]}” … “${rows[rows.length - 1][1]}”` : '';
  const summary = `${count(rows.length)} bant ayrı rasterlere yazıldı: ${shown} (${count(size[0])} × ${count(size[1])} hücre).`;
  return {
    changes: add.length ? { add } : undefined,
    outputs: { table: { columns: ['Bant', 'Dosya'], rows } },
    summary,
    above: first.layerId,
  };
}

/** Doğruluk analizi (§8): the matrix's table, the overall accuracy and kappa; nothing written. */
export async function runAccuracy(
  v: { input?: FeatureSet | null; band?: number | null; reference?: FeatureSet | null; referenceField?: string | null },
  ctx: RunContext,
  feedback: Feedback,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const { shapes, texts } = objectsWith(v.reference, v.referenceField);
  const tool = { kind: 'accuracy', band: v.band ?? 1, reference: texts };
  const label = 'Doğruluk hesaplanıyor';
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes, feedback, label);
  if (!ran.ok) return ran.end;
  feedback.progress(1, label);
  const n = notesOf(ran.result.notes).remote;
  for (const w of n.warnings) feedback.warn(w);
  const outputs: Record<string, unknown> = { overall: n.overall, kappa: n.kappa };
  if (n.table) outputs.table = n.table;
  return { outputs, summary: n.tail };
}
