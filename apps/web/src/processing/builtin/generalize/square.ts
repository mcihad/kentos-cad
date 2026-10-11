import { fixed } from '../../../core/displayNumber';
import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeSquare } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { outputLayer, placed, zsOf } from '../convert/shared';
import { GENERALIZE_SCOPES, degreesParam, finish } from './shared';

/**
 * Dik açılandır (docs/adr/0221 §3; iD's, JOSM's and QGIS's Orthogonalize): corners near a right angle or near straight
 * made exact; the other corners stay, elevations go with their vertices. The core's `ops::generalize::orthogonal`.
 */

export const square = defineTool({
  id: 'generalize.orthogonalize',
  label: 'Dik açılandır',
  category: 'generalization',
  icon: 'orthogonalize',
  description: 'Dik açıya ve düzlüğe yakın köşeleri tam dik ve düz yapar (binalar, parseller); öbür köşeler olduğu gibi kalır.',
  help: [
    'Örnekler: sayısallaştırılmış bina köşelerini dik yapmak, el çizimi yapı adalarını düzeltmek.',
    "Açısı dik açıya ya da düzlüğe Açı eşiği kadar yakın köşeler açıortayları boyunca adım adım kaydırılır (iD'nin ve QGIS'in kuralı); en kareye yakın sonuç yazılır. Daha uzak açılı köşeler (örneğin 45°) kalır. Açık çizginin uçları ve yayların uçları kıpırdamaz; kotlar köşeleriyle gider.",
  ].join('\n\n'),
  keywords: ['dik', 'açı', 'kare', 'orthogonalize', 'bina', 'doksan', 'square'],
  aliases: ['DIKACI', 'ORTHOGONALIZE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: ['polygon', 'polyline'], scopes: GENERALIZE_SCOPES, description: 'Köşeleri dik açılandırılacak alanlar ve çoklu çizgiler.' },
    degreesParam('threshold', 'Açı eşiği', 45, 15, 'Dik açıya ya da düzlüğe bu kadar yakın köşeler düzeltilir.'),
    { name: 'iterations', label: 'En çok yineleme', type: 'number', integer: true, min: 1, max: 10000, default: 1000, description: 'Köşelerin en çok kaç adımda kaydırılacağı.' },
    outputLayer('Dik açılandırılan', '#7C66DC'),
  ] as const,
  outputs: [
    { name: 'squared', label: 'Yazılan nesneler', type: 'features' },
    { name: 'count', label: 'Düzeltilen nesne sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Köşeler düzeltiliyor');
    const results = generalizeSquare(inputs.map(entityGeometry), inputs.map(zsOf), v.threshold ?? 15, v.iterations ?? 1000);
    const add: NewEntity[] = [];
    let moved = 0;
    let worst = 0;
    results.forEach((r, i) => {
      if (!r) return;
      if (r.moved) moved++;
      worst = Math.max(worst, r.shift);
      add.push(placed({ shape: r.shape, zs: r.zs ?? undefined }, v.layer.id, { ...inputs[i].attrs }));
    });
    const summary = moved
      ? `${add.length} nesne yazıldı; ${moved} nesnenin köşeleri dik açılandı, en büyük kayma ${fixed(worst, ctx.units.lengthDecimals)} m.`
      : `${add.length} nesne yazıldı; dik açıya ya da düzlüğe yakın köşe yok.`;
    return finish(add, summary, moved);
  },
});
