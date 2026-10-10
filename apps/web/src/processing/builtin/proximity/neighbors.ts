import type { Entity } from '../../../model/entities';
import { defineTool, type Shown } from '../../types';
import { withAttr } from '../attributeWrites';
import { NEIGHBOR_KINDS, PROXIMITY_SCOPES, areaText, lengthText, nameOf } from './shared';

/** How two areas neighbour, as the table names it. */
export const NEIGHBOR_LABEL = { edge: 'Kenar', corner: 'Köşe', overlap: 'Örtüşme' } as const;

/**
 * Komşu alanlar (docs/adr/0215 §3.4; ArcGIS Polygon Neighbors): each area's neighbours, both ways round, with the
 * length of boundary they share within the tolerance, those that only meet at a corner (if asked) and those whose
 * insides overlap (if asked) with the overlapping area. A table; if asked, each area's neighbour count and names
 * written to its attributes in one undo step. The pairs are the run's store's (`Store::neighbors`).
 */
export const proximityNeighbors = defineTool({
  id: 'proximity.neighbors',
  label: 'Komşu alanlar',
  category: 'proximity',
  icon: 'polygonNeighbors',
  description: 'Alanların komşularını, ortak kenar uzunluklarını, köşeden değenleri ve örtüşenleri tablo olarak verir; isterseniz komşu sayısını ve adlarını yazar.',
  help: [
    'Örnekler: bir adanın parsellerinin komşuları ve ortak sınır uzunlukları; ifraz öncesi komşu parsellerin listesi; örtüşen parsellerin bulunması.',
    'İki alanın ortak kenarı sınırlarının birbirinin üstünde kalan parçalarının uzunluğudur: aynı doğrudaki düz kenarlar ve aynı dairedeki yaylar, toleransla. Ortak kenarı toleranstan kısa olup değenler köşe komşusudur; içleri örtüşenler Örtüşme olarak örtüşen alanlarıyla yazılır.',
    'Satırlar iki yönlüdür (A–B ve B–A), girdinin sırasıyla. Adlar Ad alanından; boşsa etiket, o da yoksa sıra. Özniteliğe de yaz açıkken her alana komşu sayısı ve komşularının adları yazılır.',
  ].join('\n\n'),
  keywords: ['komşu', 'komşu parsel', 'ortak kenar', 'ortak sınır', 'polygon neighbors', 'bitişik', 'örtüşme', 'yakınlık'],
  aliases: ['KOMSU', 'KOMSUPARSEL'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: NEIGHBOR_KINDS, scopes: PROXIMITY_SCOPES, writes: true, description: 'Komşulukları aranan alanlar; kilitli katmandakiler alınmaz.' },
    { name: 'tolerance', label: 'Tolerans', type: 'number', unit: 'm', min: 0, max: 1, default: 0.001, description: 'Bu kadar yakın sınırlar ortak sayılır.' },
    { name: 'corners', label: 'Köşe komşuları da', type: 'boolean', default: false, description: 'Yalnız köşeden değenler de yazılır.' },
    { name: 'overlaps', label: 'Örtüşenler de', type: 'boolean', default: true, description: 'İçleri örtüşen alanlar örtüşen alanlarıyla yazılır.' },
    { name: 'name', label: 'Ad alanı', type: 'field', of: 'input', optional: true, description: 'Alanların tablodaki adı; boşsa etiket, o da yoksa sıra.' },
    { name: 'write', label: 'Özniteliğe de yaz', type: 'boolean', default: false, description: 'Her alana komşu sayısı ve komşularının adları.' },
    { name: 'countField', label: 'Komşu sayısı alanı', type: 'field', of: 'input', allowNew: true, default: 'Komşu sayısı', visibleWhen: (v: Shown) => v.write === true },
    { name: 'listField', label: 'Komşular alanı', type: 'field', of: 'input', allowNew: true, default: 'Komşular', visibleWhen: (v: Shown) => v.write === true },
  ] as const,
  outputs: [
    { name: 'table', label: 'Komşuluk tablosu', type: 'table' },
    { name: 'count', label: 'Komşuluk sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const areas = v.input.entities;
    feedback.progress(0, 'Komşular aranıyor');
    const found = ctx.geometry.neighbors(
      areas.map((e) => e.id),
      v.tolerance,
      v.corners,
      v.overlaps,
    );
    const name = (i: number) => nameOf(areas[i], v.name ?? '', i);
    const rows = found.map((f) => [
      name(f.area),
      name(f.neighbor),
      NEIGHBOR_LABEL[f.kind],
      lengthText(ctx.units, f.length),
      f.kind === 'overlap' ? areaText(ctx.units, f.overlap) : '',
    ]);
    const update: { id: number; patch: Partial<Entity> }[] = [];
    if (v.write) {
      const lists = areas.map((): string[] => []);
      for (const f of found) lists[f.area].push(name(f.neighbor));
      areas.forEach((e, i) => {
        let current: Entity = e;
        const put = (field: string, text: string | null) => {
          const attrs = withAttr(ctx, current, field, text);
          if (attrs) current = { ...current, attrs };
        };
        put(v.countField, String(lists[i].length));
        put(v.listField, lists[i].length ? lists[i].join(', ') : null);
        if (current !== e) update.push({ id: e.id, patch: { attrs: current.attrs } });
      });
    }
    const overlapping = found.filter((f) => f.kind === 'overlap').length / 2;
    if (overlapping) feedback.warn(`${overlapping} alan çiftinin içleri örtüşüyor.`);
    return {
      ...(v.write ? { changes: { update } } : {}),
      outputs: { table: { columns: ['Alan', 'Komşu', 'Komşuluk', 'Ortak kenar', 'Örtüşen alan'], rows }, count: found.length },
      summary: `${areas.length} alanda ${found.length / 2} komşuluk bulundu.`,
    };
  },
});
