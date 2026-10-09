import { fixed } from '../../../core/displayNumber';
import { STOPPED, type AnalysisFeatures, type PointResult } from '../../../io/rasterAnalysisProtocol';
import type { Entity, NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { contourElevations } from '../../../model/ops/contourElevations';
import { assignElevations, elevatedPaths } from '../../../product/elevation';
import { rasterRunHost } from '../../rasterHost';
import type { Feedback, FeatureSet, RunContext, RunResult, TargetLayer } from '../../types';
import { outputStyle } from '../geometry/shared';
import { stemOfName } from '../interpolation/shared';
import { analyze, rastersFor, specOf } from '../rasterOps/shared';
import { count, withTif } from '../surface/shared';

/**
 * What Raster ve vektör and Taranmış harita's tools share (docs/adr/0234; the desktop's `builtin/raster_vector/mod.rs`):
 * the output layer, the run of a vectorizing job in the page's analysis worker (processing/rasterHost.ts) and its
 * features as objects, Rasterleştir's point job and raster object, Eğrilere kot ver's elevations, the summaries.
 */

export const RASTER_VECTOR = 'rasterVector';
export const SCANNED = 'scannedMap';

/** The kinds Rasterleştir burns: areas (by their cells' centres), lines (every cell they touch) and points. */
export const BURN_KINDS = ['polygon', 'circle', 'ellipse', 'spline', 'hatch', 'line', 'polyline', 'arc', 'point'] as const;

/** Çıktı katmanı of a vectorizing tool: a new one goes right above the raster's (under it the raster would hide it). */
export const vectorLayer = <N extends string>(name: N, color: string) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName: name },
    newLayerStyle: outputStyle(color),
    above: 'input',
    description: 'Bu adda katman yoksa oluşturulur; rasterin katmanının hemen üstünde.',
  }) as const;

/** An elevation or a value as a summary writes it: at most three decimals, trailing zeros dropped. */
export function short(v: number): string {
  const t = fixed(v, 3);
  return t.includes('.') ? t.replace(/0+$/, '').replace(/\.$/, '') : t;
}

/** The features as objects on `layerId`: areas with their holes, polylines, points (with `z` when given). */
export function objectsOf(f: AnalysisFeatures, layerId: string, attrs: (k: number) => Record<string, string>, z?: (k: number) => number | undefined): NewEntity[] {
  const out: NewEntity[] = [];
  let at = 0;
  let ring = 0;
  const path = (n: number): Vec2[] => {
    const pts: Vec2[] = [];
    for (let q = 0; q < n; q++) pts.push({ x: f.xy[2 * (at + q)], y: f.xy[2 * (at + q) + 1] });
    at += n;
    return pts;
  };
  for (let k = 0; k < f.values.length; k++) {
    const zk = z?.(k);
    if (f.kind === 'points') {
      const [p] = path(1);
      out.push({ kind: 'point', layerId, attrs: attrs(k), p, ...(zk !== undefined ? { z: zk } : {}) });
    } else if (f.kind === 'lines') {
      const pts = path(f.sizes[k]);
      out.push({ kind: 'polyline', layerId, attrs: attrs(k), pts, ...(zk !== undefined ? { zs: pts.map(() => zk) } : {}) });
    } else {
      const rings: Vec2[][] = [];
      for (let r = 0; r < f.rings[k]; r++) rings.push(path(f.sizes[ring + r]));
      ring += f.rings[k];
      out.push({ kind: 'polygon', layerId, attrs: attrs(k), pts: rings[0], ...(rings.length > 1 ? { holes: rings.slice(1).map((pts) => ({ pts })) } : {}) });
    }
  }
  return out;
}

/** What a vectorizing tool reads of its values. */
export interface VectorValues {
  input?: FeatureSet | null;
  layer?: TargetLayer | null;
}

/** Runs a vectorizing job over the one raster of the input; `made` turns its features into objects and a summary. */
export async function runVector(
  v: VectorValues,
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  label: string,
  made: (f: AnalysisFeatures, layerId: string) => { add: NewEntity[]; summary: string },
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const ran = await analyze(list, specOf(tool, list, names, ctx), [], feedback, label);
  if (!ran.ok) return ran.end;
  const f = ran.result.features;
  if (!f) return { refused: 'Çözümleme nesne vermedi.' };
  feedback.progress(1, label);
  const { add, summary } = made(f, v.layer?.id ?? '');
  return { changes: add.length && v.layer ? { add } : undefined, outputs: { count: add.length }, summary, above: list[0].layerId };
}

const NO_HOST = 'Bu araç sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** The values Rasterleştir runs with. */
interface BurnValues {
  input?: FeatureSet | null;
  valueFrom?: string | null;
  value?: number | null;
  field?: string | null;
  overlap?: string | null;
  sample?: string | null;
  cellSize?: number | null;
  extent?: string | null;
  grid?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

/** Rasterleştir (docs/adr/0234 §3): the objects burnt onto the grid, the GeoTIFF kept, its object right below the input's layer. */
export async function runRasterize(v: BurnValues, ctx: RunContext, feedback: Feedback): Promise<RunResult> {
  const host = rasterRunHost();
  if (!host) return { refused: NO_HOST };
  const entities = v.input?.entities ?? [];
  if (!entities.length) return { refused: 'Girdi boş: nesne seçin.' };
  const field = (v.field ?? '').trim();
  const byField = v.valueFrom === 'field';
  if (byField && !field) return { refused: 'Değer alanını seçin: Değer alandan okunur.' };
  const values = byField ? JSON.stringify(entities.map((e) => (Object.hasOwn(e.attrs, field) ? e.attrs[field] : null))) : 'null';
  const objects = JSON.stringify(entities, (k, x: unknown) => (k === 'attrs' || k === 'label' || k === 'uid' ? undefined : x));
  let grid: { affine: number[]; width: number; height: number } | null = null;
  if (v.extent === 'raster') {
    const rasters = (v.grid?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
    if (rasters.length !== 1) return { refused: rasters.length ? `${rasters.length} raster seçili; ızgara için tek raster seçin.` : 'Izgara rasterini seçin: Kapsam rasterin ızgarası.' };
    grid = { affine: [...rasters[0].affine], width: rasters[0].width, height: rasters[0].height };
  }
  const srid = ctx.project.srid;
  const spec = JSON.stringify({
    tool: { kind: 'rasterize', value: v.value ?? 1, overlap: v.overlap ?? 'last', sample: v.sample ?? 'f32' },
    cell: v.cellSize ?? 0,
    grid,
    epsg: srid > 0 ? srid : null,
    system: ctx.crs?.system ?? null,
  });
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOfName(ctx.layerName(entities[0].layerId))}-raster.tif`;
  const label = 'Rasterleştiriliyor';
  let r: PointResult;
  try {
    const watch = {
      progress: (s: number) => feedback.progress(0.97 * s, label),
      get canceled() {
        return feedback.canceled;
      },
    };
    r = await host.analyzePoints(objects, values, spec, true, watch);
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return why === STOPPED ? {} : { refused: why };
  }
  const [width, height] = [r.grid[6], r.grid[7]];
  const kept = await host.keep(r.bytes, name, width, height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const notes = JSON.parse(r.notes) as { taken: number; unread: number; outside: number; empty: number };
  if (notes.unread > 0) feedback.warn(`${count(notes.unread)} nesnenin değeri sayı olarak okunamadığı için alınmadı.`);
  if (notes.outside > 0) feedback.warn(`${count(notes.outside)} nesne ızgaranın dışında kaldı.`);
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
  let summary = `${count(notes.taken)} nesneden ${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.`;
  if (notes.empty > 0) summary += ` Değersiz ${count(notes.empty)} hücre.`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary };
}

/** The values Eğrilere kot ver runs with. */
interface ElevationValues {
  curves?: FeatureSet | null;
  start?: Vec2 | null;
  end?: Vec2 | null;
  first?: number | null;
  step?: number | null;
}

/** Eğrilere kot ver (docs/adr/0234 §9): each curve the cut crosses its elevation at every vertex, in the run's one step. */
export function runContourElevations(v: ElevationValues, _ctx: RunContext, feedback: Feedback): RunResult {
  const curves = v.curves?.entities ?? [];
  if (!curves.length) return { refused: 'Eğrileri seçin: çizgi, çoklu çizgi ya da kapalı alan.' };
  const { start, end } = v;
  if (!start || !end) return { refused: 'Kesen çizginin başlangıç ve bitiş noktalarını seçin.' };
  if (start.x === end.x && start.y === end.y) return { refused: 'Başlangıç ve bitiş aynı nokta: kesen çizgi eğrileri boydan boya geçmeli.' };
  const step = v.step ?? 1;
  if (!(step !== 0 && Number.isFinite(step))) return { refused: 'Aralık 0 olamaz: eksi bir aralık kotları azaltır.' };
  const first = v.first ?? 0;
  const zs = contourElevations(curves, start, end, first, step);
  const update: { id: number; patch: Partial<Entity> }[] = [];
  let missed = 0;
  curves.forEach((e, k) => {
    const z = zs[k];
    if (z === null || z === undefined) {
      missed++;
      return;
    }
    const copy = structuredClone(e) as NewEntity;
    assignElevations(
      copy,
      elevatedPaths(e).map((p) => p.pts.map(() => z)),
    );
    const c = copy as unknown as Record<string, unknown>;
    const patch: Record<string, unknown> = {};
    for (const key of e.kind === 'line' ? ['za', 'zb'] : ['zs', 'holes', 'parts']) if (c[key] !== undefined) patch[key] = c[key];
    update.push({ id: e.id, patch: patch as Partial<Entity> });
  });
  if (missed > 0) feedback.warn(`${count(missed)} eğri kesen çizgiyle kesişmediği için değişmedi.`);
  if (!update.length) return { summary: 'Kesen çizgi seçilen eğrilerin hiçbirini kesmiyor.' };
  return {
    changes: { update },
    outputs: { changed: update.map((u) => u.id), count: update.length },
    summary: `${count(update.length)} eğriye kot verildi: ${short(first)} ile ${short(first + (update.length - 1) * step)} arası.`,
  };
}
