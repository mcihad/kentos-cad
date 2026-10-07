import type { EntityKind } from '../../model/entities';
import type { SpatialRelation } from '../geometry';
import { defineTool, type Shown } from '../types';

/**
 * Konuma göre seç (docs/adr/0200 §2; QGIS "Select by location", Netcad Konumsal Seçim): selects the objects that
 * stand in a relation to any of the reference objects (Ayrık: to none), combined with the current selection as the
 * user chose. The relations are the core's (`ops::spatial_query`): the run's store pairs the inputs with the
 * references. Edits nothing, so there is nothing to undo.
 */

/** Every kind a query reads: construction lines reach everywhere and stand in no relation. */
export const QUERY_KINDS: readonly EntityKind[] = ['point', 'line', 'polyline', 'polygon', 'circle', 'arc', 'ellipse', 'spline', 'text', 'dimension', 'hatch', 'insert', 'leader', 'table', 'image'];

/** The kinds that can enclose: a closed area, a circle, a whole ellipse, a closed curve, a hatch's region. */
export const AREA_KINDS: readonly EntityKind[] = ['polygon', 'circle', 'ellipse', 'spline', 'hatch'];

/** The relations as the dialog and the summary name them. */
export const RELATION_LABEL: Record<SpatialRelation, string> = {
  intersects: 'Kesişen',
  contains: 'İçeren',
  within: 'İçinde kalan',
  disjoint: 'Ayrık',
  near: 'Uzaklıkta',
  centerIn: 'Merkezi içinde',
};

export const selectByLocation = defineTool({
  id: 'selection.byLocation',
  label: 'Konuma göre seç',
  category: 'selection',
  icon: 'selectLocation',
  description: 'Başka nesnelerle konum ilişkisi olan nesneleri seçer: kesişen, içeren, içinde kalan, ayrık, uzaklıkta ya da merkezi içinde.',
  help: [
    'Seçilecek nesneler başvuru nesnelerinden en az biriyle ilişkiyi sağlıyorsa seçilir; Ayrık, hiçbiriyle kesişmeyenleri seçer.',
    'Kesişen: ortak bir noktaları vardır (değmek de sayılır). İçeren: başvuru bütünüyle bu nesnenin alanının içindedir. İçinde kalan: nesne bütünüyle başvurunun alanının içindedir. Uzaklıkta: aradaki en kısa uzaklık verilen uzaklıktan büyük değildir. Merkezi içinde: nesnenin ağırlık merkezi ($merkez_y, $merkez_x) başvurunun alanının içinde ya da sınırındadır.',
    'Sınırda olmak ve değmek 1 mm içinde sayılır. Bir nesne kendisiyle karşılaştırılmaz.',
  ].join('\n\n'),
  keywords: ['konum', 'mekânsal', 'mekansal', 'sorgu', 'kesişen', 'içeren', 'içinde', 'ayrık', 'uzaklık', 'yakın', 'select by location', 'spatial', 'intersect', 'within', 'contains'],
  aliases: ['KONUMSEC', 'KONUMSALSECIM'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Seçilecek nesneler', type: 'features', kinds: QUERY_KINDS, scopes: ['all', 'visible', 'layer', 'selection'], default: { scope: 'all' }, description: 'İlişkisi denenen nesneler.' },
    {
      name: 'relation',
      label: 'İlişki',
      type: 'enum',
      options: [
        { value: 'intersects', label: RELATION_LABEL.intersects, hint: 'Ortak noktası olan' },
        { value: 'contains', label: RELATION_LABEL.contains, hint: 'Başvuruyu bütünüyle içine alan' },
        { value: 'within', label: RELATION_LABEL.within, hint: 'Bütünüyle başvurunun içinde olan' },
        { value: 'disjoint', label: RELATION_LABEL.disjoint, hint: 'Hiçbir başvuruyla kesişmeyen' },
        { value: 'near', label: RELATION_LABEL.near, hint: 'Verilen uzaklıktan yakın olan' },
        { value: 'centerIn', label: RELATION_LABEL.centerIn, hint: 'Ağırlık merkezi başvurunun içinde olan' },
      ],
      default: 'intersects',
    },
    { name: 'reference', label: 'Başvuru nesneleri', type: 'features', kinds: QUERY_KINDS, scopes: ['selection', 'layer', 'visible', 'all'], default: { scope: 'selection' }, description: 'İlişkinin karşı tarafı.' },
    { name: 'distance', label: 'Uzaklık', type: 'number', unit: 'm', min: 0, default: 10, visibleWhen: (v: Shown) => v.relation === 'near', description: 'Bu uzaklıktan yakın ya da tam bu uzaklıkta olanlar seçilir.' },
    {
      name: 'mode',
      label: 'Seçim biçimi',
      type: 'enum',
      options: [
        { value: 'new', label: 'Yeni seçim', hint: 'Mevcut seçim bırakılır' },
        { value: 'add', label: 'Seçime ekle', hint: 'İlişkiyi sağlayanlar mevcut seçime eklenir' },
        { value: 'remove', label: 'Seçimden çıkar', hint: 'İlişkiyi sağlayanlar mevcut seçimden çıkar' },
        { value: 'within', label: 'Seçim içinde ara', hint: 'Seçili olanlardan yalnızca ilişkiyi sağlayanlar kalır' },
      ],
      default: 'new',
    },
  ] as const,
  outputs: [
    { name: 'matched', label: 'İlişkiyi sağlayanlar', type: 'features' },
    { name: 'count', label: 'İlişkiyi sağlayan sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const refs = v.reference.entities;
    const relation = v.relation as SpatialRelation;
    feedback.progress(0, 'İlişkiler deneniyor');
    // Ayrık is asked as Kesişen: the inputs that meet no reference.
    const pairs = ctx.geometry.relatePairs(
      inputs.map((e) => e.id),
      refs.map((e) => e.id),
      relation === 'disjoint' ? 'intersects' : relation,
      relation === 'near' ? v.distance : 0,
    );
    const met = new Set(pairs.map(([i]) => i));
    const hits: number[] = [];
    inputs.forEach((e, i) => {
      if (met.has(i) !== (relation === 'disjoint')) hits.push(e.id);
    });
    const current = ctx.selection;
    const hit = new Set(hits);
    const select = v.mode === 'new' ? hits : v.mode === 'add' ? [...current, ...hits] : v.mode === 'remove' ? current.filter((id) => !hit.has(id)) : current.filter((id) => hit.has(id));
    return {
      select,
      outputs: { matched: hits, count: hits.length },
      summary: `${hits.length} / ${inputs.length} nesne “${RELATION_LABEL[relation]}” ilişkisini sağladı; seçimde ${new Set(select).size} nesne var.`,
    };
  },
});
