import { fixed } from '../../../core/displayNumber';
import { STOPPED, type PointResult } from '../../../io/rasterAnalysisProtocol';
import type { EntityKind, NewEntity, RasterEntity, RasterStyle } from '../../../model/entities';
import { rasterRunHost } from '../../rasterHost';
import type { Feedback, FeatureSet, RunContext, RunResult, Shown, TargetLayer } from '../../types';
import { RASTER_LAYER, count, withTif } from '../surface/shared';

/**
 * What İnterpolasyon and Yoğunluk's tools share (docs/adr/0232; the desktop's `builtin/interpolation/mod.rs`): their
 * parameters, the job's settings as the raster core reads them (`kentos_raster::from_points::PointSpec`), the run in
 * the page's analysis worker (processing/rasterHost.ts), the raster objects, the summary and the cross-validation's
 * table. The objects go to the worker as JSON, their points gathered by the core (one rule for both platforms).
 */

export const INTERPOLATION = 'interpolation';
export const DENSITY = 'density';

/** The kinds whose vertices an interpolation takes. */
export const POINT_KINDS: readonly EntityKind[] = ['point', 'line', 'polyline', 'polygon'];
/** The kinds Çizgi yoğunluğu takes: lines, curves and areas' boundaries. */
export const LINE_KINDS: readonly EntityKind[] = ['line', 'polyline', 'polygon', 'arc', 'circle', 'ellipse', 'spline'];
const SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** An interpolation's Noktalar and Değer alanı. */
export const POINTS = {
  name: 'input',
  label: 'Noktalar',
  type: 'features',
  kinds: POINT_KINDS,
  scopes: SCOPES,
  description: 'Değerleri okunan noktalar; çizgi, çoklu çizgi ve alanların köşeleri de alınır.',
} as const;
export const FIELD = {
  name: 'field',
  label: 'Değer alanı',
  type: 'field',
  of: 'input',
  optional: true,
  description: 'Boş bırakılırsa köşelerin kotu okunur; seçilirse nesnenin bu alandaki sayısı bütün köşelerine verilir.',
} as const;
export const DENSITY_POINTS = { name: 'input', label: 'Noktalar', type: 'features', kinds: ['point'], scopes: SCOPES, description: 'Yoğunluğu alınan noktalar.' } as const;
export const LINES = { name: 'input', label: 'Çizgiler', type: 'features', kinds: LINE_KINDS, scopes: SCOPES, description: 'Yoğunluğu alınan çizgiler; alanların sınırları da.' } as const;
export const weightField = (description: string) => ({ name: 'weightField', label: 'Ağırlık alanı', type: 'field', of: 'input', optional: true, description }) as const;

export const CELL = {
  name: 'cellSize',
  label: 'Hücre boyu',
  type: 'number',
  default: 0,
  min: 0,
  max: 1e6,
  unit: 'm',
  description: "0: kendiliğinden, kutunun kısa kenarının 250'de biri yuvarlanarak (1, 2, 2,5, 5 × 10ᵏ); ızgara bu boyun katlarına oturur.",
} as const;
export const EXTENT = {
  name: 'extent',
  label: 'Kapsam',
  type: 'enum',
  options: [
    { value: 'points', label: 'Girdinin kutusu' },
    { value: 'raster', label: 'Rasterin ızgarası' },
  ],
  default: 'points',
  description: 'Rasterin ızgarası: sonuç seçilen rasterle hücre hücre üst üste gelir.',
} as const;
export const GRID = {
  name: 'grid',
  label: 'Izgara rasteri',
  type: 'features',
  kinds: ['raster'],
  scopes: ['selection', 'layer'],
  optional: true,
  description: 'Izgarası (yeri, hücre boyu, boyu) alınan raster.',
  visibleWhen: (v: Shown) => v.extent === 'raster',
} as const;
export const CROSS = {
  name: 'cross',
  label: 'Çapraz doğrulama',
  type: 'boolean',
  default: false,
  description: 'Her nokta dışarıda bırakılıp öbürlerinden tahmin edilir; tablo ve karesel ortalama hata.',
} as const;
export const ADD = { name: 'add', label: 'Çizime ekle', type: 'boolean', default: true, description: 'Sonuç yeni katmanda nesne olarak eklenir; bu işlemin adımında.' } as const;

const adds = (v: Shown): boolean => v.add !== false;

/** Çıktı dosyası: on the web the result's name (empty: the input's layer's name and `suffix`). */
export const output = (suffix: string) =>
  ({
    name: 'output',
    label: 'Çıktı dosyası',
    type: 'string',
    default: '',
    allowEmpty: true,
    optional: true,
    placeholder: `<katman>${suffix}.tif`,
    description: `Sonucun dosya adı; boşsa girdinin katmanının adının sonuna “${suffix}” eklenir.`,
  }) as const;

/** A result's layer (shown while Çizime ekle is on): a new one goes right below the input's, the points over their surface. */
export const layer = (name: string) =>
  ({
    name: 'layer',
    label: 'Çıktı katmanı',
    type: 'layer',
    default: { newName: name },
    newLayerStyle: { color: RASTER_LAYER, lineWeight: 0.25 },
    below: 'input',
    description: 'Bu adda katman yoksa oluşturulur.',
    visibleWhen: adds,
  }) as const;

export const HELP_POINTS =
  'Noktaların, çizgi ve alanların köşelerinin kotları (ya da Değer alanı\'nın sayıları) alınır; aynı yerdeki köşeler değerlerinin ortalamasıyla tek nokta olur. Kotu olmayan köşe ve sayı olmayan değer alınmaz, söylenir.';
export const HELP_GRID =
  "Hücre boyu 0 ise noktaların kutusunun kısa kenarının 250'de biri, 1, 2, 2,5 ya da 5 × 10ᵏ'ye yuvarlanır; ızgara bu boyun katlarına oturur. Rasterin ızgarası seçilirse sonuç o rasterle hücre hücre üst üste gelir. Değer hücrenin merkezinde hesaplanır.";
export const HELP_OUTPUT =
  "Sonuç karolu, Deflate'li ve önizleme katlı 32 bit GeoTIFF'tir; çıktı dosyası boşsa girdinin katmanının adıyla adlanır. Web'de 32 MB'a kadar olan sonuç projeye gömülür, büyüğü indirilir ve bu oturumda bağlı kalır. Çizime ekle açıksa raster, girdinin katmanının hemen altındaki yeni katmana eklenir.";
export const HELP_CROSS =
  'Çapraz doğrulama her noktayı dışarıda bırakıp öbürlerinden tahmin eder: tabloda ölçülen, tahmin ve fark; özette ortalama fark, karesel ortalama hata ve ortalama mutlak fark.';

/** The parameters every tool ends with: the grid, (Çapraz doğrulama,) Çıktı dosyası, Çizime ekle, Çıktı katmanı. */
export const ends = <S extends string, L extends string>(suffix: S, name: L) => [CELL, EXTENT, GRID, output(suffix), ADD, layer(name)] as const;
export const endsCross = <S extends string, L extends string>(suffix: S, name: L) => [CELL, EXTENT, GRID, CROSS, output(suffix), ADD, layer(name)] as const;

/** A result's name from a layer's (the desktop's `Beside::stem`): its last part, a known extension cut. */
export function stemOfName(full: string): string {
  const base = (full.split(/[/\\]/).pop() ?? full).split(/[?#]/)[0];
  const lower = base.toLowerCase();
  const ext = ['.copc.laz', '.laz', '.las', '.xyz', '.pts', '.txt', '.csv', '.tif', '.tiff', '.png', '.jpg', '.jpeg'].find((e) => lower.endsWith(e));
  const stem = ext ? base.slice(0, base.length - ext.length) : base;
  return stem || 'yuzey';
}

const NO_HOST = 'Bu araç rasterin dosyasını okuyup sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** What a run takes: valued points, weighted points or lines. */
export type Input = 'points' | 'weighted' | 'lines';

const num = (v: number) => fixed(v, 3);

interface Notes {
  taken: number;
  merged: number;
  unread: number;
  noElevation: number;
  empty: number;
  radius?: number;
  variogram?: { model: string; nugget: number; sill: number; range: number };
  cross?: { count: number; missing: number; mean: number; rmse: number; mae: number; stdMean: number | null; stdRmse: number | null };
}

/** The cross-validation's table (docs/adr/0232 §12). */
function crossTable(r: PointResult, entities: readonly { label?: string }[], cad: boolean, kriging: boolean): { columns: string[]; rows: string[][] } {
  const columns = ['Sıra', 'Ad', ...(cad ? ['X', 'Y'] : ['Y', 'X']), 'Ölçülen', 'Tahmin', 'Fark', ...(kriging ? ['Standart hata', 'Standart fark'] : [])];
  const rows: string[][] = [];
  for (let k = 0; k < r.crossPoint.length; k++) {
    const [x, y, measured, predicted, error] = r.crossValues.subarray(5 * k, 5 * k + 5);
    const row = [String(r.crossPoint[k] + 1), entities[r.crossObject[k]]?.label ?? '', num(x), num(y), num(measured)];
    if (Number.isNaN(predicted)) row.push('', '');
    else row.push(num(predicted), num(predicted - measured));
    if (kriging) {
      if (Number.isNaN(predicted) || Number.isNaN(error)) row.push('', '');
      else row.push(num(error), error > 0 ? num((predicted - measured) / error) : '');
    }
    rows.push(row);
  }
  return { columns, rows };
}

/** The values a point tool runs with. */
interface PointValues {
  input?: FeatureSet | null;
  field?: string | null;
  weightField?: string | null;
  cellSize?: number | null;
  extent?: string | null;
  grid?: FeatureSet | null;
  cross?: boolean | null;
  output?: string | null;
  add?: boolean | null;
  layer?: TargetLayer | null;
  errorLayer?: TargetLayer | null;
}

/**
 * Runs a point job: the result kept (embedded or the session's file, named after the input's layer or as asked), its
 * object or objects on the layers, the summary, the notes and the cross-validation's table.
 */
export async function runPoints(v: PointValues, ctx: RunContext, feedback: Feedback, tool: Record<string, unknown>, suffix: string, label: string, input: Input): Promise<RunResult> {
  const host = rasterRunHost();
  if (!host) return { refused: NO_HOST };
  const entities = v.input?.entities ?? [];
  if (!entities.length) return { refused: 'Girdi boş: nesne seçin.' };
  const field = ((input === 'points' ? v.field : v.weightField) ?? '').trim();
  const values = field ? JSON.stringify(entities.map((e) => (Object.hasOwn(e.attrs, field) ? e.attrs[field] : null))) : 'null';
  const objects = JSON.stringify(entities, (k, x: unknown) => (k === 'attrs' || k === 'label' || k === 'uid' ? undefined : x));
  let grid: { affine: number[]; width: number; height: number } | null = null;
  if (v.extent === 'raster') {
    const rasters = (v.grid?.entities ?? []).filter((e): e is RasterEntity => e.kind === 'raster');
    if (rasters.length !== 1) return { refused: rasters.length ? `${rasters.length} raster seçili; ızgara için tek raster seçin.` : 'Izgara rasterini seçin: Kapsam rasterin ızgarası.' };
    grid = { affine: [...rasters[0].affine], width: rasters[0].width, height: rasters[0].height };
  }
  const srid = ctx.project.srid;
  const spec = JSON.stringify({
    tool,
    cell: v.cellSize ?? 0,
    grid,
    epsg: srid > 0 ? srid : null,
    system: ctx.crs?.system ?? null,
    cross: input === 'points' && v.cross === true,
  });
  const name = v.output?.trim() ? withTif(v.output.trim()) : `${stemOfName(ctx.layerName(entities[0].layerId))}${suffix}.tif`;
  let r: PointResult;
  try {
    const watch = {
      progress: (s: number) => feedback.progress(0.97 * s, label),
      get canceled() {
        return feedback.canceled;
      },
    };
    r = await host.analyzePoints(objects, values, spec, input === 'lines', watch);
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return why === STOPPED ? {} : { refused: why };
  }
  const [width, height] = [r.grid[6], r.grid[7]];
  const kept = await host.keep(r.bytes, name, width, height);
  feedback.progress(1, label);
  if (kept.note) feedback.info(kept.note);
  const notes = JSON.parse(r.notes) as Notes;
  if (notes.merged > 0) feedback.warn(`${count(notes.merged)} köşe aynı yerdeki noktayla birleşti (değerlerinin ortalaması alındı).`);
  if (notes.unread > 0) feedback.warn(`${count(notes.unread)} nesnenin ${input === 'points' ? 'değeri' : 'ağırlığı'} sayı olarak okunamadığı için alınmadı.`);
  if (notes.noElevation > 0) feedback.warn(`${count(notes.noElevation)} köşenin kotu olmadığı için alınmadı.`);
  if (notes.empty > 0) feedback.warn(`${count(notes.empty)} hücrede değer yok (noktaların kapsamı dışında ya da yeterli komşu yok).`);
  const add: NewEntity[] = [];
  const object = (layerId: string, style: string): NewEntity =>
    ({
      kind: 'raster',
      layerId,
      attrs: {},
      affine: r.grid.slice(0, 6) as RasterEntity['affine'],
      width,
      height,
      bands: r.bands,
      sample: 'f32',
      ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
      srid,
      style: JSON.parse(style) as RasterStyle,
    }) as NewEntity;
  if (v.add !== false && v.layer) {
    add.push(object(v.layer.id, r.styles[0]));
    if (r.bands === 2 && v.errorLayer) add.push(object(v.errorLayer.id, r.styles[1]));
  }
  const cells = `${count(width)} × ${count(height)} hücre`;
  let summary =
    input === 'points'
      ? `${count(notes.taken)} noktadan ${cells}lik raster; “${name}” yazıldı.`
      : input === 'weighted'
        ? `${count(notes.taken)} noktanın yoğunluğu, yarıçap ${num(notes.radius ?? NaN)} m; ${cells}; “${name}” yazıldı.`
        : `${count(notes.taken)} çizgi parçasının yoğunluğu, yarıçap ${num(notes.radius ?? NaN)} m; ${cells}; “${name}” yazıldı.`;
  const g = notes.variogram;
  if (g) summary += ` Variogram: ${g.model}, külçe ${num(g.nugget)}, kısmi eşik ${num(g.sill)}, erim ${num(g.range)} m.`;
  const outputs: Record<string, unknown> = { file: name };
  const s = notes.cross;
  if (s) {
    if (s.missing > 0) feedback.warn(`${count(s.missing)} noktanın çapraz doğrulaması yok (kabuğun üstünde ya da yeterli komşusu yok).`);
    if (s.count > 0) {
      summary += ` Çapraz doğrulama: ${count(s.count)} noktada ortalama fark ${num(s.mean)}, karesel ortalama hata ${num(s.rmse)}, ortalama mutlak fark ${num(s.mae)}.`;
      if (s.stdMean !== null && s.stdRmse !== null) summary += ` Standart farkların ortalaması ${num(s.stdMean)}, karesel ortalaması ${num(s.stdRmse)}.`;
    }
    outputs.table = crossTable(r, entities, ctx.project.type === 'cad', tool.kind === 'kriging');
  }
  return { changes: add.length ? { add } : undefined, outputs, summary };
}
