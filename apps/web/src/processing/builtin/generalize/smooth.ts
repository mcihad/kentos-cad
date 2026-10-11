import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeSmooth } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { outputLayer, placed, zsOf } from '../convert/shared';
import { GENERALIZE_SCOPES, degreesParam, finish } from './shared';

/**
 * Yumuşat (docs/adr/0221 §2; QGIS Smooth): Chaikin's corner cutting on lines and areas, the neighbours' shared edges
 * together when asked; elevations go along. The core's `ops::generalize::smooth`.
 */

export const smooth = defineTool({
  id: 'generalize.smooth',
  label: 'Yumuşat',
  category: 'generalization',
  icon: 'smoothLine',
  description: 'Çizgilerin ve alanların köşelerini keserek yumuşatır (Chaikin); komşu alanların ortak kenarları birlikte yumuşayabilir.',
  help: [
    'Örnekler: sayısallaştırılmış dere ve yolları yumuşatmak, küçük ölçekli haritada kıyı ve orman sınırlarını yuvarlamak.',
    "Her yinelemede her düz kenar iki ucundan Kesim oranı kadar içeride iki nokta verir; köşe kesilir. Açık çizginin uçları, yayların uçları ve dönüşü En büyük açı'dan büyük köşeler olduğu gibi kalır; yaylar bütün kalır. Kotlar kenar boyunca aynı oranla taşınır.",
    'Ortak kenarları koru açıkken komşu alanların ortak kenarları bir kez yumuşar, iki komşu da aynı sonucu alır; üç ya da daha çok alanın buluştuğu köşeler kıpırdamaz. Komşular ortak köşelerini tam aynı koordinatla paylaşmalıdır.',
  ].join('\n\n'),
  keywords: ['yumuşat', 'düzgünleştir', 'chaikin', 'smooth', 'kıvrım', 'genelleştir'],
  aliases: ['YUMUSAT', 'SMOOTHLINES'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['polygon', 'polyline'], scopes: GENERALIZE_SCOPES, description: 'Yumuşatılacak çoklu çizgiler ve alanlar.' },
    { name: 'iterations', label: 'Yineleme', type: 'number', integer: true, min: 1, max: 10, default: 3, description: 'Köşe kesmenin kaç kez yineleneceği.' },
    { name: 'offset', label: 'Kesim oranı', type: 'number', min: 0.01, max: 0.5, default: 0.25, description: "Kenarın her ucundan kesilen pay; 0,5'te kenarların ortaları kalır." },
    degreesParam('maxAngle', 'En büyük açı', 180, 180, 'Bundan çok dönen köşeler keskin kalır.'),
    { name: 'shared', label: 'Ortak kenarları koru', type: 'boolean', default: false, description: 'Komşu alanların ortak kenarları bir kez yumuşar.' },
    outputLayer('Yumuşatılan', '#0090FF'),
  ] as const,
  outputs: [
    { name: 'smoothed', label: 'Yazılan nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Köşeler kesiliyor');
    const results = generalizeSmooth(inputs.map(entityGeometry), inputs.map(zsOf), v.iterations ?? 3, v.offset ?? 0.25, v.maxAngle ?? 180, v.shared);
    const add: NewEntity[] = [];
    let before = 0;
    let after = 0;
    results.forEach((r, i) => {
      if (!r) return;
      before += r.vertices[0];
      after += r.vertices[1];
      add.push(placed({ shape: r.shape, zs: r.zs ?? undefined }, v.layer.id, { ...inputs[i].attrs }));
    });
    return finish(add, `${add.length} nesne yazıldı; köşe sayısı ${before} → ${after}.`, add.length);
  },
});
