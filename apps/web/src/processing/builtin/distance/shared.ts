import { STOPPED, type PointResult } from '../../../io/rasterAnalysisProtocol';
import type { Entity, NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import { rasterRunHost } from '../../rasterHost';
import type { Feedback, FeatureSet, RunContext, RunResult, TargetLayer } from '../../types';
import { trimmed } from '../hydrology/shared';
import { stemOfName } from '../interpolation/shared';
import { analyze, rastersFor, specOf } from '../rasterOps/shared';
import { objectsOf } from '../rasterVector/shared';
import { count, stemOf, withTif } from '../surface/shared';

/**
 * What Uzaklık ve maliyet's tools share (docs/adr/0236; the desktop's `builtin/distance/mod.rs`): the runs of the raster
 * core's operation job over the cost raster (and the surface) in the page's analysis worker (processing/rasterHost.ts),
 * a raster result kept beside it with its object right above its layer, the paths as polylines, and Uzaklık yüzeyi from
 * objects as the point job; the notes for the summaries.
 */

export const DISTANCE = 'distance';

/** What a distance run met (the WASM module's notes' `distance`; least and most null when no cell has a value). */
export interface DistanceNotes {
  sources: number;
  outside: number;
  unreached: number[];
  least: number | null;
  most: number | null;
  cells: number;
}

/** An attribute's text: the numbers whole, costs, lengths and grades to three decimals. */
export function attrText(field: string, v: number): string {
  return field === 'Yol' || field === 'Kaynak' ? trimmed(v, 0) : trimmed(v, 3);
}

/** The values the distance tools read. */
export interface DistanceValues {
  input?: FeatureSet | null;
  sources?: FeatureSet | null;
  targets?: FeatureSet | null;
  useSurface?: boolean | null;
  surface?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

function notesOf(text: string): DistanceNotes {
  const n = JSON.parse(text) as { distance?: DistanceNotes };
  return n.distance ?? { sources: 0, outside: 0, unreached: [], least: null, most: null, cells: 0 };
}

/** The cost raster (the one of the input) and, while Yükseklik modeliyle is on, the surface: the run's rasters in that order. */
function rastersOf(v: DistanceValues, ctx: RunContext): { list: RasterEntity[]; names: string[] } | { refused: string } {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got || v.useSurface !== true) return got;
  const surface = (v.surface?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
  if (!surface.length) return { refused: 'Yükseklik modelini seçin: Yükseklik modeliyle açık.' };
  if (surface.length > 1) return { refused: `${surface.length} raster seçili; yükseklik modeli için tek raster seçin.` };
  const named = rastersFor(v.surface, ctx, true);
  if ('refused' in named) return named;
  return { list: [...got.list, surface[0]], names: [...got.names, ...named.names] };
}

/** Says the objects that fell on no cell. */
function warnOutside(n: number, what: string, feedback: Feedback): void {
  if (n > 0) feedback.warn(`${count(n)} ${what} rasterin hücrelerine düşmediği için alınmadı.`);
}

/**
 * Runs a tool whose result is a raster over the input raster (and the surface): kept beside it (or named), its object
 * right above its layer, the summary `said` writes from the notes and the grid's cells.
 */
export async function runDistanceRaster(
  v: DistanceValues,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  shapes: readonly Entity[],
  what: string,
  suffix: string,
  label: string,
  said: (n: DistanceNotes, cells: number) => string,
): Promise<RunResult> {
  const got = rastersOf(v, ctx);
  if ('refused' in got) return got;
  const { list, names } = got;
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
  warnOutside(notes.outside, what, feedback);
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
  const summary = `${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.${said(notes, width * height)}`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary, above: first.layerId };
}

/** En düşük maliyetli yol: the paths on a new layer right above the cost raster's, their numbers as attributes. */
export async function runDistancePaths(v: DistanceValues, ctx: RunContext, feedback: Feedback, tool: Record<string, unknown>, shapes: readonly Entity[]): Promise<RunResult> {
  const label = 'En düşük maliyetli yollar bulunuyor';
  const got = rastersOf(v, ctx);
  if ('refused' in got) return got;
  const { list, names } = got;
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes, feedback, label);
  if (!ran.ok) return ran.end;
  const f = ran.result.features;
  if (!f) return { refused: 'Çözümleme nesne vermedi.' };
  feedback.progress(1, label);
  const d = notesOf(ran.result.notes);
  warnOutside(d.outside, 'başlangıç nesnesi', feedback);
  if (d.unreached.length)
    feedback.warn(`${count(d.unreached.length)} varışa yol yok (erişilemiyor ya da rasterin değerli hücrelerine düşmüyor): ${d.unreached.join(', ')}.`);
  const stride = f.fields.length;
  const add = objectsOf(f, v.layer?.id ?? '', (k) => {
    const out: Record<string, string> = {};
    f.fields.forEach((field, x) => (out[field] = attrText(field, f.numbers[k * stride + x])));
    return out;
  });
  const summary = f.values.length ? `${count(f.values.length)} yol yazıldı.` : 'Yol bulunamadı.';
  return { changes: add.length && v.layer ? { add } : undefined, outputs: { count: add.length }, summary, above: list[0].layerId };
}

const NO_HOST = 'Bu araç sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** The values Uzaklık yüzeyi from objects runs with. */
interface ObjectValues {
  sources?: FeatureSet | null;
  cellSize?: number | null;
  extent?: string | null;
  grid?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

/**
 * Uzaklık yüzeyi from objects: the point job over the sources, on their box with its margin or a raster's grid; the
 * raster right below the sources' layer.
 */
export async function runDistanceFromObjects(v: ObjectValues, ctx: RunContext, feedback: Feedback, tool: Record<string, unknown>): Promise<RunResult> {
  const host = rasterRunHost();
  if (!host) return { refused: NO_HOST };
  const entities = v.sources?.entities ?? [];
  if (!entities.length) return { refused: 'Kaynakları seçin: nokta, çizgi ya da alan.' };
  const objects = JSON.stringify(entities, (k, x: unknown) => (k === 'attrs' || k === 'label' || k === 'uid' ? undefined : x));
  let grid: { affine: number[]; width: number; height: number } | null = null;
  if (v.extent === 'raster') {
    const rasters = (v.grid?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
    if (rasters.length !== 1) return { refused: rasters.length ? `${rasters.length} raster seçili; ızgara için tek raster seçin.` : 'Izgara rasterini seçin: Kapsam rasterin ızgarası.' };
    grid = { affine: [...rasters[0].affine], width: rasters[0].width, height: rasters[0].height };
  }
  const srid = ctx.project.srid;
  const spec = JSON.stringify({ tool, cell: v.cellSize ?? 0, grid, epsg: srid > 0 ? srid : null, system: ctx.crs?.system ?? null });
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOfName(ctx.layerName(entities[0].layerId), 'uzaklik')}-uzaklik.tif`;
  const label = 'Uzaklık yüzeyi hesaplanıyor';
  let r: PointResult;
  try {
    const watch = {
      progress: (s: number) => feedback.progress(0.97 * s, label),
      get canceled() {
        return feedback.canceled;
      },
    };
    r = await host.analyzePoints(objects, 'null', spec, true, watch);
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return why === STOPPED ? {} : { refused: why };
  }
  const [width, height] = [r.grid[6], r.grid[7]];
  const kept = await host.keep(r.bytes, name, width, height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const notes = JSON.parse(r.notes) as { taken: number; outside: number; empty: number };
  warnOutside(notes.outside, 'kaynak', feedback);
  const add: NewEntity[] = [];
  if (v.add !== false && v.layer) {
    add.push({
      kind: 'raster',
      layerId: v.layer.id,
      attrs: {},
      affine: r.grid.slice(0, 6) as RasterEntity['affine'],
      width,
      height,
      bands: r.bands,
      sample: r.sample as RasterSample,
      ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
      srid,
      style: JSON.parse(r.styles[0]) as RasterStyle,
    } as NewEntity);
  }
  let summary = `${count(notes.taken)} kaynaktan ${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.`;
  if (notes.empty > 0) summary += ` En büyük uzaklığın ötesinde ${count(notes.empty)} hücre değersiz.`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary };
}

/** A number of the notes as the summaries write it: three decimals at most (none: empty). */
export const least3 = (v: number | null): string => (v === null ? '' : trimmed(v, 3));
