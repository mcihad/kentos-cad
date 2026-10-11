import { fixed } from '../../../core/displayNumber';
import { convertVertices } from '../../../model/ops/convert';
import type { NewEntity } from '../../../model/entities';
import { defineTool } from '../../types';
import { CONVERT_SCOPES, outputLayer, zsOf } from './shared';

/**
 * Köşeleri noktaya çevir (docs/adr/0218 §3; ArcGIS Feature Vertices To Points, QGIS Extract vertices): every vertex,
 * the start, the end, both ends or the middle of each object as points with its attributes and Köşe (its number in
 * the object), Parça, Halka (0 the outer ring or the path, 1 … holes) and Uzaklık (along its ring or path from its
 * start, the project's length decimals); a vertex's elevation is the point's.
 */

export const vertices = defineTool({
  id: 'convert.vertices',
  label: 'Köşeleri noktaya çevir',
  category: 'conversion',
  icon: 'verticesToPoints',
  description: 'Çizgilerin ve alanların köşelerini (ya da yalnız uçlarını, ortalarını) öznitelikleriyle yeni katmana nokta olarak yazar.',
  help: [
    'Örnekler: parsel köşelerini noktaya çevirip numaralamak, bir güzergâhın başlangıç ve bitiş noktalarını almak, hatların ortalarına etiket noktası koymak.',
    'Her nokta nesnenin özniteliklerine ek Köşe (nesnedeki sırası), Parça, Halka (0 dış halka ya da çizgi, 1 … delikler) ve Uzaklık (halkanın ya da yolun başından yol boyunca, m) alır; köşenin kotu noktanın kotudur.',
    'Kapalı halkanın ilk köşesi bir kez yazılır; başlangıcı ve bitişi ilk köşesidir. Orta, yolun (alanda dış halkanın) yarısındaki noktadır; köşe olmadığından Köşe yazılmaz.',
  ].join('\n\n'),
  keywords: ['köşe', 'nokta', 'uç', 'başlangıç', 'bitiş', 'orta', 'vertices', 'extract vertices', 'feature vertices to points'],
  aliases: ['KOSELERINOKTA', 'KOSENOKTA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['line', 'polyline', 'polygon', 'arc'], scopes: CONVERT_SCOPES, description: 'Köşeleri alınacak çizgiler ve alanlar.' },
    {
      name: 'which',
      label: 'Köşeler',
      type: 'enum',
      options: [
        { value: 'all', label: 'Bütün köşeler' },
        { value: 'start', label: 'Başlangıç' },
        { value: 'end', label: 'Bitiş' },
        { value: 'ends', label: 'İki uç' },
        { value: 'middle', label: 'Orta', hint: 'Yolun yarısındaki nokta' },
      ],
      default: 'all',
    },
    outputLayer('Köşeler', '#0090FF'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Köşeler okunuyor');
    const found = convertVertices(inputs, inputs.map(zsOf), v.which);
    const ld = ctx.units.lengthDecimals;
    const add: NewEntity[] = [];
    found.forEach((list, i) => {
      for (const q of list) {
        const attrs: Record<string, string> = { ...inputs[i].attrs };
        if (q.index !== undefined) attrs['Köşe'] = String(q.index);
        attrs['Parça'] = String(q.part);
        attrs['Halka'] = String(q.ring);
        attrs['Uzaklık'] = fixed(q.distance, ld);
        add.push({ kind: 'point', layerId: v.layer.id, attrs, p: q.p, ...(q.z !== undefined && { z: q.z }) });
      }
    });
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} nesneden ${add.length} nokta yazıldı.`,
    };
  },
});
