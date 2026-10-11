import { entityGeometry, type NewEntity } from '../../../model/entities';
import { type BoundKind, generateBounding, generateConcave } from '../../../model/ops/generate';
import { defineTool } from '../../types';
import { outputLayer } from '../convert/shared';
import { GEO_KINDS } from '../geometry/shared';
import { areaText, bearingText, lengthText } from '../proximity/shared';
import { emptyNote, GENERATE_SCOPES, groupedSummary, grouping, groupParams, inputNotes } from './shared';

/**
 * Sınırlayıcı geometri and İçbükey zarf (docs/adr/0219 §4, §5; ArcGIS Minimum Bounding Geometry, QGIS Minimum bounding
 * geometry, Convex hull and Concave hull): each object's, all objects' or each field value's convex hull, box, rotated
 * box of least area, least enclosing circle or concave hull, with its measures, from the core (`ops::generate`).
 */

const WHAT: Record<BoundKind, string> = { hull: 'dışbükey zarf', box: 'sınır kutusu', rotated: 'döndürülmüş kutu', circle: 'daire' };

export const bounding = defineTool({
  id: 'generate.bounding',
  label: 'Sınırlayıcı geometri',
  category: 'generation',
  icon: 'boundingGeometry',
  description: 'Nesneleri saran dışbükey zarfı, sınır kutusunu, en küçük döndürülmüş kutuyu ya da en küçük daireyi ölçüleriyle yeni katmana yazar.',
  help: [
    'Örnekler: yapıların döndürülmüş kutusuyla yönünü ve boyunu bulmak, bir nokta kümesinin kapladığı alanı çizmek, adaların sınır kutularını almak, bir kuyunun koruma dairesini çizmek.',
    'Dışbükey zarf noktaları içine alan en küçük dışbükey alandır. Sınır kutusu eksenlere paraleldir. Döndürülmüş kutu en küçük alanlı dikdörtgendir; bir kenarı zarfın bir kenarındadır. Daire en küçük çevreleyen dairedir.',
    'Gruplama Her nesne iken her nesnenin, Hepsi iken bütün nesnelerin, Alana göre iken alanın değeri aynı olanların sınırlayıcısı yazılır. Ölçüler: Zarf alanı; Kutu genişliği ve Kutu yüksekliği; Kutu uzunluğu, Kutu genişliği ve uzun kenarın Kutu semti; Daire yarıçapı. Yaylar, elipsler ve eğriler 0,1 mm içinde noktalarla alınır. Alanı olmayan sonuç (bir nokta, bir doğru) yazılmaz.',
  ].join('\n\n'),
  keywords: ['sınırlayıcı', 'dışbükey', 'zarf', 'kutu', 'döndürülmüş', 'daire', 'convex hull', 'bounding', 'envelope', 'mbr'],
  aliases: ['SINIRLAYICI', 'ZARF', 'MBG'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GENERATE_SCOPES, description: 'Sarılacak nesneler.' },
    {
      name: 'kind',
      label: 'Tür',
      type: 'enum',
      options: [
        { value: 'hull', label: 'Dışbükey zarf', hint: 'Noktaları saran en küçük dışbükey alan' },
        { value: 'box', label: 'Sınır kutusu', hint: 'Eksenlere paralel dikdörtgen' },
        { value: 'rotated', label: 'Döndürülmüş kutu', hint: 'En küçük alanlı dikdörtgen' },
        { value: 'circle', label: 'Daire', hint: 'En küçük çevreleyen daire' },
      ],
      default: 'hull',
    },
    ...groupParams('each'),
    outputLayer('Sınırlayıcı', '#F76B15'),
  ] as const,
  outputs: [
    { name: 'bounds', label: 'Yazılan sınırlayıcılar', type: 'features' },
    { name: 'count', label: 'Sınırlayıcı sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const kind = v.kind as BoundKind;
    const g = grouping(inputs, v.group, v.field ?? '');
    feedback.progress(0, 'Sınırlayıcılar hesaplanıyor');
    // Geometry only: the attributes would only cross into the core to be read and dropped.
    const r = generateBounding(inputs.map(entityGeometry), g.groups, g.count, kind);
    inputNotes(feedback, inputs, r.curved);
    const add: NewEntity[] = [];
    let empty = 0;
    r.list.forEach((b, k) => {
      if (!b) return void (empty += 1);
      const attrs: Record<string, string> = { ...g.attrs[k] };
      if (b.area !== undefined) attrs['Zarf alanı'] = areaText(ctx.units, b.area);
      if (b.height !== undefined && b.width !== undefined) {
        attrs['Kutu genişliği'] = lengthText(ctx.units, b.width);
        attrs['Kutu yüksekliği'] = lengthText(ctx.units, b.height);
      }
      if (b.length !== undefined && b.width !== undefined && b.bearing !== undefined) {
        attrs['Kutu uzunluğu'] = lengthText(ctx.units, b.length);
        attrs['Kutu genişliği'] = lengthText(ctx.units, b.width);
        attrs['Kutu semti'] = bearingText(ctx.units, b.bearing);
      }
      if (b.r !== undefined) attrs['Daire yarıçapı'] = lengthText(ctx.units, b.r);
      add.push({ ...b.shape, layerId: v.layer.id, attrs } as NewEntity);
    });
    emptyNote(feedback, empty);
    return { changes: { add }, outputs: { count: add.length }, summary: groupedSummary(inputs.length, g.count, v.group, add.length, WHAT[kind]) };
  },
});

export const concaveHull = defineTool({
  id: 'generate.concaveHull',
  label: 'İçbükey zarf',
  category: 'generation',
  icon: 'concaveHull',
  description: 'Nesnelerin noktalarını saran, içe doğru oyulmuş zarfı (isteğe bağlı delikleriyle) alanıyla yeni katmana yazar.',
  help: [
    'Örnekler: bir yerleşimin evlerinin kapladığı alanı, bir nokta bulutunun ya da kümenin gerçek biçimini çizmek.',
    'Noktaların Delaunay üçgenleri dıştan içe aşındırılır: sınırdaki kenarı en uzun olan üçgen önce gider, kenarı Oran’ın verdiği uzunluktan kısa olunca durulur (GEOS’un ve JTS’in kuralı). Oran 0 en oyuk zarfı, 1 dışbükey zarfı verir. Alan tek parça kalır.',
    'Delikler açıkken içteki uzun kenarlı üçgenler sınıra değmeden delik açar. Gruplama ve öznitelikler Sınırlayıcı geometri’ninki gibidir; Zarf alanı yazılır.',
  ].join('\n\n'),
  keywords: ['içbükey', 'zarf', 'concave hull', 'alfa', 'kapladığı alan', 'küme sınırı'],
  aliases: ['ICBUKEYZARF', 'CONCAVEHULL'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GENERATE_SCOPES, description: 'Noktaları sarılacak nesneler.' },
    { name: 'ratio', label: 'Oran', type: 'number', min: 0, max: 1, default: 0.3, description: '0 en oyuk, 1 dışbükey zarf.' },
    { name: 'holes', label: 'Delikler', type: 'boolean', default: false, description: 'Zarfın içinde delikler açılabilir.' },
    ...groupParams('all'),
    outputLayer('İçbükey zarf', '#8E4EC6'),
  ] as const,
  outputs: [
    { name: 'hulls', label: 'Yazılan zarflar', type: 'features' },
    { name: 'count', label: 'Zarf sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    const g = grouping(inputs, v.group, v.field ?? '');
    feedback.progress(0, 'Zarflar hesaplanıyor');
    const r = generateConcave(inputs.map(entityGeometry), g.groups, g.count, v.ratio, v.holes);
    inputNotes(feedback, inputs, r.curved);
    const add: NewEntity[] = [];
    let empty = 0;
    r.list.forEach((h, k) => {
      if (!h) return void (empty += 1);
      add.push({ ...h.shape, layerId: v.layer.id, attrs: { ...g.attrs[k], 'Zarf alanı': areaText(ctx.units, h.area) } } as NewEntity);
    });
    emptyNote(feedback, empty);
    return { changes: { add }, outputs: { count: add.length }, summary: groupedSummary(inputs.length, g.count, v.group, add.length, 'içbükey zarf') };
  },
});
