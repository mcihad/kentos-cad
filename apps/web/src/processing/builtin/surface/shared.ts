import { STOPPED, type AnalysisLines, type AnalysisResult } from '../../../io/rasterAnalysisProtocol';
import type { NewEntity, RasterEntity, RasterSample, RasterStyle } from '../../../model/entities';
import type { System } from '../../../model/geom/crsTransform';
import { rasterRunHost, type RasterRunHost } from '../../rasterHost';
import type { Feedback, FeatureSet, RunContext, RunResult, Shown, TargetLayer } from '../../types';

/**
 * What the surface tools share (docs/adr/0231; the desktop's `builtin/surface/mod.rs`): their parameters, the job's
 * settings as the raster core reads them, a raster result's name and object, contour lines as polylines. The job runs
 * in the page's analysis worker (processing/rasterHost.ts); a host without one refuses, as the desktop does without
 * files.
 */

export const CATEGORY = 'surface';

/** The layers' colour for a raster result (not drawn), and the contours' (brown, as topographic maps draw them). */
export const RASTER_LAYER = '#7A6B5B';
export const CONTOUR_LAYER = '#A0522D';

export const RASTER = { name: 'input', label: 'Raster', type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description: 'Çözümlenecek yükseklik rasteri (DEM): tek bir raster nesnesi.' } as const;
export const BAND = { name: 'band', label: 'Bant', type: 'number', default: 1, min: 1, max: 255, integer: true, unit: '', description: "Yüksekliklerin okunduğu bant (1'den)." } as const;
export const Z_FACTOR = { name: 'zFactor', label: 'Z çarpanı', type: 'number', default: 1, min: 1e-6, max: 1e6, unit: '', description: 'Yükseklikler bununla çarpılır: yüksekliklerin birimi metre değilse ya da düşey abartı için.' } as const;
export const METHOD = {
  name: 'method',
  label: 'Yöntem',
  type: 'enum',
  options: [
    { value: 'horn', label: 'Horn' },
    { value: 'zevenbergenThorne', label: 'Zevenbergen-Thorne' },
  ],
  default: 'horn',
  description: "Horn: sekiz komşu, gdaldem'in ve ArcGIS'in varsayılanı; Zevenbergen-Thorne: dört komşu.",
} as const;
export const ADD = { name: 'add', label: 'Çizime ekle', type: 'boolean', default: true, description: 'Sonuç yeni katmanda nesne olarak eklenir; bu işlemin adımında.' } as const;

const adds = (v: Shown): boolean => v.add !== false;

/** Çıktı dosyası: on the web the result's name (the desktop's is a path; empty: beside the source, `suffix` added). */
export const output = (suffix: string) =>
  ({
    name: 'output',
    label: 'Çıktı dosyası',
    type: 'string',
    default: '',
    allowEmpty: true,
    optional: true,
    placeholder: `<kaynak>${suffix}.tif`,
    description: `Sonucun dosya adı; boşsa kaynağın adının sonuna “${suffix}” eklenir.`,
  }) as const;

/** Çıktı katmanı (shown while Çizime ekle is on): a new one goes right above the raster's (under it the raster would hide it). */
export const layer = (name: string, color = RASTER_LAYER) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName: name },
    newLayerStyle: { color, lineWeight: 0.25 },
    above: 'input',
    description: 'Bu adda katman yoksa oluşturulur.',
    visibleWhen: adds,
  }) as const;

export const HELP_SOURCE =
  "Raster tek bir yükseklik rasteridir (DEM); bandın nodata'sı ve NaN boş sayılır. Pencerenin dışında kalan komşu kenar hücresinin, boş komşu merkezin değerini alır; boş hücrenin sonucu boştur. Coğrafi (derece) rasterde türevler satırın enleminde metreye çevrilir.";
export const HELP_OUTPUT =
  "Sonuç karolu, Deflate'li ve önizleme katlı GeoTIFF'tir; çıktı dosyası boşsa kaynağın adının sonuna ek konur. Web'de 32 MB'a kadar olan sonuç projeye gömülür, büyüğü indirilir ve bu oturumda bağlı kalır. Çizime ekle açıksa raster yeni katmana eklenir; görünüşü Raster stili ile değişir.";

/** The thousands apart (the desktop's `count_words`). */
export const count = (n: number): string => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');

/** A file's stem as the result names take it (the desktop's `Beside::stem`): its name without folder, query and known extension. */
export function stemOf(r: Pick<RasterEntity, 'file' | 'url' | 'asset'>): string {
  const full = r.file ?? r.url ?? r.asset ?? '';
  const base = (full.split(/[/\\]/).pop() ?? full).split(/[?#]/)[0];
  const lower = base.toLowerCase();
  const ext = ['.copc.laz', '.laz', '.las', '.xyz', '.pts', '.txt', '.csv', '.tif', '.tiff', '.png', '.jpg', '.jpeg'].find((e) => lower.endsWith(e));
  const stem = ext ? base.slice(0, base.length - ext.length) : base;
  return stem || 'raster';
}

/** `name` with its raster extension made `.tif` (the desktop's `with_extension`). */
export function withTif(name: string): string {
  const lower = name.toLowerCase();
  const ext = ['.copc.laz', '.laz', '.las', '.tif', '.tiff', '.vpc'].find((e) => lower.endsWith(e));
  return `${ext ? name.slice(0, name.length - ext.length) : name}.tif`;
}

/** The run's raster: the one object of the input, or why not. */
export function oneRaster(input: FeatureSet | null | undefined): RasterEntity | { refused: string } {
  const rasters = (input?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
  if (rasters.length === 1) return rasters[0];
  return { refused: rasters.length ? `${rasters.length} raster seçili; çözümleme tek raster ister.` : 'Raster seçin: çözümleme bir yükseklik rasteri (DEM) ister.' };
}

/** The job's settings (`kentos_raster::job::Spec`): the tool's, the raster's place, band, nodata and the project's system. */
export function specOf(tool: Record<string, unknown>, band: number | null | undefined, r: RasterEntity, system: System | null | undefined): string {
  return JSON.stringify({ tool, band: band ?? 1, affine: r.affine, nodata: r.style.nodata ?? null, epsg: r.srid > 0 ? r.srid : null, system: system ?? null });
}

const NO_HOST = 'Bu araç rasterin dosyasını okuyup sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** A job's end: its result and the host that keeps it, or the run's end (refused, or stopped saying nothing). */
type Ran = { ok: true; host: RasterRunHost; result: AnalysisResult } | { ok: false; end: RunResult };

/** Runs the job over the raster. */
async function analyze(r: RasterEntity, spec: string, feedback: Feedback, label: string): Promise<Ran> {
  const host = rasterRunHost();
  if (!host) return { ok: false, end: { refused: NO_HOST } };
  const blob = host.source(r);
  if (!(blob instanceof Blob)) return { ok: false, end: blob };
  try {
    const watch = {
      progress: (s: number) => feedback.progress(0.97 * s, label),
      get canceled() {
        return feedback.canceled;
      },
    };
    return { ok: true, host, result: await host.analyze(blob, spec, watch) };
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return { ok: false, end: why === STOPPED ? {} : { refused: why } };
  }
}

/** A raster tool's run: the result file kept (embedded or the session's) and, asked, its object on the layer. */
export async function runRaster(
  v: { input?: FeatureSet | null; band?: number | null; output?: string | null; add?: boolean | null; layer?: TargetLayer | null },
  ctx: RunContext,
  feedback: Feedback,
  tool: Record<string, unknown>,
  suffix: string,
  label: string,
): Promise<RunResult> {
  const r = oneRaster(v.input);
  if ('refused' in r) return r;
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOf(r)}${suffix}.tif`;
  const ran = await analyze(r, specOf(tool, v.band, r, ctx.crs?.system), feedback, label);
  if (!ran.ok) return ran.end;
  if (!('raster' in ran.result)) return { refused: 'Çözümleme raster vermedi.' };
  const { bytes, bands, sample, style } = ran.result.raster;
  const kept = await ran.host.keep(bytes, name, r.width, r.height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const add: NewEntity[] = [];
  if (v.add !== false && v.layer) {
    const object: Omit<RasterEntity, 'id' | 'uid'> = {
      kind: 'raster',
      layerId: v.layer.id,
      attrs: {},
      affine: [...r.affine],
      width: r.width,
      height: r.height,
      bands,
      sample: sample as RasterSample,
      ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
      srid: r.srid,
      style: JSON.parse(style) as RasterStyle,
    };
    add.push(object);
  }
  return { changes: add.length ? { add } : undefined, outputs: { file: name }, summary: `${count(r.width)} × ${count(r.height)} hücrelik raster; “${name}” yazıldı.` };
}

/** The contour lines as polylines on the layer: their vertices at their level, Kot and Tür; a main line 0.35 mm. */
export function lineObjects(l: AnalysisLines, layerId: string): NewEntity[] {
  const out: NewEntity[] = [];
  let at = 0;
  for (let i = 0; i < l.sizes.length; i++) {
    const n = l.sizes[i];
    const pts = [];
    for (let k = 0; k < n; k++) pts.push({ x: l.points[2 * (at + k)], y: l.points[2 * (at + k) + 1] });
    at += n;
    const main = l.main[i] === 1;
    out.push({ kind: 'polyline', layerId, attrs: { Kot: l.texts[i], Tür: main ? 'Ana' : 'Ara' }, pts, zs: new Array<number>(n).fill(l.values[i]), ...(main ? { lineWeight: 0.35 } : {}) });
  }
  return out;
}

/** Eş yükselti eğrileri's run: the lines on the layer. */
export async function runLines(v: { input?: FeatureSet | null; band?: number | null; layer?: TargetLayer | null }, ctx: RunContext, feedback: Feedback, tool: Record<string, unknown>): Promise<RunResult> {
  const r = oneRaster(v.input);
  if ('refused' in r) return r;
  const label = 'Eğriler çıkarılıyor';
  const ran = await analyze(r, specOf(tool, v.band, r, ctx.crs?.system), feedback, label);
  if (!ran.ok) return ran.end;
  if (!('lines' in ran.result)) return { refused: 'Çözümleme eğri vermedi.' };
  const l = ran.result.lines;
  feedback.progress(1, label);
  const lines = l.sizes.length;
  const main = l.main.reduce((a, b) => a + b, 0);
  const vertices = l.sizes.reduce((a, b) => a + b, 0);
  const summary = lines
    ? `${count(lines)} eğri (${count(main)} ana, ${count(lines - main)} ara), ${count(vertices)} köşe; kotlar ${l.texts[0]} ile ${l.texts[lines - 1]} arası.`
    : 'Bu aralıkla eğri çıkmadı: rasterin değerleri hiçbir düzeyi geçmiyor.';
  const add = v.layer ? lineObjects(l, v.layer.id) : [];
  return { changes: add.length ? { add } : undefined, outputs: { lines }, summary };
}
