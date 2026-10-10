import type { Entity, NewEntity } from '../../../model/entities';
import { fieldNames } from '../../parameters';
import { defineTool, type ToolLayer } from '../../types';
import { copyOf, fieldsValue, MODE_OPTIONS } from './shared';

/**
 * Özniteliğe göre böl (docs/adr/0217 §2; QGIS Split vector layer, ArcGIS Split By Attributes): the objects split into
 * layers by their fields' values, a layer a value in the order the values first come, in a new group named after the
 * fields right above the first object's layer; each layer takes the look and fields of its value's first object's
 * layer. The objects are copied, or moved. One undo step.
 */

/** The most values: past them the run is refused. */
const MOST_LAYERS = 1000;

export const splitByAttribute = defineTool({
  id: 'layers.splitByAttribute',
  label: 'Özniteliğe göre böl',
  category: 'layers',
  icon: 'layerSplitAttr',
  description: 'Nesneleri bir ya da birkaç özniteliğin değerlerine göre yeni bir gruptaki katmanlara böler: her değer bir katman.',
  help: [
    'Örnekler: parselleri mahallelerine, yapıları kullanım ve kat sayılarına, yolları türlerine göre ayrı katmanlarda toplamak.',
    'Değerler baştaki ve sondaki boşluklar atılarak okunur; değeri olmayan “(boş)” katmanına gider; birden çok alanda değerler “ – ” ile birleşir. Katmanlar alanların adını taşıyan yeni bir grupta, değerlerin ilk görüldüğü sırayla açılır ve her biri değerinin ilk nesnesinin katmanının görünüşünü ve alanlarını alır.',
    'Kopyala nesneleri yerinde bırakır; Taşı onları yeni katmanlara geçirir. Kilitli katmandaki nesneler alınmaz. En çok 1000 katmana bölünür.',
  ].join('\n\n'),
  keywords: ['böl', 'ayır', 'katman', 'öznitelik', 'değer', 'grupla', 'split', 'split by attributes', 'split vector layer'],
  aliases: ['OZNITELIKBOL'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Bölünecek nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'fields', label: 'Alanlar', type: 'field', of: 'input', multiple: true, description: 'Değerleri katmanları veren alan ya da alanlar.' },
    { name: 'mode', label: 'İşlem', type: 'enum', options: MODE_OPTIONS, default: 'copy' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Bölme raporu', type: 'table' },
    { name: 'count', label: 'Katman sayısı', type: 'number' },
  ],
  run: async (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    const names = fieldNames({ multiple: true }, v.fields);
    feedback.progress(0, 'Değerler okunuyor');
    const groups = new Map<string, Entity[]>();
    const valueOf = new Map<number, string>();
    for (const e of inputs) {
      const value = fieldsValue(e, names);
      valueOf.set(e.id, value);
      const list = groups.get(value);
      if (list) list.push(e);
      else groups.set(value, [e]);
    }
    if (groups.size > MOST_LAYERS) return { refused: `${groups.size} farklı değer var; en çok ${MOST_LAYERS} katmana bölünür. Daha az değerli bir alan seçin.` };
    // The run's group, then a layer a value; objects and patches name them by their keys.
    const key = new Map<string, string>();
    const layers: ToolLayer[] = [{ key: '#grup', name: names.join(', '), group: true, above: inputs[0]?.layerId }];
    for (const [value, list] of groups) {
      key.set(value, `#${key.size + 1}`);
      layers.push({ key: key.get(value)!, name: value, parent: '#grup', like: list[0].layerId });
    }
    const keyOf = (e: Entity) => key.get(valueOf.get(e.id)!)!;
    const move = v.mode === 'move';
    const add: NewEntity[] = move ? [] : inputs.map((e) => copyOf(e, keyOf(e)));
    const update = move ? inputs.map((e) => ({ id: e.id, patch: { layerId: keyOf(e) } })) : [];
    const rows = [...groups].map(([value, list]) => [value, String(list.length)]);
    return {
      changes: { add, update },
      layers,
      outputs: { table: { columns: ['Değer', 'Nesne'], rows }, count: groups.size },
      summary: `${inputs.length} nesne ${groups.size} katmana bölündü (${move ? 'taşındı' : 'kopyalandı'}).`,
    };
  },
});
