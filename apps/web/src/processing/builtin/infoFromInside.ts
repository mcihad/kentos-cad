import type { Entity } from '../../model/entities';
import { statisticMany, type StatKind } from '../../model/ops/statistics';
import type { SpatialRelation } from '../geometry';
import { defineTool, type Shown } from '../types';
import { meanScale, withAttr } from './attributeWrites';
import { AREA_KINDS, QUERY_KINDS, RELATION_LABEL } from './selectByLocation';

/**
 * İçindekinden bilgi al (docs/adr/0200 §3; Netcad's İçindekinden Bilgi Al, QGIS "Join attributes by location
 * (summary)"): each target area gets a statistic of the source objects in the relation to it (in it, meeting it, or
 * with their centre in it): how many, the sum, mean, least or most of a field, or the first value. The pairs come from
 * the run's store, the numbers from the core's `ops::statistics`; one undo step.
 */

/** The statistics as the dialog and the summary name them. */
export const STAT_LABEL: Record<StatKind, string> = { count: 'sayı', sum: 'toplam', mean: 'ortalama', min: 'en az', max: 'en çok', first: 'ilk değer' };

export const infoFromInside = defineTool({
  id: 'attributes.fromInside',
  label: 'İçindekinden bilgi al',
  category: 'attributes',
  icon: 'infoInside',
  description: 'Her alana içindeki ya da ona değen nesnelerin sayısını ya da bir alanlarının toplamını, ortalamasını, en azını, en çoğunu yazar.',
  help: [
    'Örnekler: parsellere içlerindeki ağaçların sayısı; adalara yapıların taban alanlarının toplamı; mahallelere içindeki yapıların kat ortalaması.',
    'İlişki kaynağın hedefe göre durumudur: İçinde kalan bütünüyle içeride olanları, Kesişen alana değen ya da onu kesenleri, Merkezi içinde ağırlık merkezi içeride olanları alır. Sınır 1 mm içinde sayılır.',
    'Toplam, en az ve en çok kesindir; ortalama yazılacak alan ondalık sayıysa onun basamağına, değilse değerlerin en çok basamağından iki fazlasına yarım çifte yuvarlanır. Sayı olarak okunamayan değerler atlanır ve söylenir. Hiçbir kaynak yoksa Sayı 0 yazar, öbürleri alanı boşaltır.',
  ].join('\n\n'),
  keywords: ['içindeki', 'içinde', 'sayı', 'toplam', 'ortalama', 'mekânsal birleştir', 'özet', 'spatial join', 'count points in polygon', 'summary'],
  aliases: ['ICBILGI', 'ICINDEKIBILGI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'target', label: 'Hedef alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Bilginin yazılacağı alanlar; kilitli katmandakiler alınmaz.' },
    { name: 'source', label: 'Kaynak nesneler', type: 'features', kinds: QUERY_KINDS, scopes: ['layer', 'all', 'visible', 'selection'], description: 'Sayılan ya da değerleri alınan nesneler.' },
    {
      name: 'relation',
      label: 'İlişki',
      type: 'enum',
      options: [
        { value: 'within', label: RELATION_LABEL.within, hint: 'Bütünüyle alanın içinde olanlar' },
        { value: 'intersects', label: RELATION_LABEL.intersects, hint: 'Alana değen ya da onu kesenler' },
        { value: 'centerIn', label: RELATION_LABEL.centerIn, hint: 'Ağırlık merkezi alanın içinde olanlar' },
      ],
      default: 'within',
    },
    {
      name: 'stat',
      label: 'İstatistik',
      type: 'enum',
      options: [
        { value: 'count', label: 'Sayı', hint: 'Kaynak nesnelerin sayısı' },
        { value: 'sum', label: 'Toplam', hint: 'Alanın değerlerinin toplamı' },
        { value: 'mean', label: 'Ortalama', hint: 'Alanın değerlerinin ortalaması' },
        { value: 'min', label: 'En az', hint: 'Alanın en küçük değeri' },
        { value: 'max', label: 'En çok', hint: 'Alanın en büyük değeri' },
        { value: 'first', label: 'İlk değer', hint: 'Çizim sırasıyla ilk dolu değer, olduğu gibi' },
      ],
      default: 'count',
    },
    { name: 'field', label: 'Alan', type: 'field', of: 'source', visibleWhen: (v: Shown) => v.stat !== 'count', description: 'Kaynak nesnelerin değeri alınan alanı.' },
    { name: 'output', label: 'Yazılacak alan', type: 'field', of: 'target', allowNew: true, default: 'Nesne sayısı', description: 'Listeden var olan bir alanı seçin ya da yeni bir ad yazın.' },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  preview: (v) => (v.output.trim() ? `“${v.output.trim()}” alanına ${STAT_LABEL[v.stat as StatKind]} yazılacak` : null),
  run: async (v, ctx, feedback) => {
    const targets = v.target.entities;
    const sources = v.source.entities;
    const stat = v.stat as StatKind;
    const field = stat === 'count' ? '' : v.field;
    const output = v.output;
    feedback.progress(0, 'İlişkiler deneniyor');
    const pairs = ctx.geometry.relatePairs(
      sources.map((e) => e.id),
      targets.map((e) => e.id),
      v.relation as Exclude<SpatialRelation, 'disjoint'>,
      0,
    );
    // Each target's sources in the drawing's order: the pairs come source by source.
    const members: Entity[][] = targets.map(() => []);
    for (const [i, j] of pairs) members[j].push(sources[i]);
    feedback.progress(0.5, 'Değerler hesaplanıyor');
    const groups = members.map((list) => list.map((s) => (field && Object.hasOwn(s.attrs, field) ? s.attrs[field] : null)));
    const scales = targets.map((t) => (stat === 'mean' ? meanScale(ctx, t, output) : null));
    let values: ReturnType<typeof statisticMany>;
    try {
      values = statisticMany(groups, stat, scales);
    } catch (e) {
      // Numbers too large or too long to add exactly: said as the core says it.
      return { refused: e instanceof Error ? e.message : String(e) };
    }
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let skipped = 0;
    targets.forEach((t, k) => {
      skipped += values[k].skipped;
      const attrs = withAttr(ctx, t, output, values[k].value ?? null);
      if (attrs) update.push({ id: t.id, patch: { attrs } });
    });
    if (skipped) feedback.warn(`${skipped} değer sayı olarak okunamadığı için atlandı.`);
    return {
      changes: { update },
      outputs: { changed: update.map((u) => u.id), count: update.length },
      summary: `${update.length} hedef nesneye “${output}” yazıldı (${STAT_LABEL[stat]}).`,
    };
  },
});
