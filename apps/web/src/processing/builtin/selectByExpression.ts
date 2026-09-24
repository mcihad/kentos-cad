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
    feedback.progress(0, 'Koşul deneniyor');
    // One call to the core for every object; $alan, $uzunluk, $y, $x from the geometry store when the condition asks.
    const met = v.condition.evaluateAll({ entities: list, layerName: ctx.layerName, measures: () => ctx.geometry.measures(list.map((e) => e.id)) }, 'bool');
    const hits: number[] = [];
    for (let i = 0; i < list.length; i++) if (met.value(i) === true) hits.push(list[i].id);
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
