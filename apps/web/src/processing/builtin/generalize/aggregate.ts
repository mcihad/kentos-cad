import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeAggregate } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { AREA_KINDS, inputNotes, newObject } from '../geometry/shared';
import { outputLayer } from '../convert/shared';
import { GENERALIZE_SCOPES, finish, measureParam } from './shared';

/**
 * Alanları kümele (docs/adr/0221 §5; ArcGIS Aggregate Polygons): areas no farther apart than the distance joined by
 * the closing's bridges, group by group; each result with how many objects it holds. The core's
 * `ops::generalize::aggregate`.
 */

export const aggregate = defineTool({
  id: 'generalize.aggregate',
  label: 'Alanları kümele',
  category: 'generalization',
  icon: 'aggregateAreas',
  description: 'Birbirine kümeleme uzaklığından yakın alanları aralarına köprü kurarak birleştirir (yapı adaları, orman ve göl kümeleri).',
  help: [
    'Örnekler: küçük ölçekte binaları yapı adalarına toplamak, dağınık orman lekelerini birleştirmek.',
    'Aralarındaki uzaklık kümeleme uzaklığını aşmayan alanlar bir kümedir. Kümenin birleşimi uzaklığın yarısı kadar dışa, sonra içe tamponlanır (kapanış); iki ya da daha çok nesneye değen dolgular köprü olur. Girintileri doldur açıkken tek nesnenin dar girintileri de dolar.',
    "Sonuç Nesne sayısı'nı ve Grupla'nın değerini alır; öbür öznitelikler alınmaz. En küçük boşluktan küçük delikler doldurulur, en küçük alandan küçük sonuçlar yazılmaz. Gruplar birbirleriyle birleşmez; kotlar taşınmaz.",
  ].join('\n\n'),
  keywords: ['kümele', 'birleştir', 'aggregate', 'yapı adası', 'köprü', 'genelleştir'],
  aliases: ['KUMELE', 'AGGREGATE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: AREA_KINDS, scopes: GENERALIZE_SCOPES, description: 'Kümelenecek alanlar.' },
    measureParam('distance', 'Kümeleme uzaklığı', 'm', 0.001, 10, 'Bu uzaklıktan yakın alanlar birleşir.'),
    measureParam('minArea', 'En küçük alan', 'm²', 0, 0, 'Bundan küçük sonuçlar yazılmaz.'),
    measureParam('minHole', 'En küçük boşluk', 'm²', 0, 0, 'Bundan küçük delikler doldurulur.'),
    { name: 'fillNotches', label: 'Girintileri doldur', type: 'boolean', default: false, description: 'Tek alanın dar girintileri de doldurulur.' },
    { name: 'group', label: 'Grupla', type: 'field', of: 'input', optional: true, description: 'Bu alanın değeri aynı olanlar birlikte kümelenir.' },
    outputLayer('Kümelenen alanlar', '#E5484D'),
  ] as const,
  outputs: [
    { name: 'aggregated', label: 'Yazılan alanlar', type: 'features' },
    { name: 'count', label: 'Yazılan alan sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    inputNotes(feedback, inputs);
    const group = v.group ?? '';
    const groups = inputs.map((e) => (group ? (e.attrs[group] ?? '') : ''));
    feedback.progress(0, 'Alanlar kümeleniyor');
    const got = generalizeAggregate(inputs.map(entityGeometry), groups, v.distance ?? 10, v.minArea ?? 0, v.minHole ?? 0, v.fillNotches);
    const add: NewEntity[] = got.shapes.map((shape, k) => {
      const attrs: Record<string, string> = { 'Nesne sayısı': String(got.members[k].length) };
      if (group) attrs[group] = got.groups[k];
      return newObject(shape, v.layer.id, attrs);
    });
    if (got.dropped) feedback.warn(`${got.dropped} kümelenmiş alan en küçük alandan küçük; yazılmadı.`);
    return finish(add, `${inputs.length} alandan ${add.length} kümelenmiş alan yazıldı.`, add.length);
  },
});
