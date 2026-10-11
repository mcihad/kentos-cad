import { convertDensify, convertDensifyCount, convertLineAreas, convertRings, convertSubstring } from '../../../model/ops/convert';
import type { NewEntity } from '../../../model/entities';
import { defineTool } from '../../types';
import { CONVERT_SCOPES, outputLayer, placed, zsOf } from './shared';

/**
 * The path tools (docs/adr/0218 §8–§11): a path's piece between two distances along it (QGIS Line substring), an
 * area's rings as lines and lines as areas (QGIS Polygons to lines, Lines to polygons), edges divided (ArcGIS
 * Densify, QGIS Densify by interval and by count). The shapes are the core's (`ops::convert`), a list a call; the
 * elevations carried by docs/adr/0142's rules.
 */

export const substring = defineTool({
  id: 'convert.substring',
  label: 'Çizginin parçası',
  category: 'conversion',
  icon: 'lineSubstring',
  description: 'Her çizginin başından verilen iki uzaklık arasındaki parçasını yeni katmana yazar.',
  help: [
    'Örnekler: bir yolun iki km arası kesimini, bir hattın ilk 100 metresini, her çizginin uçlarından 5 metre kısaltılmışını almak.',
    'Uzaklıklar yolun başından yol boyunca metredir (yaylar kesin); ifade olduklarından her nesnede başka olabilir: $uzunluk / 2 yolun yarısı. Uzaklıklar 0 ile uzunluk arasına alınır.',
    'Doğru parçaları doğru, yaylar yay kalır; düz bir kenarda kalan parça doğrudur. Başlangıcı bitişinden küçük olmayan, uzaklığı sayı olmayan ve çok parçalı nesne yazılmaz, söylenir. Köşelerin kotları kenarları boyunca taşınır.',
  ].join('\n\n'),
  keywords: ['parça', 'kesim', 'alt çizgi', 'km', 'uzaklık', 'line substring', 'segment'],
  aliases: ['CIZGIPARCASI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['line', 'polyline', 'arc'], scopes: CONVERT_SCOPES, description: 'Parçası alınacak çizgiler ve yaylar.' },
    { name: 'start', label: 'Başlangıç', type: 'expression', returns: 'value', of: 'input', default: '0', placeholder: '0', description: 'Yolun başından uzaklık (m).' },
    { name: 'end', label: 'Bitiş', type: 'expression', returns: 'value', of: 'input', default: '$uzunluk', placeholder: '$uzunluk', description: 'Yolun başından uzaklık (m).' },
    outputLayer('Parçalar', '#3E63DD'),
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Yazılan parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Uzaklıklar hesaplanıyor');
    const objects = { entities: inputs, layerName: ctx.layerName, geometry: ctx.geometry, layers: ctx.layers };
    const number = (x: unknown): number | null => (typeof x === 'number' && Number.isFinite(x) ? x : null);
    const starts = v.start.evaluateAll(objects, 'number');
    const ends = v.end.evaluateAll(objects, 'number');
    const s0 = inputs.map((_, i) => number(starts.value(i)));
    const s1 = inputs.map((_, i) => number(ends.value(i)));
    const pieces = convertSubstring(inputs, inputs.map(zsOf), s0, s1);
    const add: NewEntity[] = [];
    let multi = 0;
    let unread = 0;
    let empty = 0;
    pieces.forEach((r, i) => {
      if (r) add.push(placed(r, v.layer.id, { ...inputs[i].attrs }));
      else if (inputs[i].kind === 'polyline' && inputs[i].parts?.length) multi++;
      else if (s0[i] === null || s1[i] === null) unread++;
      else empty++;
    });
    if (multi) feedback.warn(`${multi} çok parçalı çizgi alınmadı.`);
    if (unread) feedback.warn(`${unread} nesnede uzaklık sayı değil; yazılmadı.`);
    if (empty) feedback.warn(`${empty} nesnede başlangıç bitişten küçük değil; yazılmadı.`);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} nesneden ${add.length} parça yazıldı.`,
    };
  },
});

export const polygonsToLines = defineTool({
  id: 'convert.polygonsToLines',
  label: 'Alanları çizgiye çevir',
  category: 'conversion',
  icon: 'polygonToLine',
  description: 'Alanların halkalarını (dış halka, delikler, parçalar) öznitelikleriyle yeni katmana çizgi olarak yazar.',
  help: [
    'Örnekler: parsel sınırlarını çizgi katmanına almak, adaların ve yapıların dış çizgilerini ayrı çizmek.',
    'Her halka son köşesi ilki olan bir çoklu çizgidir; yaylar ve kotlar kalır. Çizgi alanın özniteliklerine ek Parça (1 …) ve Halka (0 dış halka, 1 … delikler) alır.',
  ].join('\n\n'),
  keywords: ['alan', 'çizgi', 'sınır', 'halka', 'polygons to lines', 'boundary'],
  aliases: ['SINIRCIZGI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: ['polygon'], scopes: CONVERT_SCOPES, description: 'Sınırları çizgiye çevrilecek alanlar.' },
    outputLayer('Sınırlar', '#AD7F58'),
  ] as const,
  outputs: [
    { name: 'lines', label: 'Yazılan çizgiler', type: 'features' },
    { name: 'count', label: 'Çizgi sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Halkalar okunuyor');
    const rings = convertRings(inputs, inputs.map(zsOf));
    const add: NewEntity[] = [];
    rings.forEach((list, i) => {
      for (const r of list) add.push(placed(r, v.layer.id, { ...inputs[i].attrs, Parça: String(r.part), Halka: String(r.ring) }));
    });
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} alandan ${add.length} çizgi yazıldı.`,
    };
  },
});

export const linesToPolygons = defineTool({
  id: 'convert.linesToPolygons',
  label: 'Çizgileri alana çevir',
  category: 'conversion',
  icon: 'lineToPolygon',
  description: 'Kapalı çoklu çizgileri (isterse açıkları kapatarak) öznitelikleriyle yeni katmana alan olarak yazar.',
  help: [
    'Örnekler: sayısallaştırılmış kapalı sınırları alana çevirmek, dosyadan çizgi olarak gelen parselleri alan yapmak.',
    'Uçları en çok 1 µm aralıklı çizgi kapalıdır (çizimin Alana çevir’i); yinelenen son köşe düşer, yaylar ve kotlar kalır. Kapat seçiliyse açık çizginin son köşesi ilkine düz kenarla bağlanır.',
    'Alanı olmayan ve çok parçalı çizgi alan vermez, Atla seçiliyse açık çizgi de yazılmaz; hepsi söylenir. Birbirini kesen çizgilerden alan kurmak Toplu alan’ın işidir.',
  ].join('\n\n'),
  keywords: ['çizgi', 'alan', 'kapat', 'lines to polygons', 'polygonize'],
  aliases: ['CIZGIALAN'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Çizgiler', type: 'features', kinds: ['polyline'], scopes: CONVERT_SCOPES, description: 'Alana çevrilecek çoklu çizgiler.' },
    {
      name: 'open',
      label: 'Açık çizgiler',
      type: 'enum',
      options: [
        { value: 'skip', label: 'Atla', hint: 'Yalnız kapalı çizgiler alan olur' },
        { value: 'close', label: 'Kapat', hint: 'Son köşe ilkine düz kenarla bağlanır' },
      ],
      default: 'skip',
    },
    outputLayer('Alanlar', '#8E4EC6'),
  ] as const,
  outputs: [
    { name: 'areas', label: 'Yazılan alanlar', type: 'features' },
    { name: 'count', label: 'Alan sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Çizgiler kapatılıyor');
    const got = convertLineAreas(inputs, inputs.map(zsOf), v.open === 'close');
    const add: NewEntity[] = [];
    const why = { open: 0, empty: 0, parts: 0 };
    got.forEach((r, i) => {
      if ('none' in r) why[r.none]++;
      else add.push(placed(r, v.layer.id, { ...inputs[i].attrs }));
    });
    if (why.open) feedback.warn(`${why.open} açık çizgi atlandı.`);
    if (why.empty) feedback.warn(`${why.empty} çizginin alanı yok; yazılmadı.`);
    if (why.parts) feedback.warn(`${why.parts} çok parçalı çizgi alınmadı.`);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${inputs.length} çizgiden ${add.length} alan yazıldı.`,
    };
  },
});

/** The most vertices Köşe ekle adds in a run: past them it is refused before anything is built. */
const MOST_ADDED = 10_000_000;

export const densify = defineTool({
  id: 'convert.densify',
  label: 'Köşe ekle',
  category: 'conversion',
  icon: 'densify',
  description: 'Çizgilerin ve alanların her kenarını eşit parçalara bölerek köşe ekler; sonucu yeni katmana yazar.',
  help: [
    'Örnekler: yeniden izdüşümden önce uzun kenarları sıklaştırmak, bir hattı 1 metrede bir köşeli yapmak, her kenarın ortasına köşe koymak.',
    'Aralıkla her kenar, parçaları aralıktan uzun olmayan en az sayıda eşit parçaya bölünür; Sayıyla her kenara o kadar köşe eklenir. Yaylı kenar aynı yayın parçalarına bölünür: yay kalır, yeni köşeler yayın üstündedir.',
    'Delikler ve parçalar da sıklaşır; doğru çoklu çizgi olur. Yeni köşelerin kotları kenarları boyunca taşınır. Bir çalıştırma en çok 10000000 köşe ekler.',
  ].join('\n\n'),
  keywords: ['köşe ekle', 'sıklaştır', 'yoğunlaştır', 'aralık', 'densify', 'densify by interval', 'densify by count'],
  aliases: ['SIKLASTIR', 'KOSEEKLETOPLU'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['line', 'polyline', 'polygon'], scopes: CONVERT_SCOPES, description: 'Köşe eklenecek çizgiler ve alanlar.' },
    {
      name: 'method',
      label: 'Yöntem',
      type: 'enum',
      options: [
        { value: 'interval', label: 'Aralıkla', hint: 'Parçalar aralıktan uzun olmaz' },
        { value: 'count', label: 'Sayıyla', hint: 'Her kenara aynı sayıda köşe' },
      ],
      default: 'interval',
    },
    { name: 'interval', label: 'Aralık', type: 'number', unit: 'm', min: 0.001, max: 1_000_000, default: 1, visibleWhen: (v: { method?: unknown }) => v.method !== 'count', description: 'Kenar parçalarının en uzun boyu.' },
    { name: 'count', label: 'Köşe sayısı', type: 'number', integer: true, min: 1, max: 1000, default: 1, visibleWhen: (v: { method?: unknown }) => v.method === 'count', description: 'Her kenara eklenecek köşe.' },
    outputLayer('Sıklaştırılan', '#E5484D'),
  ] as const,
  outputs: [
    { name: 'objects', label: 'Yazılan nesneler', type: 'features' },
    { name: 'count', label: 'Eklenen köşe sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    const by = v.method === 'count' ? { count: v.count } : { interval: v.interval };
    const total = convertDensifyCount(inputs, by);
    if (total > MOST_ADDED) return { refused: `Bu ayarla ${total} köşe eklenirdi; bir çalıştırma en çok ${MOST_ADDED} köşe ekler. Aralığı büyütün ya da sayıyı küçültün.` };
    feedback.progress(0, 'Köşeler ekleniyor');
    const got = convertDensify(inputs, inputs.map(zsOf), by);
    const add = got.map((r, i) => placed(r, v.layer.id, { ...inputs[i].attrs }));
    return {
      changes: { add },
      outputs: { count: total },
      summary: `${inputs.length} nesneye ${total} köşe eklendi.`,
    };
  },
});
