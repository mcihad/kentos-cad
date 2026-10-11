import { convertPoints } from '../../../model/ops/convert';
import type { NewEntity } from '../../../model/entities';
import { defineTool } from '../../types';
import { hasElevation } from '../geometry/shared';
import { CONVERT_SCOPES, outputLayer } from './shared';

/**
 * Nesneden nokta (docs/adr/0218 §2; ArcGIS Feature To Point, QGIS Centroids and Point on surface): each object's
 * centroid or interior point (on it always), each part's with Her parça için, a point on the output layer with the
 * object's attributes. The points are the core's (`ops::convert`); they have no elevation.
 */

export const toPoints = defineTool({
  id: 'convert.toPoints',
  label: 'Nesneden nokta',
  category: 'conversion',
  icon: 'featureToPoint',
  description: 'Her nesnenin ağırlık merkezini ya da üstündeki bir iç noktayı, öznitelikleriyle yeni katmana nokta olarak yazar.',
  help: [
    'Örnekler: parsellerin merkezlerine numara noktaları koymak, yapıları nokta katmanına çevirmek, yolların orta noktalarını almak.',
    'Ağırlık merkezi alanda alanın merkezidir (delikler çıkar, yaylı kenarlar kesin), çizgide uzunlukla ağırlıklı merkezdir; L biçimli bir alanda ya da kıvrık bir çizgide nesnenin dışında kalabilir. İç nokta her zaman nesnenin üstündedir: alanda yatay bir doğrunun alanın içinde kalan en geniş aralığının ortası (GEOS’un kuralı), çizgide yolun yarısı.',
    'Her parça için açıkken çok parçalı nesnenin her parçası bir nokta olur, Parça alanı 1, 2 … yazılır. Noktaların kotu yoktur.',
  ].join('\n\n'),
  keywords: ['nokta', 'merkez', 'ağırlık merkezi', 'iç nokta', 'centroid', 'point on surface', 'feature to point'],
  aliases: ['NESNEDENNOKTA', 'MERKEZNOKTA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['point', 'line', 'polyline', 'polygon', 'circle', 'arc'], scopes: CONVERT_SCOPES, description: 'Noktası alınacak nesneler.' },
    {
      name: 'location',
      label: 'Konum',
      type: 'enum',
      options: [
        { value: 'centroid', label: 'Ağırlık merkezi', hint: 'Nesnenin dışında kalabilir' },
        { value: 'inside', label: 'İç nokta', hint: 'Her zaman nesnenin üstünde' },
      ],
      default: 'centroid',
    },
    { name: 'parts', label: 'Her parça için', type: 'boolean', default: false, description: 'Çok parçalı nesnenin her parçası bir nokta olur.' },
    outputLayer('Merkezler', '#E54666'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    const heights = inputs.filter(hasElevation).length;
    if (heights) feedback.warn(`${heights} nesnenin kotları sonuca taşınmadı.`);
    feedback.progress(0, 'Noktalar hesaplanıyor');
    const points = convertPoints(inputs, v.location === 'inside', v.parts);
    const add: NewEntity[] = [];
    points.forEach((list, i) => {
      list.forEach((p, k) => {
        const attrs = v.parts ? { ...inputs[i].attrs, Parça: String(k + 1) } : { ...inputs[i].attrs };
        add.push({ kind: 'point', layerId: v.layer.id, attrs, p });
      });
    });
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} nesneden ${add.length} nokta yazıldı.`,
    };
  },
});
