import type { NewEntity } from '../../../model/entities';
import { generateDelaunay, generateVoronoi } from '../../../model/ops/generate';
import { defineTool } from '../../types';
import { outputLayer } from '../convert/shared';
import { hasElevation } from '../geometry/shared';
import { nameOf } from '../proximity/shared';
import { GENERATE_SCOPES, sitesOf } from './shared';

/**
 * Voronoi alanları and Delaunay üçgenleri (docs/adr/0219 §2, §3; QGIS Voronoi polygons and Delaunay triangulation,
 * ArcGIS Create Thiessen Polygons): each point's cell inside a frame round the points, and the points' triangles, from
 * the core's exact triangulation (`ops::generate`, docs/adr/0232 §4).
 */

export const voronoi = defineTool({
  id: 'generate.voronoi',
  label: 'Voronoi alanları',
  category: 'generation',
  icon: 'voronoi',
  description: 'Her noktanın, kendisine öbür noktalardan daha yakın yerlerini (Thiessen alanı) öznitelikleriyle yeni katmana yazar.',
  help: [
    'Örnekler: yağış istasyonlarının etki alanları, okulların ya da sağlık ocaklarının hizmet bölgeleri, ölçü noktalarının sorumluluk alanları.',
    'Alanlar noktaların sınır kutusunun her yandan uzun kenarının Genişletme yüzdesi kadar büyütülmüşüne kırpılır; bir sınırın içinde kalsın isteniyorsa ardından Kırp kullanılır. Komşu alanlar ortak köşelerini aynen paylaşır.',
    'Her alan noktasının özniteliklerini alır; çok noktalı nesnenin her noktası bir alandır, Parça 1, 2 … yazılır. Başka bir noktayla aynı yerdeki nokta alan vermez, söylenir.',
  ].join('\n\n'),
  keywords: ['voronoi', 'thiessen', 'etki alanı', 'en yakın', 'bölge', 'hücre'],
  aliases: ['VORONOI', 'THIESSEN'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Noktalar', type: 'features', kinds: ['point'], scopes: GENERATE_SCOPES, description: 'Alanları üretilecek noktalar.' },
    {
      name: 'margin',
      label: 'Genişletme',
      type: 'number',
      unit: '%',
      min: 0,
      max: 1000,
      default: 10,
      description: 'Çerçeve, noktaların sınır kutusunun her yandan uzun kenarının bu yüzdesi kadar büyütülmüşüdür.',
    },
    outputLayer('Voronoi', '#3E63DD'),
  ] as const,
  outputs: [
    { name: 'cells', label: 'Yazılan alanlar', type: 'features' },
    { name: 'count', label: 'Alan sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    const sites = sitesOf(inputs);
    feedback.progress(0, 'Alanlar hesaplanıyor');
    const r = generateVoronoi(
      sites.map((s) => s.p),
      v.margin / 100,
    );
    if ('refused' in r) return { refused: r.refused };
    const add: NewEntity[] = [];
    let same = 0;
    let empty = 0;
    r.cells.forEach((c, i) => {
      if (c === 'same') same += 1;
      else if (c === null) empty += 1;
      else {
        const s = sites[i];
        const attrs = s.part === null ? { ...inputs[s.at].attrs } : { ...inputs[s.at].attrs, Parça: String(s.part) };
        add.push({ kind: 'polygon', layerId: v.layer.id, attrs, pts: c });
      }
    });
    if (same) feedback.warn(`${same} nokta başka bir noktayla aynı yerde; alanı yazılmadı.`);
    const heights = inputs.filter(hasElevation).length;
    if (heights) feedback.warn(`${heights} nesnenin kotları sonuca taşınmadı.`);
    if (empty) feedback.warn(`${empty} noktanın alanı çerçevenin içinde alan değil; yazılmadı.`);
    return { changes: { add }, outputs: { count: add.length }, summary: `${sites.length} noktadan ${add.length} alan yazıldı.` };
  },
});

export const delaunay = defineTool({
  id: 'generate.delaunay',
  label: 'Delaunay üçgenleri',
  category: 'generation',
  icon: 'delaunay',
  description: 'Noktaların Delaunay üçgenlerini, köşelerinin adları ve kotlarıyla yeni katmana alan olarak yazar.',
  help: [
    'Örnekler: ölçü noktalarından üçgen ağı (TIN) kurmak, komşu noktaları bulmak, yüzeyin yüzlerini görmek.',
    'Üçgenler kesin Delaunay üçgenleridir: hiçbir üçgenin çevrel çemberinde başka nokta yoktur. Aynı çemberdeki dört noktada köşegen girdinin sırasıyla seçilir.',
    'Her üçgen Nokta 1, Nokta 2 ve Nokta 3 alanlarında köşelerinin adlarını taşır: Ad alanındaki değer, boşsa etiket, o da yoksa girdideki sırası; çok noktalı nesnenin noktaları ad/1, ad/2 … Noktaların kotları köşelerin kotu olur. Başka bir noktayla aynı yerdeki nokta bir kez girer, söylenir.',
  ].join('\n\n'),
  keywords: ['delaunay', 'üçgen', 'tin', 'üçgenleme', 'triangulation', 'üçgen ağı'],
  aliases: ['DELAUNAY', 'UCGENLE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Noktalar', type: 'features', kinds: ['point'], scopes: GENERATE_SCOPES, description: 'Üçgenlenecek noktalar.' },
    { name: 'name', label: 'Ad alanı', type: 'field', of: 'input', optional: true, description: 'Köşelerin adları bu alandan; boşsa etiket, o da yoksa sıra.' },
    outputLayer('Üçgenler', '#12A594'),
  ] as const,
  outputs: [
    { name: 'triangles', label: 'Yazılan üçgenler', type: 'features' },
    { name: 'count', label: 'Üçgen sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const inputs = v.input.entities;
    const sites = sitesOf(inputs);
    feedback.progress(0, 'Üçgenler hesaplanıyor');
    const r = generateDelaunay(sites.map((s) => s.p));
    if ('refused' in r) return { refused: r.refused };
    const field = (v.name ?? '').trim();
    const names = sites.map((s) => {
      const n = nameOf(inputs[s.at], field, s.at);
      return s.part === null ? n : `${n}/${s.part}`;
    });
    const add: NewEntity[] = r.triangles.map((t) => {
      const zs = t.map((i) => sites[i].z);
      const attrs = { 'Nokta 1': names[t[0]], 'Nokta 2': names[t[1]], 'Nokta 3': names[t[2]] };
      const e: NewEntity = { kind: 'polygon', layerId: v.layer.id, attrs, pts: t.map((i) => sites[i].p) };
      return zs.some((z) => z !== null) ? ({ ...e, zs } as NewEntity) : e;
    });
    const places = new Set(sites.map((s) => `${s.p.x + 0},${s.p.y + 0}`)).size;
    const same = sites.length - places;
    if (same) feedback.warn(`${same} nokta başka bir noktayla aynı yerde; üçgenlere bir kez girdi.`);
    return { changes: { add }, outputs: { count: add.length }, summary: `${sites.length} noktadan ${add.length} üçgen yazıldı.` };
  },
});
