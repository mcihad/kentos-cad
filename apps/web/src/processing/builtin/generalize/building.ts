import { fixed } from '../../../core/displayNumber';
import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeBuilding } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { inputNotes, newObject } from '../geometry/shared';
import { outputLayer } from '../convert/shared';
import { GENERALIZE_SCOPES, degreesParam, finish, measureParam, table } from './shared';

/**
 * Bina sadeleştir (docs/adr/0221 §4; Ruas, AGENT, CartAGen's simplify_building_ruas): short edges gone as their
 * neighbours' directions say; a building whose result would grow to more than twice its area or be invalid stays as
 * it is. The core's `ops::generalize::building`.
 */

const COLUMNS = ['Nesne', 'Köşe (önce)', 'Köşe (sonra)', 'Alan değişimi (m²)', 'Alan değişimi (%)'];

export const building = defineTool({
  id: 'generalize.building',
  label: 'Bina sadeleştir',
  category: 'generalization',
  icon: 'buildingSimplify',
  description: 'Binaların en kısa kenardan kısa kenarlarını komşularına göre atar: basamaklar, çıkıntılar, girintiler ve pahlar gider, dik açılar kalır.',
  help: [
    'Örnekler: büyük ölçekli bina ayak izlerini 1/5000 haritaya indirmek, küçük girinti ve çıkıntıları atmak.',
    "En kısa kenardan başlanır (Ruas'ın kuralı): komşu kenarları dikse uzatılıp buluşur; aynı yöne giderse (çıkıntı ya da girinti) kısa yanın ucundan kesilir; ters yöne giderse (basamak) iki kenar ortadan geçen tek kenar olur. Sonra kısa kenarlar yeniden aranır. Yaylı kenarlar ve yanlarındakiler kalır.",
    'Sadeleşen bina iki katından büyük ya da geçersiz çıkarsa olduğu gibi yazılır ve söylenir. Kotlar taşınmaz.',
  ].join('\n\n'),
  keywords: ['bina', 'sadeleştir', 'genelleştir', 'simplify building', 'basamak', 'çıkıntı', 'girinti'],
  aliases: ['BINASADE', 'SIMPLIFYBUILDING'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Binalar', type: 'features', kinds: ['polygon'], scopes: GENERALIZE_SCOPES, description: 'Sadeleştirilecek bina alanları.' },
    measureParam('minEdge', 'En kısa kenar', 'm', 0.01, 3, 'Bundan kısa kenarlar atılır.'),
    degreesParam('parallel', 'Paralel sınırı', 45, 20, 'Komşu kenarların paralel sayıldığı açı farkı.'),
    degreesParam('orthogonal', 'Dik sınırı', 45, 20, 'Komşu kenarların dik sayıldığı açı farkı.'),
    outputLayer('Sadeleştirilen binalar', '#D6409F'),
  ] as const,
  outputs: [
    { name: 'simplified', label: 'Yazılan binalar', type: 'features' },
    { name: 'report', label: 'Sadeleştirme raporu', type: 'table' },
    { name: 'count', label: 'Atılan kenar sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    inputNotes(feedback, inputs);
    feedback.progress(0, 'Kısa kenarlar atılıyor');
    const results = generalizeBuilding(inputs.map(entityGeometry), v.minEdge ?? 3, v.parallel ?? 20, v.orthogonal ?? 20);
    const ad = ctx.units.areaDecimals;
    const add: NewEntity[] = [];
    const rows: string[][] = [];
    let edits = 0;
    let grew = 0;
    let invalid = 0;
    results.forEach((r, i) => {
      if (!r) return;
      if (r.case === 'grew') grew++;
      else if (r.case === 'invalid') invalid++;
      else if (r.case === 'changed') {
        edits += r.edits;
        const change = r.area[1] - r.area[0];
        rows.push([`#${inputs[i].id}`, String(r.vertices[0]), String(r.vertices[1]), fixed(change, ad), fixed((change / r.area[0]) * 100, 2)]);
      }
      add.push(newObject(r.shape, v.layer.id, { ...inputs[i].attrs }));
    });
    if (grew) feedback.warn(`${grew} binanın sadeleşmişi iki katından büyük çıktı; olduğu gibi yazıldı.`);
    if (invalid) feedback.warn(`${invalid} binanın sadeleşmişi geçersiz çıktı; olduğu gibi yazıldı.`);
    const summary = rows.length
      ? `${add.length} nesne yazıldı; ${rows.length} binada ${edits} kısa kenar atıldı.`
      : `${add.length} nesne yazıldı; atılacak kısa kenar yok.`;
    return finish(add, summary, edits, table(COLUMNS, rows));
  },
});
