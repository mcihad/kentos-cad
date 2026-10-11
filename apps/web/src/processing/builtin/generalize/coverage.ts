import { fixed } from '../../../core/displayNumber';
import { entityGeometry, type NewEntity } from '../../../model/entities';
import { generalizeCoverage } from '../../../model/ops/generalize';
import { defineTool } from '../../types';
import { outputLayer, placed, zsOf } from '../convert/shared';
import { GENERALIZE_SCOPES, finish, measureParam, table } from './shared';

/**
 * Ortak kenarları koruyarak sadeleştir (docs/adr/0221 §8; GEOS's CoverageSimplifier, QGIS Simplify coverage, PostGIS
 * ST_CoverageSimplify): the coverage's edges simplified once each; neighbours keep sharing their edges. The core's
 * `ops::generalize::simplify`.
 */

const COLUMNS = ['Nesne', 'Köşe (önce)', 'Köşe (sonra)', 'Alan değişimi (m²)', 'Alan değişimi (%)'];

export const coverageSimplify = defineTool({
  id: 'generalize.coverageSimplify',
  label: 'Ortak kenarları koruyarak sadeleştir',
  category: 'generalization',
  icon: 'coverageSimplify',
  description: 'Komşu alanların ortak kenarlarını bir kez sadeleştirir; aralarında boşluk ya da örtüşme açılmaz.',
  help: [
    'Örnekler: il, ilçe ve mahalle sınırlarını küçük ölçeğe indirmek, parsel katmanının köşelerini azaltmak.',
    "Üç ya da daha çok alanın buluştuğu köşeler kalır; aralarındaki kenarlar Visvalingam–Whyatt'la sadeleşir (GEOS'un ve QGIS'in kuralı): üçgeninin alanı toleransın karesinden küçük köşe atılır, üçgeninde başka bir köşe varsa kalır. Sınırı koru açıkken dış sınır değişmez.",
    'Komşular ortak köşelerini tam aynı koordinatla paylaşmalıdır; paylaşmayanların sınırı ayrı ayrı sadeleşir (önce Topolojik temizlik). Kotlar köşeleriyle kalır.',
  ].join('\n\n'),
  keywords: ['sadeleştir', 'ortak kenar', 'kaplama', 'coverage', 'simplify', 'visvalingam', 'genelleştir'],
  aliases: ['KAPLAMASADE', 'COVERAGESIMPLIFY'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Alanlar', type: 'features', kinds: ['polygon'], scopes: GENERALIZE_SCOPES, description: 'Ortak kenarları birlikte sadeleştirilecek komşu alanlar.' },
    measureParam('tolerance', 'Tolerans', 'm', 0.001, 1, 'Üçgeninin alanı bunun karesinden küçük köşeler atılır.'),
    { name: 'keepBoundary', label: 'Sınırı koru', type: 'boolean', default: false, description: 'Yalnız ortak kenarlar sadeleşir; dış sınır değişmez.' },
    outputLayer('Sadeleştirilen kaplama', '#3E63DD'),
  ] as const,
  outputs: [
    { name: 'simplified', label: 'Yazılan alanlar', type: 'features' },
    { name: 'report', label: 'Sadeleştirme raporu', type: 'table' },
    { name: 'count', label: 'Atılan köşe sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Ortak kenarlar sadeleşiyor');
    const results = generalizeCoverage(inputs.map(entityGeometry), inputs.map(zsOf), v.tolerance ?? 1, v.keepBoundary);
    const ad = ctx.units.areaDecimals;
    const add: NewEntity[] = [];
    const rows: string[][] = [];
    let removed = 0;
    results.forEach((r, i) => {
      if (!r) return;
      const [b, a] = r.vertices;
      if (a !== b) {
        removed += Math.max(0, b - a);
        const change = r.area[1] - r.area[0];
        rows.push([`#${inputs[i].id}`, String(b), String(a), fixed(change, ad), fixed((change / r.area[0]) * 100, 2)]);
      }
      add.push(placed({ shape: r.shape, zs: r.zs ?? undefined }, v.layer.id, { ...inputs[i].attrs }));
    });
    const summary = rows.length ? `${add.length} nesne yazıldı; ${rows.length} nesnede ${removed} köşe atıldı.` : `${add.length} nesne yazıldı; atılacak köşe yok.`;
    return finish(add, summary, removed, table(COLUMNS, rows));
  },
});
