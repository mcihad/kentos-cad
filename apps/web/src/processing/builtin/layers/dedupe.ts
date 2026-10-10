import type { Entity } from '../../../model/entities';
import { fieldNames } from '../../parameters';
import { defineTool } from '../../types';
import { codePointOrder, fieldsValue, shapeKey } from './shared';

/**
 * Yinelenenleri sil (docs/adr/0217 §5; QGIS Delete duplicates by attribute, ArcGIS Delete Identical): of the objects
 * whose key is the same the first (in the input's order) stays, the rest go. A key is the fields' values trimmed (with
 * no field named: every attribute with a value), and with Geometri de aynı olsun the kind and the shape exactly; the
 * shape is read only for objects whose values and first place another object shares. One undo step.
 */

export const deleteDuplicates = defineTool({
  id: 'layers.deleteDuplicates',
  label: 'Yinelenenleri sil',
  category: 'layers',
  icon: 'deleteDuplicates',
  description: 'Aynı öznitelikli (isterse aynı geometrili) nesnelerden ilkini bırakıp öbürlerini siler.',
  help: [
    'Örnekler: iki kez içe aktarılmış parselleri temizlemek; aynı numaralı noktalardan yalnız ilkini bırakmak.',
    'Alanlar boşsa bütün öznitelikler karşılaştırılır (değeri boş alanlar yok sayılır). Değerler baştaki ve sondaki boşluklar atılarak karşılaştırılır. Geometri de aynı olsun açıkken türü ve şekli (köşeler, yaylar, kotlar) tam aynı olanlar yinelenen sayılır. Girdinin sırasıyla ilk nesne kalır.',
  ].join('\n\n'),
  keywords: ['yinelenen', 'tekrar', 'çift', 'sil', 'temizle', 'duplicate', 'delete duplicates', 'delete identical'],
  aliases: ['YINELENENSIL'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', scopes: ['layer', 'selection', 'visible', 'all'], writes: true, description: 'Yinelenenleri silinecek nesneler; kilitli katmandakiler alınmaz.' },
    { name: 'fields', label: 'Alanlar', type: 'field', of: 'input', multiple: true, optional: true, description: 'Boş bırakılırsa bütün öznitelikler.' },
    { name: 'geometry', label: 'Geometri de aynı olsun', type: 'boolean', default: true, description: 'Türü ve şekli de aynı olanlar yinelenen sayılır.' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Yinelenenler', type: 'table' },
    { name: 'count', label: 'Silinen nesne sayısı', type: 'number' },
  ],
  run: async (v, _ctx, feedback) => {
    const names = fieldNames({ multiple: true }, v.fields ?? '');
    const entities = v.input.entities;
    feedback.progress(0, 'Anahtarlar karşılaştırılıyor');
    // The objects' classes of values, in the order they first come.
    const classOf = new Int32Array(entities.length);
    const sizes: number[] = [];
    const classes = new Map<string, number>();
    for (const [i, e] of entities.entries()) {
      const key = valuesKey(e, names);
      let c = classes.get(key);
      if (c === undefined) {
        c = sizes.length;
        classes.set(key, c);
        sizes.push(0);
      }
      sizes[c]++;
      classOf[i] = c;
    }
    // An object alone with its values has no duplicate; with Geometri de aynı olsun nor has one alone with its values
    // and what equal shapes share (`rough`): the shape of neither is read.
    const candidate = entities.map((_, i) => sizes[classOf[i]] >= 2);
    if (v.geometry) {
      const roughKeys = entities.map((e, i) => (candidate[i] ? `${classOf[i]}\u0000${rough(e)}` : ''));
      const counts = new Map<string, number>();
      for (const [i, k] of roughKeys.entries()) if (candidate[i]) counts.set(k, (counts.get(k) ?? 0) + 1);
      for (const [i, k] of roughKeys.entries()) if (candidate[i] && counts.get(k)! < 2) candidate[i] = false;
    }
    const first = new Map<string, number>();
    const groups = new Map<number, number>();
    const remove: number[] = [];
    for (const [i, e] of entities.entries()) {
      if (!candidate[i]) continue;
      const key = v.geometry ? `${classOf[i]}\u0000${shapeKey(e)}` : String(classOf[i]);
      const kept = first.get(key);
      if (kept === undefined) {
        first.set(key, i);
        continue;
      }
      remove.push(e.id);
      groups.set(kept, (groups.get(kept) ?? 0) + 1);
    }
    const table = { columns: ['Değer', 'Silinen'], rows: [...groups].map(([kept, n]) => [shown(entities[kept], names), String(n)]) };
    if (!remove.length) return { outputs: { table, count: 0 }, summary: 'Yinelenen nesne yok.' };
    return {
      changes: { remove },
      outputs: { table, count: remove.length },
      summary: `${groups.size} grupta ${remove.length} yinelenen nesne silindi.`,
    };
  },
});

/** Every attribute with a value, trimmed, in the names' code point order. */
function valuedAttrs(e: Entity): (readonly [string, string])[] {
  return Object.entries(e.attrs)
    .map(([k, x]) => [k, x.trim()] as const)
    .filter(([, x]) => x !== '')
    .sort(([a], [b]) => codePointOrder(a, b));
}

/** An object's values as its key: one field's value itself, more fields' or every attribute's as JSON. */
function valuesKey(e: Entity, names: readonly string[]): string {
  const value = (f: string) => (Object.hasOwn(e.attrs, f) ? e.attrs[f] : '').trim();
  if (names.length === 1) return value(names[0]);
  if (names.length) return JSON.stringify(names.map(value));
  return JSON.stringify(valuedAttrs(e));
}

/** The values as the table shows them. */
function shown(e: Entity, names: readonly string[]): string {
  if (names.length) return fieldsValue(e, names);
  return (
    valuedAttrs(e)
      .map(([k, x]) => `${k}: ${x}`)
      .join('; ') || '—'
  );
}

/** What equal shapes share, read at once: the kind and the first place (a zero's two signs write alike). */
function rough(e: Entity): string {
  const o = e as { kind: string; pts?: readonly { x: number; y: number }[]; p?: { x: number; y: number }; a?: { x: number; y: number }; c?: { x: number; y: number } };
  const at = o.pts?.[0] ?? o.p ?? o.a ?? o.c;
  return at ? `${o.kind}|${o.pts?.length ?? 0}|${at.x}|${at.y}` : o.kind;
}
