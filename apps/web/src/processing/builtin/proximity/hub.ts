import { defineTool } from '../../types';
import { newObject, outputStyle } from '../geometry/shared';
import { PROXIMITY_KINDS, PROXIMITY_SCOPES, bound, lengthText, nameOf } from './shared';

/**
 * En yakın merkeze bağla (docs/adr/0215 §3.3; QGIS "Distance to nearest hub (line to hub)"): each object's centre
 * joined to the nearest hub's by a line on the output layer, the line carrying the object's attributes, the hub's name
 * (Merkez) and the distance (Uzaklık). Centre to centre, by the run's store (`Store::nearest`).
 */
export const proximityHub = defineTool({
  id: 'proximity.hub',
  label: 'En yakın merkeze bağla',
  category: 'proximity',
  icon: 'nearestHub',
  description: 'Her nesnenin merkezini en yakın merkezin merkezine bir çizgiyle bağlar; çizgi merkezin adını ve uzaklığı taşır.',
  help: [
    'Örnekler: her mahalleyi en yakın sağlık ocağına bağlayan çizgiler; her kuyudan en yakın depoya; her parselden en yakın okula.',
    'Uzaklık merkezden merkezedir: alanın ağırlık merkezi, öbür nesnelerin yer noktası. Eşit uzaklıkta çizimde önce gelen merkez alınır. Çizgi nesnenin özniteliklerini, “Merkez”i (merkezin Ad alanındaki değeri; boşsa etiketi, o da yoksa sırası) ve “Uzaklık”ı taşır.',
    'En çok uzaklık içinde merkezi olmayan nesne bağlanmaz ve sayısı söylenir; merkezi merkezle aynı yerde olan nesneye çizgi çizilmez.',
  ].join('\n\n'),
  keywords: ['en yakın merkez', 'hub', 'nearest hub', 'bağla', 'örümcek diyagramı', 'spider', 'yakınlık', 'tesis'],
  aliases: ['MERKEZEBAGLA', 'ENYAKINMERKEZ'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Merkeze bağlanacak nesneler.' },
    { name: 'hubs', label: 'Merkezler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Okullar, sağlık ocakları, depolar …' },
    { name: 'hubName', label: 'Merkez ad alanı', type: 'field', of: 'hubs', optional: true, description: 'Çizgilere “Merkez” olarak yazılır; boşsa etiket, o da yoksa sıra.' },
    { name: 'max', label: 'En çok uzaklık', type: 'number', unit: 'm', min: 0, default: 0, description: '0: sınırsız.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Merkeze bağlantılar' }, newLayerStyle: outputStyle('#E5732E'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'lines', label: 'Bağlantılar', type: 'features' },
    { name: 'count', label: 'Bağlantı sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const hubs = v.hubs.entities;
    feedback.progress(0, 'En yakın merkezler aranıyor');
    const found = ctx.geometry.nearest(
      inputs.map((e) => e.id),
      hubs.map((e) => e.id),
      1,
      bound(v.max),
      'centers',
    );
    let same = 0;
    const add = found.flatMap((f) => {
      if (f.d === 0) {
        same++;
        return [];
      }
      const attrs = { ...inputs[f.input].attrs, Merkez: nameOf(hubs[f.target], v.hubName ?? '', f.target), Uzaklık: lengthText(ctx.units, f.d) };
      return [newObject({ kind: 'line', a: f.a, b: f.b }, v.layer.id, attrs)];
    });
    const none = inputs.length - found.length;
    if (none) feedback.info(`${none} nesnenin ${v.max > 0 ? 'en çok uzaklık içinde ' : ''}merkezi yok; bağlanmadı.`);
    if (same) feedback.info(`${same} nesnenin merkezi merkezle aynı yerde; çizgi çizilmedi.`);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${add.length} nesne en yakın merkeze bağlandı.`,
    };
  },
});
