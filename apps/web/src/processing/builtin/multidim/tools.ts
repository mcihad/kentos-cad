import type { MultidimRequest, MultidimResult } from '../../../io/rasterAnalysisProtocol';
import type { NewEntity, RasterEntity, RasterStyle } from '../../../model/entities';
import { STOPPED } from '../../../io/rasterAnalysisProtocol';
import { rasterPart, rasterRunHost, type RasterRunHost } from '../../rasterHost';
import { defineTool, type Feedback, type FeatureSet, type RunResult, type Shown, type TableOutput, type TargetLayer } from '../../types';
import { ADD, layer as resultLayer } from '../rasterOps/shared';
import { oneRaster, stemOf } from '../surface/shared';

/**
 * Çok boyutlu veri (docs/adr/0243 §8–§10; the desktop's `builtin/multidim/`): Kesit (a raster's values along lines: a
 * table, and the lines with the values as their vertices' elevations on a new layer), Zaman serisi (values at points over
 * a NetCDF dataset's time steps, or over a raster's bands: a table) and Mesh hesaplayıcı (a new dataset on a mesh: a UGRID
 * NetCDF kept as a raster result is, and a raster showing it right above the source's layer). The jobs are the raster
 * core's (`kentos_raster::multidim`) in the page's analysis worker; their tables and texts are the core's.
 */

export const MULTIDIM = 'multidim';

const NO_HOST = 'Bu araç rasterin dosyasını okuyup sonucu dosyaya yazar; bu ortamda dosya erişimi yok.';

/** The lines a Kesit walks. */
export const LINE_KINDS = ['line', 'polyline', 'arc', 'circle', 'ellipse', 'spline'] as const;

/** What a tool's job gave (the core's JSON). */
interface Done {
  table: TableOutput | null;
  pieces: { line: number; points: [number, number, number][] }[];
  summary: string;
  warnings: string[];
}

/** The objects as the core reads them: shapes, and a point's `ad`. */
function objectsJson(set: FeatureSet | null | undefined, attrs: boolean): string {
  return JSON.stringify(set?.entities ?? [], (k, x: unknown) => (k === 'label' || k === 'uid' || (!attrs && k === 'attrs') ? undefined : x));
}

/** Runs a tool's job over the raster in the analysis worker. */
async function run(
  r: RasterEntity,
  request: Omit<MultidimRequest, 'type' | 'blob' | 'part' | 'affine' | 'nodata'>,
  feedback: Feedback,
  label: string,
): Promise<{ ok: true; host: RasterRunHost; result: MultidimResult; done: Done } | { ok: false; end: RunResult }> {
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
    const result = await host.analyzeMultidim({ ...request, blob, part: rasterPart(r), affine: [...r.affine], nodata: r.style.nodata ?? null }, watch);
    const done = JSON.parse(result.result) as Done;
    for (const w of done.warnings) feedback.warn(w);
    feedback.progress(1, label);
    return { ok: true, host, result, done };
  } catch (e) {
    const why = e instanceof Error ? e.message : String(e);
    return { ok: false, end: why === STOPPED ? {} : { refused: why } };
  }
}

const raster = (description: string) => ({ name: 'input', label: 'Raster', type: 'features', kinds: ['raster'], scopes: ['selection', 'layer'], description }) as const;
const draws = (v: Shown): boolean => v.draw === true;

export const profile = defineTool({
  id: 'multidim.profile',
  label: 'Kesit',
  category: MULTIDIM,
  icon: 'multidimProfile',
  description: 'Rasterin değerlerini çizgiler boyunca, Adım aralıklarla okur: tablo ve isteğe bağlı kotlu çizgiler.',
  help: [
    'Her çizginin başından Adım aralıklarla ve sonunda bir nokta; yay, daire, elips ve eğri kendi eğrileriyle yürünür. Değer noktanın düştüğü hücrenin değeridir; mesh rasterde ağdan enterpolasyondur (pikselin değil). NetCDF ve mesh rasterde gösterilen dilim okunur.',
    'Tablo: Çizgi, Uzaklık (m), koordinatlar ve Değer; panoya kopyalanır ya da CSV olarak kaydedilir. Kotlu çizgi açıksa değerler köşe kotu olan çoklu çizgiler rasterin katmanının hemen üstündeki yeni katmana eklenir.',
  ].join('\n\n'),
  keywords: ['kesit', 'profil', 'boyuna kesit', 'profile', 'cross section', 'transect'],
  aliases: ['KESIT', 'PROFIL', 'BOYKESIT'],
  targets: ['client'],
  parameters: [
    raster('Değerleri okunan raster: ızgara, NetCDF dilimi ya da mesh.'),
    {
      name: 'lines',
      label: 'Çizgiler',
      type: 'features',
      kinds: LINE_KINDS,
      scopes: ['selection', 'layer'],
      description: 'Kesitin çizgileri: çizgi, çoklu çizgi, yay, daire, elips ya da eğri; her biri başından yürünür.',
    },
    {
      name: 'step',
      label: 'Adım',
      type: 'number',
      min: 1e-6,
      max: 1e9,
      unit: 'm',
      optional: true,
      default: null,
      placeholder: 'Hücre boyu',
      description: 'Çizginin başından bu aralıklarla ve sonunda nokta; boşsa rasterin hücre boyu.',
    },
    { name: 'band', label: 'Bant', type: 'number', default: 1, min: 1, max: 255, integer: true, unit: '', description: "Çok bantlı rasterde değerin okunduğu bant (1'den)." },
    {
      name: 'draw',
      label: 'Kotlu çizgi',
      type: 'boolean',
      default: false,
      description: 'Değerler köşe kotu olan çoklu çizgiler yeni katmana eklenir; değersiz noktalar çizgiyi böler.',
    },
    { ...resultLayer('Kesit'), newLayerStyle: { color: '#B4572E', lineWeight: 0.35 }, visibleWhen: draws },
  ],
  outputs: [{ name: 'table', label: 'Kesit', type: 'table' }],
  run: async (v, ctx, feedback) => {
    const r = oneRaster(v.input);
    if ('refused' in r) return r;
    const step = v.step ?? Math.hypot(r.affine[1], r.affine[4]);
    const axes = ctx.project.type === 'cad' ? 'X,Y' : 'Y,X';
    const ran = await run(r, { kind: 'profile', objects: objectsJson(v.lines, false), step, band: v.band ?? 1, axes }, feedback, 'Kesit çıkarılıyor');
    if (!ran.ok) return ran.end;
    const { done } = ran;
    const target = v.layer as TargetLayer | null | undefined;
    const add: NewEntity[] =
      v.draw === true && target
        ? done.pieces.map((p) => ({
            kind: 'polyline' as const,
            layerId: target.id,
            attrs: { Çizgi: String(p.line) },
            pts: p.points.map(([x, y]) => ({ x, y })),
            zs: p.points.map((q) => q[2]),
          }))
        : [];
    return { changes: add.length ? { add } : undefined, outputs: done.table ? { table: done.table } : {}, summary: done.summary, above: r.layerId };
  },
});

export const series = defineTool({
  id: 'multidim.series',
  label: 'Zaman serisi',
  category: MULTIDIM,
  icon: 'timeSeries',
  description: 'Noktalardaki değerleri veri setinin her zaman adımında (ya da rasterin her bandında) okur: tablo.',
  help: [
    'Zaman boyutlu veri setinde öbür boyutlar rasterin gösterdiği değerlerde kalır; yalnız noktaların değerleri okunur. Zaman boyutu olmayan çok bantlı rasterde her bant bir satırdır. Değer hücrenin değeridir; mesh rasterde ağdan enterpolasyondur.',
    'Tablo: satırlar adımlar (Adım ve Zaman, ya da Bant), sütunlar noktalar; panoya kopyalanır ya da CSV olarak kaydedilir. Çizim değişmez.',
  ].join('\n\n'),
  keywords: ['zaman serisi', 'zaman', 'seri', 'time series', 'netcdf', 'mesh'],
  aliases: ['ZAMANSERISI', 'ZSERI'],
  targets: ['client'],
  parameters: [
    raster('Zaman boyutlu veri seti (NetCDF ya da mesh) ya da çok bantlı raster.'),
    {
      name: 'points',
      label: 'Noktalar',
      type: 'features',
      kinds: ['point'],
      scopes: ['selection', 'layer'],
      description: 'Değerlerin okunduğu noktalar; çok noktalı nesnenin her noktası. Sütunun adı noktanın ad özniteliği.',
    },
  ],
  outputs: [{ name: 'table', label: 'Zaman serisi', type: 'table' }],
  run: async (v, _ctx, feedback) => {
    const r = oneRaster(v.input);
    if ('refused' in r) return r;
    const timed = r.dataset?.dims?.some((d) => d.time) ?? false;
    const ran = await run(r, { kind: timed ? 'timeSeries' : 'bandSeries', objects: objectsJson(v.points, true) }, feedback, 'Zaman serisi okunuyor');
    if (!ran.ok) return ran.end;
    return { outputs: ran.done.table ? { table: ran.done.table } : {}, summary: ran.done.summary };
  },
});

/** A NetCDF file's name ends in `.nc`. */
function withNc(name: string): string {
  const lower = name.toLowerCase();
  const ext = ['.nc', '.tif', '.tiff'].find((e) => lower.endsWith(e));
  return `${ext ? name.slice(0, name.length - ext.length) : name}.nc`;
}

export const meshCalculator = defineTool({
  id: 'multidim.meshCalculator',
  label: 'Mesh hesaplayıcı',
  category: MULTIDIM,
  icon: 'meshCalculator',
  description: "Mesh'in veri setlerinden ifadeyle yeni veri seti yazar: aynı ağda, yeni UGRID dosyasında.",
  help: [
    'İfadenin andığı veri setleri aynı konumda (düğümlerde ya da yüzlerde) olmalı; zamanlı olanlar aynı zaman adımlarında, zamansızlar her adımda geçerlidir; katmanlı veri seti okunmaz. Değersiz girdi değersiz sonuç verir; etkin olmayan yüzün değeri yoktur.',
    "Örnekler: [depth] * 2; [Su derinliği] + bed; durum eğer depth > 0,5 ise 1 yoksa 0 son. Matematik işlevleri Raster hesaplayıcı'nınkiler.",
    "Sonuç kaynağın ağı, zamanı (özetle yok olur) ve yeni veri setiyle 32 bit ondalık UGRID NetCDF'tir; web'de 32 MB'a kadar olanı projeye gömülür, büyüğü indirilir. Çizime ekle açıksa onu gösteren mesh raster kaynağın katmanının hemen üstündeki yeni katmana eklenir.",
  ].join('\n\n'),
  keywords: ['mesh', 'hesap', 'ifade', 'veri seti', 'mesh calculator', 'ugrid'],
  aliases: ['MESHHESAP', 'AGHESAP'],
  targets: ['client'],
  parameters: [
    raster('Mesh ekle ile eklenen mesh raster; ifadenin veri setleri onun ağındandır.'),
    {
      name: 'expression',
      label: 'İfade',
      type: 'expression',
      returns: 'value',
      placeholder: '[depth] * 2',
      description: 'Veri setleri değişken adlarıyla ya da gösterilen adlarıyla ([Su derinliği]); vektörün adı büyüklüğü.',
    },
    {
      name: 'summary',
      label: 'Zaman özeti',
      type: 'enum',
      options: [
        { value: 'none', label: 'Yok' },
        { value: 'max', label: 'En büyük' },
        { value: 'min', label: 'En küçük' },
        { value: 'mean', label: 'Ortalama' },
        { value: 'sum', label: 'Toplam' },
      ],
      default: 'none',
      description: 'Yok: her zaman adımı yazılır; öbürleri adımları birleştirir (değersizler atlanır).',
    },
    { name: 'name', label: 'Veri setinin adı', type: 'string', default: 'Hesap', placeholder: 'Hesap', maxLength: 256 },
    {
      name: 'output',
      label: 'Çıktı dosyası',
      type: 'string',
      default: '',
      allowEmpty: true,
      optional: true,
      placeholder: '<kaynak>-hesap.nc',
      description: 'Boş bırakılırsa kaynağın adının sonuna -hesap eklenir (UGRID NetCDF).',
    },
    ADD,
    resultLayer('Mesh hesabı'),
  ],
  outputs: [{ name: 'file', label: 'Sonuç dosyası', type: 'string' }],
  run: async (v, _ctx, feedback) => {
    const r = oneRaster(v.input);
    if ('refused' in r) return r;
    const d = r.dataset;
    if (!d?.mesh) return { refused: 'Mesh hesaplayıcı mesh raster ister: Mesh ekle ile eklenen bir raster seçin.' };
    const spec = JSON.stringify({ expression: v.expression?.source ?? '', summary: v.summary ?? 'none', name: v.name ?? '' });
    const ran = await run(r, { kind: 'meshCalc', objects: '[]', spec }, feedback, 'Mesh hesaplanıyor');
    if (!ran.ok) return ran.end;
    const file = ran.result.file;
    if (!file) return { refused: 'Mesh hesaplayıcı dosya yazmadı.' };
    const name = v.output?.trim() ? withNc(v.output.trim()) : `${stemOf(r)}-hesap.nc`;
    const kept = await ran.host.keep(file, name, r.width, r.height, 'netcdf');
    if (kept.note) feedback.info(kept.note);
    const out = JSON.parse(ran.result.output ?? '{}') as { variable: string; mesh: string; steps: { values: number[]; time: boolean } | null };
    const target = v.layer as TargetLayer | null | undefined;
    const add: NewEntity[] = [];
    if (v.add !== false && target) {
      const { min: _min, max: _max, ...rest } = r.style;
      const style: RasterStyle = { ...rest, stretch: 'minMax' };
      const dims = out.steps ? [{ name: 'time', index: 0, values: out.steps.values, ...(out.steps.time ? { time: true } : {}) }] : [];
      const follow = d.followTime === true && dims.some((x) => 'time' in x && x.time);
      add.push({
        kind: 'raster',
        layerId: target.id,
        attrs: {},
        affine: [...r.affine] as RasterEntity['affine'],
        width: r.width,
        height: r.height,
        bands: r.bands,
        sample: 'f32',
        ...(kept.asset ? { asset: kept.asset } : { file: kept.file ?? name }),
        srid: r.srid,
        ...(r.opacity !== undefined ? { opacity: r.opacity } : {}),
        style,
        dataset: { variable: out.variable, mesh: out.mesh, ...(dims.length ? { dims } : {}), ...(follow ? { followTime: true } : {}) },
      } satisfies Omit<RasterEntity, 'id' | 'uid'>);
    }
    return {
      changes: add.length ? { add } : undefined,
      outputs: { file: name },
      summary: `${ran.done.summary} “${name}” yazıldı.`,
      above: r.layerId,
    };
  },
});

export const MULTIDIM_TOOLS = [profile, series, meshCalculator];
