import { fixed } from '../../../core/displayNumber';
import { STOPPED, type OpsResult } from '../../../io/rasterAnalysisProtocol';
import type { Entity, NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import { rasterRunHost, type RasterRunHost } from '../../rasterHost';
import type { Feedback, FeatureSet, RunContext, RunResult, Shown, TargetLayer } from '../../types';
import { rasterRun } from '../../features';
import { meanScale, withAttr } from '../attributeWrites';
import { RASTER_LAYER, count, stemOf, withTif } from '../surface/shared';

/**
 * What Raster işlemleri and Raster istatistiği's tools share (docs/adr/0233; the desktop's `builtin/raster_ops/mod.rs`):
 * their parameters, the inputs in the run's order, the job's settings as the raster core reads them
 * (`kentos_raster::ops::OpsSpec`), the run in the page's analysis worker (processing/rasterHost.ts), the raster object,
 * the zones' writes and table, the histogram's table, and the summaries.
 */

export const OPS = 'rasterOps';
export const STATS = 'rasterStats';

/** The kinds an area of a mask or a zone is. */
export const AREA_KINDS = ['polygon', 'circle', 'ellipse', 'spline', 'hatch'] as const;

/** One raster. */
export const oneRaster = (description: string) => ({ name: 'input', label: 'Raster', type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description }) as const;

/** Several rasters, `first` the scope offered first. */
export const rasters = <F extends 'visible' | 'selection'>(description: string, first: F) =>
  ({
    name: 'input',
    label: 'Rasterler',
    type: 'features',
    kinds: ['raster'],
    scopes: first === 'visible' ? (['visible', 'selection', 'layer', 'all'] as const) : (['selection', 'visible', 'layer', 'all'] as const),
    description,
  }) as const;

/** Areas: a mask's or the zones' (closed objects). */
export const areas = <N extends string, L extends string>(name: N, label: L, writes: boolean, description: string) =>
  ({ name, label, type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], ...(writes ? { writes: true } : {}), description }) as const;

export const BAND = { name: 'band', label: 'Bant', type: 'number', default: 1, min: 1, max: 255, integer: true, unit: '', description: "Yüksekliklerin okunduğu bant (1'den)." } as const;

/** The statistics a cell's window or stack gives (§9); Sayı too where `withCount`. */
const STAT_OPTIONS = [
  { value: 'mean', label: 'Ortalama' },
  { value: 'sum', label: 'Toplam' },
  { value: 'min', label: 'En küçük' },
  { value: 'max', label: 'En büyük' },
  { value: 'range', label: 'Aralık' },
  { value: 'std', label: 'Standart sapma' },
  { value: 'median', label: 'Ortanca' },
  { value: 'majority', label: 'Çoğunluk' },
  { value: 'minority', label: 'Azınlık' },
  { value: 'variety', label: 'Çeşit' },
] as const;
const STAT_DESCRIPTION = 'Standart sapma örneklemindir (n − 1); Çoğunluk en sık, Azınlık en seyrek değer (eşitse küçüğü); Çeşit farklı değer sayısı.';
export const STAT = { name: 'stat', label: 'İstatistik', type: 'enum', options: STAT_OPTIONS, default: 'mean', description: STAT_DESCRIPTION } as const;
export const STAT_COUNT = { name: 'stat', label: 'İstatistik', type: 'enum', options: [...STAT_OPTIONS, { value: 'count', label: 'Sayı' }], default: 'mean', description: STAT_DESCRIPTION } as const;

export const IGNORE = {
  name: 'ignore',
  label: 'Değersizleri yok say',
  type: 'boolean',
  default: true,
  description: 'Açıkken değeri olan hücrelerin istatistiği; kapalıyken değeri olmayan hücre varsa sonuç değersiz.',
} as const;

export const ADD = { name: 'add', label: 'Çizime ekle', type: 'boolean', default: true, description: 'Sonuç yeni katmanda nesne olarak eklenir; bu işlemin adımında.' } as const;
const adds = (v: Shown): boolean => v.add !== false;

/** Çıktı dosyası: on the web the result's name (empty: the first raster's name and `suffix`). */
export const output = (suffix: string) =>
  ({
    name: 'output',
    label: 'Çıktı dosyası',
    type: 'string',
    default: '',
    allowEmpty: true,
    optional: true,
    placeholder: `<kaynak>${suffix}.tif`,
    description: `Sonucun dosya adı; boşsa ilk rasterin adının sonuna “${suffix}” eklenir.`,
  }) as const;

/** Çıktı katmanı (shown while Çizime ekle is on): a new one goes right above the first raster's. */
export const layer = <N extends string>(name: N) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName: name },
    newLayerStyle: { color: RASTER_LAYER, lineWeight: 0.25 },
    above: 'input',
    description: 'Bu adda katman yoksa oluşturulur.',
    visibleWhen: adds,
  }) as const;

export const ends = <S extends string, N extends string>(suffix: S, name: N) => [output(suffix), ADD, layer(name)] as const;

export const HELP_OUTPUT =
  "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa ilk rasterin adının sonuna ek konur. Web'de 32 MB'a kadar olan sonuç projeye gömülür, büyüğü indirilir ve bu oturumda bağlı kalır. Çizime ekle açıksa raster ilk rasterin katmanının hemen üstündeki yeni katmana eklenir.";
export const HELP_EMPTY = "Rasterin nodata'sı, NaN ve alfası 0 olan pikseller değersizdir.";

/** The input's rasters in the run's order and their names in an expression (§2, §3; processing/features.ts). */
function rastersOf(input: FeatureSet | null | undefined, ctx: RunContext): { list: RasterEntity[]; names: string[] } {
  const run = rasterRun(
    (input?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster'),
    { layerIndex: (id) => ctx.layerIndex(id), byLayer: (id) => ctx.doc.byLayer(id), layerName: (id) => ctx.layerName(id) },
  );
  return { list: run.map((r) => r.raster), names: run.map((r) => r.name) };
}

/** The run's settings: the tool's, and each input's place, nodata, name and look. */
export function specOf(tool: Record<string, unknown>, list: readonly RasterEntity[], names: readonly string[], ctx: RunContext): string {
  return JSON.stringify({
    tool,
    inputs: list.map((r, k) => ({ affine: r.affine, nodata: r.style.nodata ?? null, name: names[k], style: r.style })),
    epsg: ctx.project.srid > 0 ? ctx.project.srid : null,
    geographic: ctx.crs?.system?.kind === 'geographic',
  });
}

const NO_HOST = 'Bu araç rasterin dosyasını okuyup sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** A job's end: its result and the host that keeps it, or the run's end. */
type Ran = { ok: true; host: RasterRunHost; result: OpsResult } | { ok: false; end: RunResult };

/**
 * Runs the job over the rasters (and the areas' objects). A raster whose file cannot be read is said only when the run
 * reads it (Raster hesaplayıcı reads the rasters its expression names).
 */
async function analyze(list: readonly RasterEntity[], spec: string, shapes: readonly Entity[], feedback: Feedback, label: string): Promise<Ran> {
  const host = rasterRunHost();
  if (!host) return { ok: false, end: { refused: NO_HOST } };
  const sources = list.map((r) => {
    const b = host.source(r);
    return b instanceof Blob ? b : b.refused;
  });
  const objects = JSON.stringify(shapes, (k, x: unknown) => (k === 'attrs' || k === 'label' || k === 'uid' ? undefined : x));
  try {
    const watch = {
      progress: (s: number) => feedback.progress(0.97 * s, label),
      get canceled() {
        return feedback.canceled;
      },
    };
    return { ok: true, host, result: await host.analyzeOps(sources, spec, objects, watch) };
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return { ok: false, end: why === STOPPED ? {} : { refused: why } };
  }
}

/** The rasters of the input and their names, or why the tool does not run. */
function rastersFor(input: FeatureSet | null | undefined, ctx: RunContext, one: boolean): { list: RasterEntity[]; names: string[] } | { refused: string } {
  const r = rastersOf(input, ctx);
  if (!r.list.length) return { refused: 'Raster seçin: bu araç raster ister.' };
  if (one && r.list.length > 1) return { refused: `${r.list.length} raster seçili; bu araç tek raster ister.` };
  return r;
}

/** What a raster tool reads of its values. */
export interface RasterValues {
  input?: FeatureSet | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
}

/** Runs a tool whose result is a raster: the file kept, its object, the summary. */
export async function runRasterOp(
  v: RasterValues & { mask?: FeatureSet | null },
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  suffix: string,
  label: string,
  one: boolean,
  shapes: 'mask' | null,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, one);
  if ('refused' in got) return got;
  const { list, names } = got;
  const ran = await analyze(list, specOf(tool, list, names, ctx), shapes ? (v.mask?.entities ?? []) : [], feedback, label);
  if (!ran.ok) return ran.end;
  const r = ran.result;
  if (!r.bytes) return { refused: 'Çözümleme raster vermedi.' };
  // The rasters read (Raster hesaplayıcı's those its expression names, the grid's first): the first names the file,
  // gives the system, and its layer has the result's right above it.
  const read = r.reads.map((k) => list[k]);
  const first = read[0];
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOf(first)}${suffix}.tif`;
  const [width, height] = [r.grid[6], r.grid[7]];
  const kept = await ran.host.keep(r.bytes, name, width, height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const notes = JSON.parse(r.notes) as { cells: number; emptyCells: number };
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
  let summary = `${count(width)} × ${count(height)} hücrelik raster; “${name}” yazıldı.`;
  const kind = tool.kind;
  if (kind === 'clipByMask') summary += ` Maskenin içinde ${count(notes.cells)} hücre.`;
  else if (kind === 'reclassify' && notes.cells > 0) summary += ` Tablonun hiçbir kuralının tutmadığı ${count(notes.cells)} hücre.`;
  else if (kind === 'mosaic' || kind === 'cellStatistics') summary += ` ${count(read.length)} raster birleşti.`;
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary, above: first.layerId };
}

/** A figure as the table and the fields write it: counts whole, the rest at `decimals`; none empty. */
export function figure(v: number, whole: boolean, decimals: number): string {
  if (Number.isNaN(v)) return '';
  return fixed(v, whole ? 0 : decimals);
}

const STAT_LABEL: Record<string, string> = {
  count: 'Sayı',
  sum: 'Toplam',
  mean: 'Ortalama',
  min: 'En küçük',
  max: 'En büyük',
  range: 'Aralık',
  std: 'Standart sapma',
  median: 'Ortanca',
  majority: 'Çoğunluk',
  minority: 'Azınlık',
  variety: 'Çeşit',
  area: 'Alan',
};

/** Runs Bölgesel istatistik: the statistic into each zone's field, the table. */
export async function runZonal(
  v: { input?: FeatureSet | null; band?: number | null; zones?: FeatureSet | null; stat?: string | null; output?: string | null; decimals?: number | null },
  ctx: RunContext,
  feedback: Feedback,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const zones = v.zones?.entities ?? [];
  if (!zones.length) return { refused: 'Bölgeleri seçin: kapalı alanlar.' };
  const stat = v.stat ?? 'mean';
  const label = 'Bölgelerin istatistikleri hesaplanıyor';
  const ran = await analyze(list, specOf({ kind: 'zonalStatistics', band: v.band ?? 1, stat }, list, names, ctx), zones, feedback, label);
  if (!ran.ok) return ran.end;
  feedback.progress(1, label);
  const figs = ran.result.zones ?? new Float64Array();
  const output = (v.output ?? '').trim();
  const decimals = Math.max(0, Math.min(12, v.decimals ?? 3));
  const whole = stat === 'count' || stat === 'variety';
  const extra = !['count', 'sum', 'mean', 'min', 'max', 'std'].includes(stat);
  const update: { id: number; patch: { attrs: Record<string, string> } }[] = [];
  const rows: string[][] = [];
  let empty = 0;
  zones.forEach((z, k) => {
    const [n, value, sum, mean, min, max, std] = figs.subarray(7 * k, 7 * k + 7);
    if (n === 0) empty++;
    const text = figure(value, whole, meanScale(ctx, z, output) ?? decimals);
    if (output) {
      const attrs = withAttr(ctx, z, output, text === '' ? null : text);
      if (attrs) update.push({ id: z.id, patch: { attrs } });
    }
    const row = [z.label ?? `#${z.id}`, fixed(n, 0), figure(sum, false, decimals), figure(mean, false, decimals), figure(min, false, decimals), figure(max, false, decimals), figure(std, false, decimals)];
    if (extra) row.push(figure(value, whole, decimals));
    rows.push(row);
  });
  if (empty > 0) feedback.warn(`${count(empty)} bölgenin içinde değeri olan hücre merkezi yok.`);
  const columns = ['Nesne', 'Sayı', 'Toplam', 'Ortalama', 'En küçük', 'En büyük', 'Standart sapma', ...(extra ? [STAT_LABEL[stat] ?? stat] : [])];
  const summary = output
    ? `${update.length} bölgeye “${output}” yazıldı (${(STAT_LABEL[stat] ?? stat).toLocaleLowerCase('tr')}).`
    : `${count(zones.length)} bölgenin istatistikleri tabloda.`;
  return {
    changes: update.length ? { update } : undefined,
    outputs: { table: { columns, rows }, changed: update.map((u) => u.id), count: update.length },
    summary,
  };
}

/** Runs Histogram: the table of intervals. */
export async function runHistogram(
  v: { input?: FeatureSet | null; band?: number | null; bins?: number | null; min?: number | null; max?: number | null },
  ctx: RunContext,
  feedback: Feedback,
): Promise<RunResult> {
  const got = rastersFor(v.input, ctx, true);
  if ('refused' in got) return got;
  const { list, names } = got;
  const label = 'Histogram çıkarılıyor';
  const tool = { kind: 'histogram', band: v.band ?? 1, bins: v.bins ?? 20, min: v.min ?? null, max: v.max ?? null };
  const ran = await analyze(list, specOf(tool, list, names, ctx), [], feedback, label);
  if (!ran.ok) return ran.end;
  feedback.progress(1, label);
  const h = JSON.parse(ran.result.histogram ?? '{}') as { lo: number; hi: number; counts: number[]; below: number; above: number; valid: number; empty: number };
  if (h.valid === 0) return { summary: 'Bandın hiçbir hücresinde değer yok.' };
  const n = h.counts.length;
  const total = Math.max(1, h.counts.reduce((a, b) => a + b, 0));
  let soFar = 0;
  const rows = h.counts.map((c, k) => {
    soFar += c;
    const [a, b] = h.hi > h.lo ? [h.lo + ((h.hi - h.lo) * k) / n, k + 1 === n ? h.hi : h.lo + ((h.hi - h.lo) * (k + 1)) / n] : [h.lo, h.hi];
    return [String(k + 1), fixed(a, 3), fixed(b, 3), String(c), fixed((100 * c) / total, 2), fixed((100 * soFar) / total, 2)];
  });
  let summary = `${count(h.valid)} hücre, ${fixed(h.lo, 3)} ile ${fixed(h.hi, 3)} arası ${count(n)} aralık.`;
  if (h.empty > 0) summary += ` Değeri olmayan ${count(h.empty)} hücre.`;
  if (h.below + h.above > 0) summary += ` Aralığın altında ${count(h.below)}, üstünde ${count(h.above)} hücre.`;
  return { outputs: { table: { columns: ['Aralık', 'Alt sınır', 'Üst sınır', 'Sayı', 'Oran (%)', 'Birikimli (%)'], rows } }, summary };
}
