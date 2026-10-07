import { geoBuffer } from '../../../model/ops/geoprocess';
import { defineTool } from '../../types';
import { GEO_KINDS, GEO_SCOPES, emptyNote, inputNotes, newObject, outputStyle } from './shared';

/**
 * Tampon (docs/adr/0201 §2; Netcad's Tampon Bölge, ArcGIS Buffer and Multiple Ring Buffer): the places within a
 * distance of each object, written to the output layer. A point's buffer is a disc, a segment's a capsule, an arc's a
 * band with discs at its ends, an area's all of them around its boundary and the area itself; a minus distance
 * shrinks an area. One side of a path is its strips on that side with its ends flat; rings are the bands between
 * (k − 1)·d and k·d. The pieces and their joins are the core's (`ops::geoprocess::buffer`), exact with their arcs.
 */
export const geometryBuffer = defineTool({
  id: 'geometry.buffer',
  label: 'Tampon',
  category: 'geometry',
  icon: 'geoBuffer',
  description: 'Nesnelerin çevresinde verilen uzaklıktaki alanı çizer: noktada daire, çizgide koridor, alanda dışa ya da (eksi uzaklıkla) içe; halkalarla ya da tek yandan da.',
  help: [
    'Noktanın tamponu dairedir; çizginin her kenarı iki yanından uzaklık kadar genişler, uçları yarım dairedir; alanın tamponu alanı da içine alır. Yaylar ve daireler kesin kalır; elips ve eğri 0,1 mm içinde doğru parçalarıyla girer.',
    'Eksi uzaklık yalnız alanlarda anlamlıdır: alan kenarlarından içe küçülür, köşeleri keskin kalır; ortasını aşan küçültme bir şey bırakmaz. Tek yanlı tampon (Sol, Sağ) çizginin çizildiği yöne göredir, uçları düzdür; alan ve noktalar iki yandan alınır.',
    'Halka sayısı birden çoksa her halka (k − 1)·d ile k·d arasındaki şerittir; her parçaya “Uzaklık” (k·d) ve “Halka” (k) yazılır. Birleştir açıkken bütün tamponlar halka başına tek alandır ve nesnelerin öznitelikleri yazılmaz.',
    'Uzaklık alanı seçilirse her nesnenin uzaklığı bu alandan okunur (ondalık nokta ya da virgül); okunamayan nesne alınmaz ve söylenir.',
  ].join('\n\n'),
  keywords: ['tampon', 'tampon bölge', 'buffer', 'koridor', 'uzaklık', 'halka', 'çoklu halka', 'multiple ring buffer', 'offset', 'etki alanı'],
  aliases: ['TAMPON', 'TAMPONBOLGE', 'BUFFER'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Tamponu çizilecek alanlar, çizgiler ve noktalar; yazı, ölçü ve blok gibi nesneler alınmaz.' },
    { name: 'distance', label: 'Uzaklık', type: 'number', unit: 'm', default: 5, description: 'Alanlarda eksi değer alanı içe küçültür.' },
    { name: 'side', label: 'Yan', type: 'enum', options: [
      { value: 'both', label: 'İki yan', hint: 'Çizginin iki yanı ve uçları' },
      { value: 'left', label: 'Sol', hint: 'Çizildiği yöne göre solu; uçları düz' },
      { value: 'right', label: 'Sağ', hint: 'Çizildiği yöne göre sağı; uçları düz' },
    ], default: 'both', description: 'Yalnız çizgilerde; alanlar ve noktalar iki yandan alınır.' },
    { name: 'rings', label: 'Halka sayısı', type: 'number', integer: true, min: 1, max: 20, default: 1, description: 'Birden çoksa her halka (k − 1)·d ile k·d arasıdır.' },
    { name: 'dissolve', label: 'Birleştir', type: 'boolean', default: false, description: 'Bütün tamponlar halka başına tek alan olur; öznitelikler yazılmaz.' },
    { name: 'distanceField', label: 'Uzaklık alanı', type: 'field', of: 'input', optional: true, advanced: true, description: 'Seçilirse her nesnenin uzaklığı bu alandan okunur; Uzaklık kullanılmaz.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Tampon' }, newLayerStyle: outputStyle('#0090FF'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'buffers', label: 'Tamponlar', type: 'features' },
    { name: 'count', label: 'Tampon sayısı', type: 'number' },
  ],
  validate: (v) => {
    if ((v.distanceField ?? '').trim()) return null;
    const d = v.distance;
    if (Math.abs(d) < 0.001) return '“Uzaklık” en az 0.001 m olmalı (eksi değerde mutlak değeri).';
    if (Math.abs(d) > 100000) return '“Uzaklık” en çok 100000 m olabilir.';
    if (d < 0 && v.rings > 1) return 'Halkalar yalnız artı uzaklıkla çizilir; Halka sayısını 1 yapın ya da uzaklığı artı yazın.';
    return null;
  },
  preview: (v) => {
    if ((v.distanceField ?? '').trim()) return `Uzaklık “${(v.distanceField ?? '').trim()}” alanından okunacak`;
    return v.rings > 1 ? `${v.rings} halka: ${v.distance} m arayla` : null;
  },
  run: (v, _ctx, feedback) => {
    const input = v.input.entities;
    const field = (v.distanceField ?? '').trim();
    const distances = input.map((e) => (field ? (Object.hasOwn(e.attrs, field) ? e.attrs[field] : null) : String(v.distance)));
    inputNotes(feedback, input);
    feedback.progress(0, 'Tamponlar çiziliyor');
    const r = geoBuffer(input, distances, v.side, v.rings, v.dissolve);
    if (r.unread.length) feedback.warn(`${r.unread.length} nesnenin uzaklığı “${field}” alanından sayı olarak okunamadı; alınmadı.`);
    if (r.inward.length) feedback.warn(`${r.inward.length} nesnenin uzaklığı eksi; halkalar yalnız artı uzaklıkla çizilir, alınmadı.`);
    emptyNote(r.empty.length, feedback);
    const add = r.pieces.map((p) => {
      const extra: Record<string, string> = {};
      if (p.distance !== undefined) extra['Uzaklık'] = p.distance;
      if (v.rings > 1) extra['Halka'] = String(p.ring);
      return newObject(p.shape, v.layer.id, p.source === undefined ? extra : { ...input[p.source].attrs, ...extra });
    });
    const used = input.length - r.unread.length - r.inward.length - r.empty.length;
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: v.dissolve ? `${used} nesnenin tamponu birleştirilerek yazıldı (${add.length} alan).` : `${used} nesnenin tamponu yazıldı (${add.length} alan).`,
    };
  },
});
