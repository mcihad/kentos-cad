import { CRS_REGISTRY, crsBySrid, crsCode, crsTitle } from '../../../geo/crs';
import { systemOf } from '../../../model/geom/crsTransform';
import { geoReproject, type Unreached } from '../../../model/ops/geoprocess';
import { defineTool } from '../../types';
import { GEO_KINDS, GEO_SCOPES, inputNotes, newObject, outputStyle } from './shared';

/**
 * Koordinat sistemine dönüştür (docs/adr/0201 §8; ArcGIS Project, QGIS Katmanı yeniden izdüşür): objects whose
 * coordinates are in another system (data brought in without converting it) moved vertex by vertex into the
 * project's, the project's datum choices taken (`crs::transform_in`, docs/adr/0168); arcs and circles first become
 * chords within 1 mm, since a projection does not keep a circle a circle. A point keeps its elevation. An object with a
 * vertex that has no value in the project's system is not written, and the reason is said.
 */

/** The source systems: the registry's but the local one, as a sentence names them. */
const SOURCES = CRS_REGISTRY.filter((c) => c.kind !== 'local').map((c) => ({ value: String(c.srid), label: crsTitle(c), hint: c.area }));

/** Why an object was not written, as the note says it. */
const REASON: Record<Unreached, string> = {
  outside: 'izdüşümün dışında',
  noLink: 'datumlar arasında yol yok',
  noGrid: 'datum seçiminin ızgarası bu cihazda yok',
  outsideGrid: 'ızgaranın dışında',
};

export const geometryReproject = defineTool({
  id: 'geometry.reproject',
  label: 'Koordinat sistemine dönüştür',
  category: 'geometry',
  icon: 'geoReproject',
  description: 'Koordinatları başka bir sistemde olan nesneleri projenin koordinat sistemine dönüştürüp yeni katmana yazar.',
  help: [
    'Kaynak sistem, nesnelerin koordinatlarının bugün hangi sistemde olduğudur (örneğin ED50 / TM30 ile alınmış eski bir pafta); hedef her zaman projenin sistemidir. Projenin datum seçimleri (Proje ayarları › Datum dönüşümleri) uygulanır.',
    'Köşeler tek tek dönüştürülür. Yay ve daire kenarları önce 1 mm içinde doğru parçalarına çevrilir, çünkü izdüşüm daireyi daire olarak korumaz. Noktanın kotu korunur; çizgi ve alanların köşe kotları taşınmaz.',
    'Bir köşesi projenin sisteminde değeri olmayan nesne (dilimin çok dışında, datumlar arasında yol ya da ızgara yok) yazılmaz ve nedeniyle söylenir. Projenin koordinat sistemi yoksa araç çalışmaz.',
  ].join('\n\n'),
  keywords: ['koordinat sistemi', 'dönüştür', 'izdüşüm', 'datum', 'reproject', 'project', 'ED50', 'TUREF', 'WGS 84', 'transform'],
  aliases: ['IZDUSUMDEGISTIR', 'REPROJECT'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Koordinatları kaynak sistemde olan alanlar, çizgiler ve noktalar.' },
    { name: 'source', label: 'Kaynak sistem', type: 'enum', options: SOURCES, default: '4326', description: 'Nesnelerin koordinatlarının sistemi; hedef projenin sistemidir.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Dönüştürülen' }, newLayerStyle: outputStyle('#AB4ABA'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'moved', label: 'Dönüştürülen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const crs = ctx.crs;
    if (!crs) return { refused: "Projenin koordinat sistemi yok; önce Proje ayarları'nda sistem seçin." };
    const src = crsBySrid(Number(v.source));
    const from = src ? systemOf(src) : null;
    if (!src || !from) return { refused: `Kaynak sistem bilinmiyor: ${v.source}.` };
    if (crs.srid === src.srid) return { refused: 'Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok.' };
    const input = v.input.entities;
    inputNotes(feedback, input, [], true);
    feedback.progress(0, 'Köşeler dönüştürülüyor');
    const results = geoReproject(input, from, crs.system, crs.choices);
    const add = results.flatMap((r, i) => (r.shape ? [newObject(r.shape, v.layer.id, { ...input[i].attrs })] : []));
    const arcs = results.filter((r) => r.shape && r.chorded > 0).length;
    if (arcs) feedback.warn(`${arcs} nesnenin yayları 1 mm içinde doğru parçalarına çevrildi.`);
    for (const why of Object.keys(REASON) as Unreached[]) {
      const n = results.filter((r) => r.error === why).length;
      if (n) feedback.warn(`${n} nesnenin bir köşesi dönüştürülemedi (${REASON[why]}); yazılmadı.`);
    }
    return { changes: { add }, outputs: { count: add.length }, summary: `${add.length} nesne dönüştürüldü: ${crsCode(src)} → ${crs.code}.` };
  },
});
