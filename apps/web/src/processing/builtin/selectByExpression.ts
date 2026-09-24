import { measuredOf, truthy } from '../../model/expression/expressionLib';
import { defineTool } from '../types';

/**
 * İfadeyle seç (QGIS "Select by expression"): selects the objects for
 * which a condition over attributes and geometry holds, combined with the
 * current selection as the user chose. Edits nothing, so there is nothing
 * to undo; the selection itself is the result.
 */
export const selectByExpression = defineTool({
  id: 'selection.byExpression',
  label: 'İfadeyle seç',
  category: 'selection',
  icon: 'selectExpression',
  description: 'Özniteliklere ve geometriye göre yazdığınız koşulu sağlayan nesneleri seçer.',
  help: [
    "Koşulda alan adları doğrudan (Nitelik) ya da köşeli parantezle ([Tapu alanı]) yazılır, metinler tırnak içindedir ('Arsa'). Geometri için $alan, $uzunluk, $köşe, $katman gibi değişkenler vardır.",
    "Örnekler: Nitelik = 'Arsa' ve $alan > 500; boş(Parsel); içerir(Mahalle, 'cumhuriyet'); $katman = 'Yapı' veya $alan < 50.",
    'Seçim biçimi sonucun mevcut seçimle nasıl birleşeceğini belirler: yeni seçim, seçime ekle, seçimden çıkar ya da yalnızca şu an seçili olanlar arasında ara.',
  ].join('\n\n'),
  keywords: ['seç', 'sorgu', 'filtre', 'ifade', 'koşul', 'öznitelik', 'select', 'query', 'expression', 'where'],
  aliases: ['IFADESEC', 'SORGU'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Aranacak nesneler', type: 'features', scopes: ['all', 'visible', 'layer', 'selection'], default: { scope: 'all' }, description: 'Koşulun denendiği nesneler.' },
    { name: 'condition', label: 'Koşul', type: 'expression', returns: 'condition', of: 'input', default: '$alan > 500', placeholder: "Nitelik = 'Arsa' ve $alan > 500" },
    {
      name: 'mode',
      label: 'Seçim biçimi',
      type: 'enum',
      options: [
        { value: 'new', label: 'Yeni seçim', hint: 'Mevcut seçim bırakılır' },
        { value: 'add', label: 'Seçime ekle', hint: 'Koşulu sağlayanlar mevcut seçime eklenir' },
        { value: 'remove', label: 'Seçimden çıkar', hint: 'Koşulu sağlayanlar mevcut seçimden çıkar' },
        { value: 'within', label: 'Seçim içinde ara', hint: 'Seçili olanlardan yalnızca koşulu sağlayanlar kalır' },
      ],
      default: 'new',
    },
  ] as const,
  outputs: [
    { name: 'matched', label: 'Koşulu sağlayanlar', type: 'features' },
    { name: 'count', label: 'Koşulu sağlayan sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const list = v.input.entities;
    // $alan, $uzunluk, $y, $x from the geometry store, for every object at once when the condition first asks.
    const measured = measuredOf(() => ctx.geometry.measures(list.map((e) => e.id)));
    const hits: number[] = [];
    for (let i = 0; i < list.length; i++) {
      if (truthy(v.condition.evaluate({ entity: list[i], index: i + 1, layerName: ctx.layerName, measured: () => measured(i) }))) hits.push(list[i].id);
      if (i % 2000 === 1999) {
        feedback.progress(i / list.length, 'Koşul deneniyor');
        await feedback.yield();
        if (feedback.canceled) return {};
      }
    }
    const current = ctx.selection;
    const hit = new Set(hits);
    const select = v.mode === 'new' ? hits : v.mode === 'add' ? [...current, ...hits] : v.mode === 'remove' ? current.filter((id) => !hit.has(id)) : current.filter((id) => hit.has(id));
    return {
      select,
      outputs: { matched: hits, count: hits.length },
      summary: `${hits.length} / ${list.length} nesne koşulu sağladı; seçimde ${new Set(select).size} nesne var.`,
    };
  },
});
