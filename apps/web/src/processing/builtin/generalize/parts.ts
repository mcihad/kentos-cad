import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeParts } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { outputLayer, placed, zsOf } from '../convert/shared';
import { GENERALIZE_SCOPES, finish, measureParam } from './shared';

/**
 * Küçük parçaları ayıkla (docs/adr/0221 §6; ArcGIS Eliminate Polygon Part): the small parts and holes of areas gone,
 * the largest part kept when all are small; what stays unchanged, elevations too. The core's `ops::generalize::parts`.
 */

export const dropParts = defineTool({
  id: 'generalize.dropParts',
  label: 'Küçük parçaları ayıkla',
  category: 'generalization',
  icon: 'dropParts',
  description: 'Çok parçalı alanların küçük parçalarını ve küçük delikleri atar; bütün parçaları küçükse en büyüğü kalır.',
  help: [
    'Örnekler: sınır verisindeki kırıntı adaları ve iğne deliklerini temizlemek, sayısallaştırma artıklarını atmak.',
    "Net alanı En küçük alan'dan küçük ya da (Yüzde verilmişse) nesnenin dış halkalarının alanı toplamının yüzdesinden küçük parça ve delikler gider. Parçalar ve Delikler ayrı ayrı açılıp kapanır. Kalan halkalar köşeleri ve kotlarıyla değişmez.",
  ].join('\n\n'),
  keywords: ['parça', 'delik', 'ayıkla', 'eliminate polygon part', 'kırıntı', 'temizle'],
  aliases: ['PARCAAYIKLA', 'ELIMINATEPART'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: ['polygon'], scopes: GENERALIZE_SCOPES, description: 'Küçük parçaları ve delikleri atılacak alanlar.' },
    measureParam('minArea', 'En küçük alan', 'm²', 0, 10, 'Bundan küçük parça ve delikler atılır.'),
    { name: 'percent', label: 'Yüzde', type: 'number', unit: '%', min: 0, max: 100, default: 0, description: 'Dış halkaların alanı toplamının bu yüzdesinden küçükler de atılır; 0: kullanılmaz.' },
    { name: 'parts', label: 'Parçalar', type: 'boolean', default: true, description: 'Küçük parçalar atılır.' },
    { name: 'holes', label: 'Delikler', type: 'boolean', default: true, description: 'Küçük delikler doldurulur.' },
    outputLayer('Ayıklanan', '#12A594'),
  ] as const,
  outputs: [
    { name: 'trimmed', label: 'Yazılan alanlar', type: 'features' },
    { name: 'count', label: 'Atılan parça ve delik sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Parçalar ayıklanıyor');
    // Only areas with holes or more parts than one can lose any; the core takes those, the others are written as they are.
    const trimmable = inputs.filter((e) => e.kind === 'polygon' && (e.holes?.length || e.parts?.length));
    const results = generalizeParts(trimmable.map(entityGeometry), trimmable.map(zsOf), v.minArea ?? 10, v.percent ?? 0, v.parts, v.holes);
    const got = new Map(trimmable.map((e, k) => [e, results[k]]));
    const add: NewEntity[] = [];
    let parts = 0;
    let holes = 0;
    for (const e of inputs) {
      if (e.kind !== 'polygon') continue;
      const r = got.get(e);
      if (r?.shape) {
        parts += r.parts;
        holes += r.holes;
        add.push(placed({ shape: r.shape, zs: r.zs ?? undefined }, v.layer.id, { ...e.attrs }));
      } else {
        add.push(placed({ shape: entityGeometry(e), zs: zsOf(e)?.map((z) => [...z]) }, v.layer.id, { ...e.attrs }));
      }
    }
    const summary =
      parts + holes ? `${add.length} nesne yazıldı; ${parts} parça ve ${holes} delik atıldı.` : `${add.length} nesne yazıldı; atılacak küçük parça ya da delik yok.`;
    return finish(add, summary, parts + holes);
  },
});
