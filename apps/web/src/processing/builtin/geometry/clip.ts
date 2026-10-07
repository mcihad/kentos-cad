import { geoClip } from '../../../model/ops/geoprocess';
import { defineTool } from '../../types';
import { AREA_KINDS, GEO_KINDS, GEO_SCOPES, emptyNote, inputNotes, newObject, outputStyle } from './shared';

/**
 * Kırp (docs/adr/0201 §3; ArcGIS Clip, QGIS Kırp): each object's parts inside the cutting areas, which are joined
 * first; an area meets them in the core's overlay, a path is cut where it crosses their boundary and keeps the pieces
 * inside or on it, a point stays when it is inside or on it. The objects keep their attributes.
 */
export const geometryClip = defineTool({
  id: 'geometry.clip',
  label: 'Kırp',
  category: 'geometry',
  icon: 'geoClip',
  description: 'Nesnelerin kesen alanların içinde kalan parçalarını yeni katmana yazar; öznitelikler olduğu gibi kalır.',
  help: [
    'Kesen alanlar önce birleşir; her nesnenin onların içinde ya da sınırında kalan kısmı yazılır: alanın ortak kısmı, çizginin içerideki parçaları, içerideki noktalar. Sınır üstündeki çizgi parçası içeride sayılır.',
    "Bir nesnenin birden çok parçası kalırsa sonuç tek, çok parçalı nesnedir. Kesen alanların dışında kalan nesne yazılmaz ve söylenir. Kesen alanların öznitelikleri alınmaz; onları da isteyen Kesişim'i kullanır.",
  ].join('\n\n'),
  keywords: ['kırp', 'kes', 'clip', 'sınırla', 'pafta', 'çalışma alanı', 'kesen alan'],
  aliases: ['KIRP', 'CLIP'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Kesilecek alanlar, çizgiler ve noktalar.' },
    { name: 'cut', label: 'Kesen alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['selection', 'layer', 'visible', 'all'], default: { scope: 'selection' }, description: 'Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler; birleşerek keser.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Kırpılan' }, newLayerStyle: outputStyle('#30A46C'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'clipped', label: 'Kırpılan nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const input = v.input.entities;
    inputNotes(feedback, input, v.cut.entities);
    feedback.progress(0, 'Nesneler kırpılıyor');
    const shapes = geoClip(input, v.cut.entities);
    const add = shapes.flatMap((s, i) => (s ? [newObject(s, v.layer.id, { ...input[i].attrs })] : []));
    emptyNote(input.length - add.length, feedback);
    return { changes: { add }, outputs: { count: add.length }, summary: `${add.length} nesne kırpılarak yazıldı.` };
  },
});
