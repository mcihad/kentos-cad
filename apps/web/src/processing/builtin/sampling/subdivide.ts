import { entityGeometry, type NewEntity } from '../../../model/entities';
import { sampleSubdivide } from '../../../model/ops/sampling';
import { defineTool } from '../../types';
import { outputLayer, placed, zsOf } from '../convert/shared';
import { SAMPLE_SCOPES, written } from './shared';

/**
 * Alt bölümlere ayır (docs/adr/0220 §6; PostGIS ST_Subdivide, QGIS Subdivide): areas and polylines cut into pieces of
 * at most so many vertices, each piece with its object's attributes, its number and its elevations, from the core
 * (`ops::sampling::subdivide`, curved geometry cut exactly).
 */

export const subdivide = defineTool({
  id: 'sample.subdivide',
  label: 'Alt bölümlere ayır',
  category: 'sampling',
  icon: 'subdivideParts',
  description: 'Çok köşeli alanları ve çoklu çizgileri en çok verilen köşe sayısı olan parçalara ayırıp yeni katmana yazar.',
  help: [
    'Örnekler: binlerce köşeli bir il ya da havza sınırını hızlı sorgulanan parçalara ayırmak, uzun eş yükselti eğrilerini bölmek.',
    'Parça köşe sayısını aşıyorsa kutusunun uzun kenarına dik ikiye kesilir; kesim halkanın kutunun ortasına en yakın köşesinden geçer (delikler köşelerin çoğunu tutuyorsa en büyük deliğinkinden). Parçalar yeniden ele alınır, en çok 50 kat. Yaylar kesin kesilir.',
    'Her parça nesnenin özniteliklerini ve Parça numarasını (kutusunun sol alt köşesine göre: önce batı, sonra güney) alır; köşelerin kotları taşınır. Köşe sayısını aşmayan nesne tek parça yazılır.',
  ].join('\n\n'),
  keywords: ['alt bölüm', 'parçala', 'köşe', 'subdivide', 'dizin', 'bölme', 'parça'],
  aliases: ['ALTBOLUM', 'SUBDIVIDE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['polygon', 'polyline'], scopes: SAMPLE_SCOPES, description: 'Ayrılacak alanlar ve çoklu çizgiler.' },
    {
      name: 'max',
      label: 'En çok köşe',
      type: 'number',
      integer: true,
      min: 5,
      default: 256,
      description: 'Bir parçanın en çok köşe sayısı (halkaların ve deliklerin köşeleri birlikte).',
    },
    outputLayer('Alt bölümler', '#30A46C'),
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Yazılan parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Parçalar kesiliyor');
    const lists = sampleSubdivide(inputs.map(entityGeometry), inputs.map(zsOf), v.max ?? 256);
    const add: NewEntity[] = [];
    let whole = 0;
    lists.forEach((pieces, i) => {
      if (pieces.length === 1) whole++;
      pieces.forEach((r, k) => add.push(placed(r, v.layer.id, { ...inputs[i].attrs, Parça: String(k + 1) })));
    });
    if (whole) feedback.info(`${whole} nesne en çok köşeyi aşmadığı için tek parça yazıldı.`);
    return written(add, `${inputs.length} nesneden ${add.length} parça yazıldı.`);
  },
});
