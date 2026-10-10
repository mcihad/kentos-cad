import { orderOf, type OrderKey } from '../../../model/ops/order';
import { defineTool } from '../../types';
import { copyOf } from './shared';

/**
 * İfadeye göre sırala (docs/adr/0217 §6; QGIS Order by expression): copies of the objects written in an expression's
 * order into the result layer (the attribute table and the drawing order are theirs), the order field 1, 2, 3 …; the
 * order is the core's (`ops::order`): values without a place where Boşlar puts them, truth values, numbers, texts in
 * the natural order, Azalan turning the values round, equal ones in the input's order. A new result layer takes the
 * look and fields of the first object's layer and goes right above it. One undo step.
 */

export const orderByExpression = defineTool({
  id: 'layers.orderBy',
  label: 'İfadeye göre sırala',
  category: 'layers',
  icon: 'orderByExpression',
  description: 'Nesnelerin kopyalarını bir ifadenin değerine göre sıralı olarak yeni bir katmana yazar; isterse sıra numarasını bir alana yazar.',
  help: [
    'Örnekler: parselleri alanlarına göre büyükten küçüğe, noktaları adlarına göre doğal sırayla (P2, P10) dizmek; sıra numarasını bir alana yazmak.',
    "Değeri olmayan nesneler Boşlar'a göre başta ya da sonda. Türler karışıksa evet/hayır, sayı, metin sırasıyla; metinler doğal sırayla (sayılar değerleriyle, Türkçe harf sırasıyla). Eşit değerler girdinin sırasıyla kalır.",
    'Kopyalar sonuç katmanına sıralı yazılır; Öznitelik tablosu ve çizim sırası onların sırasıdır. Girdi değişmez.',
  ].join('\n\n'),
  keywords: ['sırala', 'sıra', 'ifade', 'artan', 'azalan', 'order', 'order by', 'sort'],
  aliases: ['IFADEYLESIRALA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Sıralanacak nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'expression', label: 'İfade', type: 'expression', returns: 'value', of: 'input', placeholder: '$alan', description: 'Nesneleri sıralayan değer.' },
    {
      name: 'direction',
      label: 'Yön',
      type: 'enum',
      options: [
        { value: 'asc', label: 'Artan', hint: "Küçükten büyüğe, A'dan Z'ye" },
        { value: 'desc', label: 'Azalan', hint: "Büyükten küçüğe, Z'den A'ya" },
      ],
      default: 'asc',
    },
    {
      name: 'nulls',
      label: 'Boşlar',
      type: 'enum',
      options: [
        { value: 'last', label: 'Sonda', hint: 'Değeri olmayan nesneler sonda' },
        { value: 'first', label: 'Başta', hint: 'Değeri olmayan nesneler başta' },
      ],
      default: 'last',
    },
    { name: 'orderField', label: 'Sıra alanı', type: 'field', of: 'input', allowNew: true, optional: true, default: 'Sıra', description: 'Sıra numarası (1, 2, 3 …) bu alana yazılır; boşsa yazılmaz.' },
    { name: 'output', label: 'Sonuç', type: 'layer', default: { newName: 'Sıralı' }, newLayerStyle: { color: '#12A594' }, above: 'input' },
  ] as const,
  outputs: [
    { name: 'objects', label: 'Sıralı nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Değerler hesaplanıyor');
    // One call to the run's geometry store: the geometry values ($alan, $uzunluk …) are read there.
    const objects = { entities: inputs, layerName: ctx.layerName, geometry: ctx.geometry, layers: ctx.layers };
    const column = v.expression.evaluateAll(objects, 'value');
    const keys: OrderKey[] = inputs.map((_, i) => column.value(i));
    feedback.progress(0.5, 'Sıralanıyor');
    const order = orderOf(keys, v.direction === 'desc', v.nulls === 'first');
    const field = (v.orderField ?? '').trim();
    const out = v.output;
    const add = order.map((i, n) => copyOf(inputs[i], out.id, field ? { ...inputs[i].attrs, [field]: String(n + 1) } : undefined));
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} nesne sıralanıp “${out.name}” katmanına yazıldı.`,
      ...(inputs[0] && { like: inputs[0].layerId }),
    };
  },
});
