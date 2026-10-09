import { fixed } from '../../../core/displayNumber';
import type { AnalysisFeatures } from '../../../io/rasterAnalysisProtocol';
import type { Entity, NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import type { Feedback, FeatureSet, RunContext, RunResult, TargetLayer } from '../../types';
import { outputStyle } from '../geometry/shared';
import { objectsOf } from '../rasterVector/shared';
import { analyze, rastersFor, specOf } from '../rasterOps/shared';
import { count, stemOf, withTif } from '../surface/shared';

/**
 * What Hidroloji's tools share (docs/adr/0235; the desktop's `builtin/hydrology/mod.rs`): the run of the raster core's
 * operation job in the page's analysis worker (processing/rasterHost.ts), a raster result kept beside the DEM with its
 * object right above the DEM's layer, or the objects with their numbers as attributes; the notes for the summaries.
 */

export const HYDROLOGY = 'hydrology';

/** What a hydrology run met (the WASM module's notes' `hydro`). */
export interface HydroNotes {
  cells: number;
  empty: number;
  skipped: number[];
  emptyPoints: number[];
  dropped: number;
  threshold: number;
  links: number;
  most: number;
}

/** A number at most `decimals` decimals, trailing zeros dropped. */
export function trimmed(v: number, decimals: number): string {
  const t = fixed(v, decimals);
  return t.includes('.') ? t.replace(/0+$/, '').replace(/\.$/, '') : t;
}

/** An attribute's text: counts and numbers whole, areas and lengths to the millimetre, slopes to five decimals. */
export function attrText(field: string, v: number): string {
  if (field === 'Eğim') return trimmed(v, 5);
  if (field === 'Alan' || field === 'Uzunluk' || field === 'Düşü' || field === 'Km' || field === 'Uzaklık') return trimmed(v, 3);
  return trimmed(v, 0);
}

/** Feature `k`'s attributes: its named numbers' texts. */
export function attrsOf(f: AnalysisFeatures, k: number): Record<string, string> {
  const out: Record<string, string> = {};
  const stride = f.fields.length;
  f.fields.forEach((name, x) => (out[name] = attrText(name, f.numbers[k * stride + x])));
  return out;
}

/** Çıktı katmanı of an object tool: a new one goes right above the DEM's. */
export const objectsLayer = <N extends string>(name: N, color: string) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName: name },
    newLayerStyle: outputStyle(color),
    above: 'input',
    description: 'Bu adda katman yoksa oluşturulur; yükseklik modelinin katmanının hemen üstünde.',
  }) as const;

/** The values a hydrology tool reads. */
export interface HydroValues {
  input?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

function notesOf(text: string): HydroNotes {
  const n = JSON.parse(text) as { hydro?: HydroNotes };
  return n.hydro ?? { cells: 0, empty: 0, skipped: [], emptyPoints: [], dropped: 0, threshold: 0, links: 0, most: 0 };
}

/** Runs a tool whose result is a raster: kept beside the DEM (or named), its object, the summary `said` writes. */
export async function runHydroRaster(
  v: HydroValues,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  suffix: string,
  label: string,
  said: (n: HydroNotes) => string,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
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
  const summary = `${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.${said(notesOf(r.notes))}`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary, above: first.layerId };
}

/**
 * Runs a tool whose result is objects (the points or routes `shapes` handed over): on a new layer right above the DEM's,
 * with their numbers as attributes.
 */
export async function runHydroObjects(
  v: HydroValues,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  shapes: readonly Entity[],
  label: string,
  said: (f: AnalysisFeatures, n: HydroNotes, feedback: Feedback) => string,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes, feedback, label);
  if (!ran.ok) return ran.end;
  const f = ran.result.features;
  if (!f) return { refused: 'Çözümleme nesne vermedi.' };
  feedback.progress(1, label);
  const add = objectsOf(f, v.layer?.id ?? '', (k) => attrsOf(f, k));
  const summary = said(f, notesOf(ran.result.notes), feedback);
  return { changes: add.length && v.layer ? { add } : undefined, outputs: { count: add.length }, summary, above: list[0].layerId };
}

/** The points no cell took, said. */
export function warnSkipped(n: HydroNotes, feedback: Feedback): void {
  if (n.skipped.length) feedback.warn(`${count(n.skipped.length)} nokta rasterin dışında ya da değersiz hücrede kaldığı için atlandı.`);
}
