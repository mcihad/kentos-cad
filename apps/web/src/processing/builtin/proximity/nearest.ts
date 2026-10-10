import type { Entity } from '../../../model/entities';
import type { NearestFound } from '../../geometry';
import { fieldNames } from '../../parameters';
import { defineTool } from '../../types';
import { withAttr } from '../attributeWrites';
import { MEASURE_OPTIONS, PROXIMITY_KINDS, PROXIMITY_SCOPES, bearingText, bound, lengthText } from './shared';

/**
 * En yakını bul (docs/adr/0215 §3.1; ArcGIS Near, QGIS "Join attributes by nearest"): each object gets its nearest
 * target's distance, the target's chosen fields and, if asked, the bearing between them, under a prefix. The search is
 * the run's store's (`Store::nearest`, edge to edge or centre to centre); one undo step.
 */
export const proximityNearest = defineTool({
  id: 'proximity.nearest',
  label: 'En yakını bul',
  category: 'proximity',
  icon: 'nearestFeature',
  description: 'Her nesneye en yakın hedefin uzaklığını, istenen alanlarını ve semtini yazar.',
  help: [
    'Örnekler: her parsele en yakın durağın adı ve uzaklığı; her yapıya en yakın yangın musluğu; her kuyuya en yakın derenin uzaklığı.',
    'Kenardan kenara ölçü en kısa uzaklıktır: değen, kesişen ya da hedefin içinde kalan nesne için 0. Merkezden merkeze ölçü alanların ağırlık merkezleri, öbür nesnelerin yer noktaları arasıdır. Eşit uzaklıkta çizimde önce gelen hedef alınır; nesne kendisinin hedefi olmaz.',
    'Yazılan alanlar önekle adlanır: “<önek>uzaklık”, alınan her alan için “<önek><alan>”, istenirse “<önek>semt” (projenin açı biriminde, kuzeyden saat yönünde). En çok uzaklık içinde hedefi olmayan nesnenin bu alanları boşaltılır.',
  ].join('\n\n'),
  keywords: ['en yakın', 'yakınlık', 'uzaklık', 'near', 'nearest', 'join by nearest', 'komşu', 'mesafe'],
  aliases: ['ENYAKIN', 'YAKINBUL', 'NEAR'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, writes: true, description: 'Yakınlık bilgisinin yazılacağı nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'targets', label: 'Hedefler', type: 'features', kinds: PROXIMITY_KINDS, scopes: PROXIMITY_SCOPES, description: 'En yakını aranan nesneler.' },
    { name: 'measure', label: 'Ölçü', type: 'enum', options: MEASURE_OPTIONS, default: 'edges' },
    { name: 'max', label: 'En çok uzaklık', type: 'number', unit: 'm', min: 0, default: 0, description: '0: sınırsız. Daha uzaktaki hedef alınmaz.' },
    { name: 'fields', label: 'Alınacak alanlar', type: 'field', of: 'targets', multiple: true, optional: true, description: 'Hedefin bu alanları önekle yazılır; boşsa yalnız uzaklık.' },
    { name: 'prefix', label: 'Önek', type: 'string', default: 'Yakın ', allowEmpty: true, maxLength: 24, description: 'Yazılan alanların adlarının başı: “Yakın uzaklık”, “Yakın Ad”.' },
    { name: 'bearing', label: 'Semt de yaz', type: 'boolean', default: false, description: 'En yakın noktalar (merkezden merkeze ölçüde merkezler) arası semt.' },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Hedefi bulunan nesne sayısı', type: 'number' },
  ],
  preview: (v) => `“${v.prefix}uzaklık” alanına yazılacak`,
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const targets = v.targets.entities;
    feedback.progress(0, 'En yakın hedefler aranıyor');
    const found = ctx.geometry.nearest(
      inputs.map((e) => e.id),
      targets.map((e) => e.id),
      1,
      bound(v.max),
      v.measure,
    );
    const nearest = new Map<number, NearestFound>();
    for (const f of found) if (!nearest.has(f.input)) nearest.set(f.input, f);
    const wanted = fieldNames({ multiple: true }, v.fields ?? '');
    const prefix = v.prefix;
    const value = (e: Entity, name: string) => (Object.hasOwn(e.attrs, name) ? e.attrs[name] : null);
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let none = 0;
    inputs.forEach((e, i) => {
      const f = nearest.get(i);
      if (!f) none++;
      const target = f ? targets[f.target] : null;
      let current: Entity = e;
      const put = (name: string, text: string | null) => {
        const attrs = withAttr(ctx, current, name, text);
        if (attrs) current = { ...current, attrs };
      };
      put(`${prefix}uzaklık`, f ? lengthText(ctx.units, f.d) : null);
      for (const name of wanted) put(`${prefix}${name}`, target ? value(target, name) : null);
      if (v.bearing) put(`${prefix}semt`, f && f.d > 0 ? bearingText(ctx.units, f.bearing) : null);
      if (current !== e) update.push({ id: e.id, patch: { attrs: current.attrs } });
    });
    if (none) feedback.info(`${none} nesnenin ${v.max > 0 ? 'en çok uzaklık içinde ' : ''}hedefi yok; alanları boşaltıldı.`);
    return {
      changes: { update },
      outputs: { changed: update.map((u) => u.id), count: inputs.length - none },
      summary: `${inputs.length - none} nesneye en yakın hedef yazıldı (“${prefix}uzaklık”).`,
    };
  },
});
