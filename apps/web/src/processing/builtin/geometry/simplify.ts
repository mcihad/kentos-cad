import { fixed } from '../../../core/displayNumber';
import { geoSimplify } from '../../../model/ops/geoprocess';
import { defineTool, type TableOutput } from '../../types';
import { GEO_SCOPES, SIMPLIFY_KINDS, inputNotes, newObject, outputStyle } from './shared';

/**
 * Sadeleştir (docs/adr/0201 §7; ArcGIS Simplify Polygon and Line, QGIS Sadeleştir): Sadeleştir's rule (ADR 0140:
 * a vertex within the tolerance of the chord that would replace it goes, arc edges stay whole) on areas and polylines,
 * written with their attributes to the output layer; the report says each changed object's vertices, its area's change
 * and the largest deviation.
 */

const COLUMNS = ['Nesne', 'Köşe (önce)', 'Köşe (sonra)', 'Alan değişimi (m²)', 'Alan değişimi (%)', 'En büyük sapma (m)'];

export const geometrySimplify = defineTool({
  id: 'geometry.simplify',
  label: 'Sadeleştir',
  category: 'geometry',
  icon: 'geoSimplify',
  description: 'Alan ve çoklu çizgilerden toleranstan az sapan köşeleri atarak yeni katmana yazar; köşe sayılarını, alan değişimini ve en büyük sapmayı tabloda verir.',
  help: [
    'Çizimdeki Sadeleştir’in kuralıdır: atılacak köşe, onun yerine geçecek doğrudan toleranstan az uzaktır; yay kenarlar bütün kalır, alan en az üç köşesini korur.',
    'Her nesne sonuçta bir kez yazılır (değişmeyen olduğu gibi). Komşu alanların ortak sınırları ayrı ayrı sadeleşir; ortak sınırı koruyan sadeleştirme ayrı bir iştir.',
  ].join('\n\n'),
  keywords: ['sadeleştir', 'basitleştir', 'genelleştir', 'simplify', 'generalize', 'douglas', 'köşe azalt'],
  aliases: ['SADELESTIRKATMAN', 'GENERALIZE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: SIMPLIFY_KINDS, scopes: GEO_SCOPES, description: 'Sadeleştirilecek kapalı alanlar ve çoklu çizgiler.' },
    { name: 'tolerance', label: 'Tolerans', type: 'number', unit: 'm', min: 0.001, max: 1000, default: 0.1, description: 'Bundan az sapan köşeler atılır.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Sadeleştirilen' }, newLayerStyle: outputStyle('#AD7F58'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'simplified', label: 'Yazılan nesneler', type: 'features' },
    { name: 'report', label: 'Sadeleştirme raporu', type: 'table' },
    { name: 'count', label: 'Atılan köşe sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const input = v.input.entities;
    inputNotes(feedback, input);
    feedback.progress(0, 'Köşeler atılıyor');
    const results = geoSimplify(input, v.tolerance);
    const { lengthDecimals: ld, areaDecimals: ad } = ctx.units;
    const add = results.map((r, i) => newObject(r.shape, v.layer.id, { ...input[i].attrs }));
    const rows: string[][] = [];
    let removed = 0;
    let worst = 0;
    results.forEach((r, i) => {
      if (!r.changed) return;
      removed += r.vertices[0] - r.vertices[1];
      worst = Math.max(worst, r.deviation);
      const change = r.area ? r.area[1] - r.area[0] : null;
      const share = r.area && change !== null && r.area[0] > 0 ? (change / r.area[0]) * 100 : null;
      rows.push([`#${input[i].id}`, String(r.vertices[0]), String(r.vertices[1]), change === null ? '' : fixed(change, ad), share === null ? '' : fixed(share, 2), fixed(r.deviation, ld)]);
    });
    const report: TableOutput = { columns: COLUMNS, rows };
    return {
      changes: { add },
      outputs: rows.length ? { report, count: removed } : { count: removed },
      summary: rows.length
        ? `${add.length} nesne yazıldı; ${rows.length} nesnede ${removed} köşe atıldı, en büyük sapma ${fixed(worst, ld)} m.`
        : `${add.length} nesne yazıldı; toleranstan az sapan köşe yok.`,
    };
  },
});
