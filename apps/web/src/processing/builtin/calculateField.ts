import type { Entity } from '../../model/entities';
import { measuredOf, toText, truthy, type ExprScope } from '../../model/expression/expressionLib';
import { defineTool, type DefaultsContext } from '../types';

/**
 * Öznitelik hesapla (QGIS "Field calculator"): writes an expression's
 * value into an attribute of every chosen object, new field or existing.
 * Values are stored as text (attributes are text until typed schemas
 * arrive, CLAUDE.md §5.11). A label that showed the old value follows the
 * new one (§7: etiket ve öznitelik tutarlılığı).
 */
export const calculateField = defineTool({
  id: 'attributes.calculate',
  label: 'Öznitelik hesapla',
  category: 'attributes',
  icon: 'fieldCalc',
  description: 'Seçtiğiniz alana her nesne için bir ifadenin değerini yazar; alan yoksa oluşturur.',
  help: [
    'Değer bir ifadedir: sabit bir metin (\'Arsa\'), başka alanlardan hesap (Ada || \'/\' || Parsel) ya da geometri ($alan, $uzunluk, $y, $x).',
    "Örnekler: metin($alan, 2) hesaplanan alanı iki ondalıkla; yuvarla([Tapu alanı] - $alan, 2) tapu farkını; 'P' || doldur($sıra, 5) sıra numarasını (P00001) yazar.",
    'Etiket o alanın eski değerini gösteriyorsa (parsel numarası gibi) yeni değeri gösterir. İşlem tek adımda geri alınır.',
  ].join('\n\n'),
  keywords: ['öznitelik', 'alan', 'hesapla', 'hesap makinesi', 'değer', 'yaz', 'field', 'calculator', 'attribute', 'update'],
  aliases: ['OZHESAP', 'ALANHESAP'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', default: { scope: 'selection' }, description: 'Özniteliği yazılacak nesneler.' },
    { name: 'field', label: 'Yazılacak alan', type: 'field', of: 'input', allowNew: true, default: 'Hesap alanı', description: 'Listeden var olan bir alanı seçin ya da yeni bir ad yazın.' },
    {
      name: 'value',
      label: 'Değer',
      type: 'expression',
      returns: 'value',
      of: 'input',
      default: (c: DefaultsContext) => `metin($alan, ${c.areaDecimals})`,
      placeholder: 'yuvarla($alan, 2)',
    },
    { name: 'where', label: 'Yalnızca şu koşulda', type: 'expression', returns: 'condition', of: 'input', optional: true, advanced: true, placeholder: "Nitelik = 'Arsa'", description: 'Boş bırakılırsa bütün nesnelere yazılır.' },
    {
      name: 'empty',
      label: 'Sonuç boşsa',
      type: 'enum',
      options: [
        { value: 'keep', label: 'Dokunma', hint: 'Alanın eski değeri kalır' },
        { value: 'clear', label: 'Boşalt', hint: 'Alan boş metin olur' },
      ],
      default: 'keep',
      advanced: true,
      description: 'Değer hesaplanamadığında (eksik alan, sayı olmayan metin) ne yapılacağı.',
    },
    { name: 'label', label: 'Etiketi de güncelle', type: 'boolean', default: true, description: 'Etiket bu alanın eski değerini gösteriyorsa yeni değeri gösterir.' },
  ] as const,
  outputs: [
    { name: 'changed', label: 'Değişen nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  preview: (v) => (v.field.trim() ? `“${v.field.trim()}” alanına yazılacak` : null),
  run: async (v, ctx, feedback) => {
    const field = v.field;
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let same = 0;
    let empty = 0;
    let filtered = 0;
    const list = v.input.entities;
    // $alan, $uzunluk, $y, $x from the geometry store, for every object at once when an expression first asks.
    const measured = measuredOf(() => ctx.geometry.measures(list.map((e) => e.id)));
    for (let i = 0; i < list.length; i++) {
      const e = list[i];
      const scope: ExprScope = { entity: e, index: i + 1, layerName: ctx.layerName, measured: () => measured(i) };
      if (v.where && !truthy(v.where.evaluate(scope))) {
        filtered++;
        continue;
      }
      const value = v.value.evaluate(scope);
      if (value === null && v.empty === 'keep') {
        empty++;
        continue;
      }
      const text = toText(value);
      const old = e.attrs[field];
      if (old === text) {
        same++;
        continue;
      }
      const patch: Partial<Entity> = { attrs: { ...e.attrs, [field]: text } };
      if (v.label && old !== undefined && e.label === old) patch.label = text;
      update.push({ id: e.id, patch });
      if (i % 2000 === 1999) {
        feedback.progress(i / list.length, 'Değerler hesaplanıyor');
        await feedback.yield();
        if (feedback.canceled) return {};
      }
    }
    const notes = [same ? `${same} nesnede değer zaten aynıydı` : '', empty ? `${empty} nesnede sonuç boş olduğu için dokunulmadı` : '', filtered ? `${filtered} nesne koşulu sağlamadı` : ''].filter(Boolean);
    return {
      changes: { update },
      outputs: { changed: update.map((u) => u.id), count: update.length },
      summary: `${update.length} nesnede “${field}” yazıldı${notes.length ? `; ${notes.join('; ')}` : ''}.`,
    };
  },
});
