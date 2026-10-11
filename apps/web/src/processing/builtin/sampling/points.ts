import { entityGeometry, type NewEntity } from '../../../model/entities';
import { sampleRandom, sampleRegular, type SampledPoints } from '../../../model/ops/sampling';
import { defineTool } from '../../types';
import { outputLayer, zsOf } from '../convert/shared';
import { AREA_KINDS } from '../geometry/shared';
import { lengthText } from '../proximity/shared';
import { LINE_KINDS, SAMPLE_SCOPES, curvedNote, pointOn, written } from './shared';

/**
 * Düzenli noktalar and Rastgele noktalar (docs/adr/0220 §3, §4; QGIS Regular points, Random points inside polygons
 * and on lines, ArcGIS Create Random Points): a lattice's points in areas, shifted by seeded numbers when asked;
 * seeded random points in areas and on paths, at least so far apart. The points are the core's (`ops::sampling`).
 */

const positive = (x: unknown) => typeof x === 'number' && x > 0;

/** The points' coordinates as Vec2s. */
function at(r: SampledPoints, i: number) {
  return { x: r.xy[2 * i], y: r.xy[2 * i + 1] };
}

export const regularPoints = defineTool({
  id: 'sample.regular',
  label: 'Düzenli noktalar',
  category: 'sampling',
  icon: 'regularPoints',
  description: 'Alanların içine doğu ve kuzey aralıklarıyla düzenli noktalar koyar, isterse noktaları tohumla rastgele kaydırır.',
  help: [
    'Örnekler: orman envanterinin örnek alanlarını, toprak örneklemesinin noktalarını, bir paftanın karelaj noktalarını koymak.',
    'Koordinatlara otur açıkken noktalar aralığın katlarındadır (yuvarlak koordinatlar); kapalıyken bütün alanların kutusunun köşesinden yarım aralık içeride, hücrelerin ortalarındadır. Alanın sınırındaki nokta da alınır, deliklerdeki alınmaz; ortak sınırdaki nokta iki alana da yazılır.',
    'Rastgele kaydırma her noktayı her eksende en çok o kadar kaydırır; aynı tohum aynı noktaları verir. Nokta alanın özniteliklerini alır. En çok 1 000 000 nokta yazılır.',
  ].join('\n\n'),
  keywords: ['düzenli', 'nokta', 'örnekleme', 'karelaj', 'kafes', 'sistematik', 'regular points', 'sampling'],
  aliases: ['DUZENLINOKTA', 'KARELAJNOKTA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: AREA_KINDS, scopes: SAMPLE_SCOPES, description: 'Noktaların konacağı alanlar.' },
    { name: 'dx', label: 'Doğu aralığı', type: 'number', unit: 'm', min: 0.001, default: 10, description: 'Noktalar arası doğu–batı uzaklığı.' },
    { name: 'dy', label: 'Kuzey aralığı', type: 'number', unit: 'm', min: 0.001, default: 10, description: 'Noktalar arası kuzey–güney uzaklığı.' },
    { name: 'snap', label: 'Koordinatlara otur', type: 'boolean', default: true, description: 'Noktalar aralığın katlarında; kapalıyken hücrelerin ortalarında.' },
    { name: 'shift', label: 'Rastgele kaydırma', type: 'number', unit: 'm', min: 0, default: 0, description: 'Her noktanın her eksende en çok kayacağı uzaklık; 0 kaydırmaz.' },
    { name: 'seed', label: 'Tohum', type: 'number', integer: true, min: 0, default: 1, visibleWhen: (v: { shift?: unknown }) => positive(v.shift), description: 'Aynı tohum aynı kaydırmaları verir.' },
    outputLayer('Düzenli noktalar', '#3E63DD'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Noktalar sınanıyor');
    const got = sampleRegular(inputs.map(entityGeometry), v.dx ?? 10, v.dy ?? 10, v.snap, v.shift ?? 0, v.seed ?? 1);
    if ('refused' in got) return { refused: got.refused };
    curvedNote(feedback, inputs);
    const r = got.points;
    const add: NewEntity[] = [];
    const hit = new Set<number>();
    r.at.forEach((o, i) => {
      hit.add(o);
      add.push(pointOn(v.layer.id, { ...inputs[o].attrs }, at(r, i), null));
    });
    const empty = inputs.length - hit.size;
    if (empty) feedback.info(`${empty} alana nokta düşmedi.`);
    return written(add, `${inputs.length} alana ${add.length} nokta yazıldı.`);
  },
});

export const randomPoints = defineTool({
  id: 'sample.random',
  label: 'Rastgele noktalar',
  category: 'sampling',
  icon: 'randomPoints',
  description: 'Alanların içine ya da çizgilerin üstüne nesne başına istenen sayıda rastgele nokta koyar; aynı tohum aynı noktaları verir.',
  help: [
    'Örnekler: tabakalı olmayan rastgele örnekleme noktaları, denetim için rastgele parsel noktaları, bir yol üstünde rastgele gözlem yerleri.',
    'Nokta sayısı nesne başınadır ve ifade olabilir: $alan / 10000 * 5 hektarda beş nokta. En az uzaklık yazılan bütün noktalar arasındadır; sağlanamazsa nokta Deneme kadar yeniden aranır, bulunamazsa yazılmaz ve söylenir. Aynı tohum her çalıştırmada ve her platformda aynı noktaları verir.',
    'Alandaki nokta alanın kutusunda rastgele aranır, kutusuna göre çok ince alan söylenir. Çizgideki nokta yolun uzunluğu boyunca rastgeledir; Uzaklık (yolun başından), çok parçalıda Parça ve köşelerin kotlarından kotunu alır. Nokta nesnenin özniteliklerini alır.',
  ].join('\n\n'),
  keywords: ['rastgele', 'nokta', 'örnekleme', 'tohum', 'random points', 'sampling', 'stokastik'],
  aliases: ['RASTGELENOKTA', 'RANDOMPOINTS'],
  targets: ['client', 'worker'],
  parameters: [
    {
      name: 'input',
      label: 'Nesneler',
      type: 'features',
      kinds: [...AREA_KINDS, ...LINE_KINDS],
      scopes: SAMPLE_SCOPES,
      description: 'Noktaların içine konacağı alanlar ya da üstüne konacağı çizgiler.',
    },
    { name: 'count', label: 'Nokta sayısı', type: 'expression', returns: 'value', of: 'input', default: '10', placeholder: '10', description: 'Nesne başına nokta sayısı (tam kısmı).' },
    { name: 'min', label: 'En az uzaklık', type: 'number', unit: 'm', min: 0, default: 0, description: 'Yazılan iki nokta arası en az uzaklık; 0 sınırsız.' },
    {
      name: 'tries',
      label: 'Deneme',
      type: 'number',
      integer: true,
      min: 1,
      max: 1000,
      default: 10,
      visibleWhen: (v: { min?: unknown }) => positive(v.min),
      description: 'En az uzaklık sağlanamazsa bir nokta için en çok aday.',
    },
    { name: 'seed', label: 'Tohum', type: 'number', integer: true, min: 0, default: 1, description: 'Aynı tohum aynı noktaları verir.' },
    outputLayer('Rastgele noktalar', '#E54666'),
  ] as const,
  outputs: [
    { name: 'points', label: 'Yazılan noktalar', type: 'features' },
    { name: 'count', label: 'Nokta sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Noktalar aranıyor');
    const objects = { entities: inputs, layerName: ctx.layerName, geometry: ctx.geometry, layers: ctx.layers };
    const values = v.count.evaluateAll(objects, 'number');
    const counts = inputs.map((_, i) => {
      const x = values.value(i);
      return typeof x === 'number' && Number.isFinite(x) ? x : null;
    });
    const unread = counts.filter((c) => c === null).length;
    const got = sampleRandom(inputs.map(entityGeometry), inputs.map(zsOf), counts, v.min ?? 0, v.tries ?? 10, v.seed ?? 1);
    if ('refused' in got) return { refused: got.refused };
    const r = got.points;
    if (unread) feedback.warn(`${unread} nesnede nokta sayısı okunamadı; atlandı.`);
    if (r.tooClose) feedback.warn(`${r.tooClose} nokta en az uzaklık sağlanamadığı için yazılmadı.`);
    if (r.thin) feedback.warn(`${r.thin} alan kutusuna göre çok ince: ${r.thinLost} noktası 10 000 denemede bulunamadı.`);
    curvedNote(feedback, inputs);
    const add: NewEntity[] = [];
    r.at.forEach((o, i) => {
      const e = inputs[o];
      const attrs: Record<string, string> = { ...e.attrs };
      const s = r.s?.[i];
      if (s !== null && s !== undefined) {
        attrs.Uzaklık = lengthText(ctx.units, s);
        const part = r.path?.[i] ?? 0;
        if (part > 0) attrs.Parça = String(part);
      }
      add.push(pointOn(v.layer.id, attrs, at(r, i), r.z?.[i]));
    });
    return written(add, `${inputs.length} nesneye ${add.length} nokta yazıldı.`);
  },
});
