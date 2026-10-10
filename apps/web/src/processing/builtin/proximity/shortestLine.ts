import { defineTool } from '../../types';
import { newObject, outputStyle } from '../geometry/shared';
import { PROXIMITY_KINDS, PROXIMITY_SCOPES, bound, lengthText, nameOf } from './shared';

/**
 * En kısa çizgi (docs/adr/0215 §3.5; QGIS "Shortest line between features"): from each object to its nearest `k`
 * targets, the segment between their nearest points on the output layer, with Kaynak, Hedef, Sıra and Uzaklık. Pairs
 * that meet (0 apart) give none and are counted. The search is the run's store's (`Store::nearest`, edge to edge).
 */
export const proximityShortestLine = defineTool({
  id: 'proximity.shortestLine',
  label: 'En kısa çizgi',
  category: 'proximity',
  icon: 'shortestLine',
  description: 'Her nesneden en yakın k hedefe en yakın noktalar arası doğru parçaları çizer.',
  help: [
    'Örnekler: her yapıdan en yakın yola bağlantı çizgisi; her kuyudan en yakın iki dereye; iki katman arası en kısa bağlantılar.',
    'Çizgi iki nesnenin en yakın noktaları arasındadır (kenardan kenara); eşit uzaklıkta çizimde önce gelen hedef alınır. Çizgiye “Kaynak”, “Hedef” (Ad alanlarından; boşsa etiket, o da yoksa sıra), “Sıra” ve “Uzaklık” yazılır.',
    'Değen ya da kesişen çiftler (uzaklık 0) çizgi vermez; sayıları söylenir.',
  ].join('\n\n'),
  keywords: ['en kısa çizgi', 'shortest line', 'bağlantı', 'en yakın', 'yakınlık', 'dik bağlantı'],
  aliases: ['ENKISACIZGI', 'KISACIZGI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Çizgilerin başladığı nesneler.' },
    { name: 'targets', label: 'Hedefler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'Çizgilerin vardığı nesneler.' },
    { name: 'k', label: 'En yakın k', type: 'number', integer: true, min: 1, max: 100, default: 1, description: 'Her nesneden kaç hedefe.' },
    { name: 'max', label: 'En çok uzaklık', type: 'number', unit: 'm', min: 0, default: 0, description: '0: sınırsız.' },
    { name: 'name', label: 'Ad alanı', type: 'field', of: 'input', optional: true, description: 'Çizgilere “Kaynak” olarak yazılır.' },
    { name: 'targetName', label: 'Hedef ad alanı', type: 'field', of: 'targets', optional: true, description: 'Çizgilere “Hedef” olarak yazılır.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'En kısa çizgiler' }, newLayerStyle: outputStyle('#8A3FFC'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'lines', label: 'Çizgiler', type: 'features' },
    { name: 'count', label: 'Çizgi sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const targets = v.targets.entities;
    feedback.progress(0, 'En yakın noktalar aranıyor');
    const found = ctx.geometry.nearest(
      inputs.map((e) => e.id),
      targets.map((e) => e.id),
      v.k,
      bound(v.max),
      'edges',
    );
    let meeting = 0;
    let last = -1;
    let rank = 0;
    const add = found.flatMap((f) => {
      rank = f.input === last ? rank + 1 : 1;
      last = f.input;
      if (f.d === 0) {
        meeting++;
        return [];
      }
      const attrs = {
        Kaynak: nameOf(inputs[f.input], v.name ?? '', f.input),
        Hedef: nameOf(targets[f.target], v.targetName ?? '', f.target),
        Sıra: String(rank),
        Uzaklık: lengthText(ctx.units, f.d),
      };
      return [newObject({ kind: 'line', a: f.a, b: f.b }, v.layer.id, attrs)];
    });
    if (meeting) feedback.info(`${meeting} çift değiyor ya da kesişiyor (uzaklık 0); çizgi çizilmedi.`);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${add.length} en kısa çizgi yazıldı.`,
    };
  },
});
