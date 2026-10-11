import { fixed } from '../../../core/displayNumber';
import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeEliminate } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { AREA_KINDS, inputNotes, newObject } from '../geometry/shared';
import { outputLayer } from '../convert/shared';
import { GENERALIZE_SCOPES, finish, measureParam, table } from './shared';

/**
 * Küçükleri komşuya kat (docs/adr/0221 §7; ArcGIS's and QGIS's Eliminate): small areas joined to the large neighbour
 * they share the longest boundary with, or the largest; the report says each joining. The core's
 * `ops::generalize::eliminate`.
 */

const COLUMNS = ['Küçük alan', 'Katıldığı', 'Ortak sınır (m)', 'Alan (m²)'];

export const eliminate = defineTool({
  id: 'generalize.eliminate',
  label: 'Küçükleri komşuya kat',
  category: 'generalization',
  icon: 'eliminateSmall',
  description: 'Küçük alanları en uzun ortak sınırlı ya da en büyük komşusuna katar (kırıntı parseller, sayısallaştırma artıkları).',
  help: [
    'Örnekler: arazi örtüsü verisindeki küçük kırıntıları komşularına katmak, en küçük haritalama birimini sağlamak.',
    "Net alanı En küçük alan'dan küçük alanlar küçüktür. Küçükler küçükten büyüğe, ortak sınırı (1 mm içinde) olan büyük komşulardan seçilene katılır; yalnız küçüklere değen küçük, onlar katıldıktan sonra katılır. Komşusu olmayan küçük olduğu gibi yazılır ve söylenir.",
    'Büyükler öznitelikleriyle yazılır; tablo her katılmayı söyler. Kotlar taşınmaz.',
  ].join('\n\n'),
  keywords: ['kat', 'küçük', 'eliminate', 'kırıntı', 'birleştir', 'komşu'],
  aliases: ['KUCUKKAT', 'ELIMINATE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: AREA_KINDS, scopes: GENERALIZE_SCOPES, description: 'Küçükleri komşularına katılacak alanlar.' },
    measureParam('minArea', 'En küçük alan', 'm²', 0, 100, 'Bundan küçük alanlar komşusuna katılır.'),
    {
      name: 'into',
      label: 'Kime',
      type: 'enum',
      options: [
        { value: 'longest', label: 'En uzun ortak sınır', hint: 'Ortak sınırı en uzun komşuya' },
        { value: 'largest', label: 'En büyük komşu', hint: 'Alanı en büyük komşuya' },
      ],
      default: 'longest',
    },
    outputLayer('Katılan', '#F76B15'),
  ] as const,
  outputs: [
    { name: 'result', label: 'Yazılan alanlar', type: 'features' },
    { name: 'report', label: 'Katılma raporu', type: 'table' },
    { name: 'count', label: 'Katılan alan sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    inputNotes(feedback, inputs);
    feedback.progress(0, 'Küçük alanlar katılıyor');
    const geometries = inputs.map(entityGeometry);
    const got = generalizeEliminate(geometries, v.minArea ?? 100, v.into === 'largest');
    const gone = new Set(got.small);
    const changed = new Map(got.changed.map((i, k) => [i, got.shapes[k]]));
    const add: NewEntity[] = [];
    inputs.forEach((e, i) => {
      if (gone.has(i)) return;
      const shape = changed.has(i) ? changed.get(i) : geometries[i];
      if (shape) add.push(newObject(shape, v.layer.id, { ...e.attrs }));
    });
    const { lengthDecimals: ld, areaDecimals: ad } = ctx.units;
    const rows = got.small.map((s, k) => [`#${inputs[s].id}`, `#${inputs[got.into[k]].id}`, fixed(got.shared[k], ld), fixed(got.smallArea[k], ad)]);
    if (got.left.length) feedback.warn(`${got.left.length} küçük alanın büyük komşusu yok; olduğu gibi yazıldı.`);
    const summary = rows.length
      ? `${add.length} nesne yazıldı; ${rows.length} küçük alan komşusuna katıldı.`
      : `${add.length} nesne yazıldı; katılacak küçük alan yok.`;
    return finish(add, summary, rows.length, table(COLUMNS, rows));
  },
});
