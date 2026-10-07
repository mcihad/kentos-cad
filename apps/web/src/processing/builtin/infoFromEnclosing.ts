import type { Entity } from '../../model/entities';
import { defineTool } from '../types';
import { withAttr } from './attributeWrites';
import { AREA_KINDS, QUERY_KINDS } from './selectByLocation';

/**
 * Çevreleyenden bilgi al (docs/adr/0200 §3; Netcad's Çevreleyenden Bilgi Al, QGIS "Join attributes by location"):
 * each target object gets a field of the source area its centre is in (a building its parcel's number). Several areas:
 * the first in the drawing's order, counted and said; none: the target stays as it is, counted and said. The centres
 * and the areas are the core's (`Merkezi içinde`); one undo step.
 */
export const infoFromEnclosing = defineTool({
  id: 'attributes.fromEnclosing',
  label: 'Çevreleyenden bilgi al',
  category: 'attributes',
  icon: 'infoEnclosing',
  description: 'Her nesneye ağırlık merkezinin içinde kaldığı alanın bir alanını yazar: yapıya parsel numarası, parsele mahalle adı.',
  help: [
    'Nesnenin merkezi ifadelerdeki $merkez_y, $merkez_x’tir: alanların ağırlık merkezi, öbür nesnelerin yerleşim noktası. Merkez alanın sınırındaysa (1 mm içinde) o alan da sayılır.',
    'Merkez birden çok alanın içindeyse çizim sırasıyla ilki alınır; hiçbirinin içinde değilse nesne değişmez. İkisi de sayılır ve söylenir.',
    'Yazılacak alan boş bırakılırsa kaynağın alanının adı kullanılır. İşlem tek adımda geri alınır.',
  ].join('\n\n'),
  keywords: ['çevreleyen', 'içinde', 'parsel', 'ada', 'mahalle', 'mekânsal birleştir', 'spatial join', 'join attributes by location'],
  aliases: ['CEVREBILGI', 'CEVRELEYENBILGI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'target', label: 'Hedef nesneler', type: 'features', kinds: QUERY_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Bilginin yazılacağı nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'source', label: 'Kaynak alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'all', 'visible', 'selection'], description: 'Hedefleri çevreleyen alanlar.' },
    { name: 'field', label: 'Alan', type: 'field', of: 'source', description: 'Kaynak alanların değeri alınan alanı.' },
    { name: 'output', label: 'Yazılacak alan', type: 'field', of: 'target', allowNew: true, optional: true, description: 'Boş bırakılırsa kaynağın alanının adı.' },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const targets = v.target.entities;
    const sources = v.source.entities;
    const field = v.field;
    const output = v.output || field;
    feedback.progress(0, 'Merkezler deneniyor');
    const pairs = ctx.geometry.relatePairs(
      targets.map((e) => e.id),
      sources.map((e) => e.id),
      'centerIn',
      0,
    );
    // Each target's areas in the drawing's order (the pairs come target by target, areas in order).
    const holders: Entity[][] = targets.map(() => []);
    for (const [i, j] of pairs) holders[i].push(sources[j]);
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let many = 0;
    let none = 0;
    targets.forEach((t, i) => {
      const first = holders[i][0];
      if (!first) {
        none++;
        return;
      }
      if (holders[i].length > 1) many++;
      const attrs = withAttr(ctx, t, output, Object.hasOwn(first.attrs, field) ? first.attrs[field] : null);
      if (attrs) update.push({ id: t.id, patch: { attrs } });
    });
    if (many) feedback.warn(`${many} nesnenin merkezi birden çok alanın içinde; çizim sırasıyla ilki alındı.`);
    if (none) feedback.warn(`${none} nesnenin merkezi hiçbir alanın içinde değil; değişmedi.`);
    return {
      changes: { update },
      outputs: { changed: update.map((u) => u.id), count: update.length },
      summary: `${update.length} nesneye “${output}” yazıldı.`,
    };
  },
});
