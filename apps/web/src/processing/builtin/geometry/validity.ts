import { fixed } from '../../../core/displayNumber';
import { geoRepair, geoValidity, type Repaired } from '../../../model/ops/geoprocess';
import { defineTool, type TableOutput } from '../../types';
import { GEO_SCOPES, VERTEX_KINDS, emptyNote, inputNotes, newObject, outputStyle } from './shared';

/**
 * Geçerliliği denetle and Onar (docs/adr/0201 §6; ArcGIS Check and Repair Geometry, QGIS Geçerliliği denetle and Fix
 * geometries): what is wrong with areas and paths as written, found by the core (`ops::geoprocess::validity`) — a
 * repeated vertex, a ring of no area, a ring or a path that crosses, touches or runs back over itself, a hole out of
 * its ring, holes over each other — each at its first place; and each area rebuilt from its own rings in the core's
 * overlay, each path without its repeated vertices, written to the output layer with a report.
 */

/** Geçerliliği denetle's table: the object, its layer, the problem and where it first shows. */
const PROBLEM_COLUMNS = ['Nesne', 'Katman', 'Sorun', 'Doğu', 'Kuzey'];

export const geometryValidity = defineTool({
  id: 'geometry.validity',
  label: 'Geçerliliği denetle',
  category: 'geometry',
  icon: 'geoValidity',
  description: 'Alan ve çizgilerin geometri sorunlarını bulup tabloda gösterir ve sorunlu nesneleri seçer; çizim değişmez.',
  help: [
    'Bulunan sorunlar: yinelenen köşe (art arda aynı yerde iki köşe), alanı sıfır olan halka (köşeleri bir doğru üstünde), kendini kesen, kendine değen ya da kendi üstünden geri dönen halka ya da yol, dış halkanın dışına taşan delik, örtüşen delikler. Başladığı yerde biten yol kendini kesmiş sayılmaz.',
    'Her sorun her halkada bir kez, ilk göründüğü yerle yazılır; yer tablonun Doğu ve Kuzey sütunlarındadır. Aynı yerde sayılma payı 1 mikrometredir (çekirdeğin örtüşme toleransı); başka gizli yuvarlama yoktur.',
    "Sorunlu nesneleri yeni katmana düzeltilmiş olarak yazmak için Onar'ı kullanın.",
  ].join('\n\n'),
  keywords: ['geçerlilik', 'denetle', 'geometri hatası', 'kendini kesen', 'yinelenen köşe', 'check geometry', 'validity', 'topoloji'],
  aliases: ['GECERLILIK', 'CHECKGEOMETRY'],
  targets: ['client', 'worker'],
  parameters: [{ name: 'input', label: 'Nesneler', type: 'features', kinds: VERTEX_KINDS, scopes: GEO_SCOPES, description: 'Denetlenecek kapalı alanlar, çoklu çizgiler ve çizgiler.' }] as const,
  outputs: [
    { name: 'problems', label: 'Sorunlar', type: 'table' },
    { name: 'invalid', label: 'Sorunlu nesneler', type: 'features' },
    { name: 'count', label: 'Sorun sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const input = v.input.entities;
    feedback.progress(0, 'Geometriler denetleniyor');
    const found = geoValidity(input);
    if (!found.length) return { outputs: { count: 0, invalid: [] }, summary: `${input.length} nesnede sorun bulunmadı.` };
    const d = ctx.units.lengthDecimals;
    const rows = found.map((f) => {
      const e = input[f.index];
      return [`#${e.id}`, ctx.layerName(e.layerId), f.text, fixed(f.at.x, d), fixed(f.at.y, d)];
    });
    const ids = [...new Set(found.map((f) => input[f.index].id))];
    const table: TableOutput = { columns: PROBLEM_COLUMNS, rows };
    return {
      select: ids,
      outputs: { problems: table, invalid: ids, count: found.length },
      summary: `${input.length} nesneden ${ids.length} nesnede ${found.length} sorun bulundu; sorunlu nesneler seçildi.`,
    };
  },
});

/** Onar's report: the object, how it was and how it is, and what was wrong. */
const REPAIR_COLUMNS = ['Nesne', 'Önce', 'Sonra', 'Değişiklik'];

/** An object's state before (0) or after (1) as the report writes it. */
function state(r: Repaired, k: 0 | 1, areaDecimals: number): string {
  if (k === 1 && !r.shape) return 'Boş';
  if (r.area) return `${r.parts[k]} parça, ${r.holes[k]} delik, ${fixed(r.area[k], areaDecimals)} m²`;
  return `${r.vertices[k]} köşe`;
}

export const geometryRepair = defineTool({
  id: 'geometry.repair',
  label: 'Onar',
  category: 'geometry',
  icon: 'geoRepair',
  description: 'Alan ve çizgilerin geometri sorunlarını düzeltip nesneleri öznitelikleriyle yeni katmana yazar; neyin değiştiğini tabloda gösterir.',
  help: [
    'Sorunlu alan kendi halkalarından yeniden kurulur: kendini kesen halka parçalarına ayrılır, dış halkadan taşan delik halkayla kesilir, örtüşen delikler tek delik olur, yinelenen köşeler düşer; alanı kalmayan halka yazılmaz. Çizgilerde yinelenen köşeler düşer; kendini kesen yol onarılmaz, olduğu gibi yazılır.',
    'Sorunu olmayan nesne olduğu gibi yazılır. Tablo her sorunlu nesnenin önceki ve sonraki parça, delik ve alan sayılarını (çizgide köşe sayısını) ve sorunlarını gösterir. Girdi değişmez.',
  ].join('\n\n'),
  keywords: ['onar', 'düzelt', 'geometri hatası', 'repair geometry', 'fix geometries', 'kendini kesen', 'temizle'],
  aliases: ['ONAR', 'REPAIRGEOMETRY'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: VERTEX_KINDS, scopes: GEO_SCOPES, description: 'Onarılacak kapalı alanlar, çoklu çizgiler ve çizgiler.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Onarılan' }, newLayerStyle: outputStyle('#12A594'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'repaired', label: 'Yazılan nesneler', type: 'features' },
    { name: 'report', label: 'Onarım raporu', type: 'table' },
    { name: 'count', label: 'Onarılan nesne sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const input = v.input.entities;
    inputNotes(feedback, input);
    feedback.progress(0, 'Geometriler onarılıyor');
    const results = geoRepair(input);
    const add = results.flatMap((r, i) => (r.shape ? [newObject(r.shape, v.layer.id, { ...input[i].attrs })] : []));
    const rows: string[][] = [];
    let repaired = 0;
    let crossing = 0;
    results.forEach((r, i) => {
      if (!r.problems.length) return;
      if (r.problems.some((p) => p !== 'pathCrossing')) repaired++;
      if (r.problems.includes('pathCrossing')) crossing++;
      const change = r.problems.map((p, k) => (p === 'pathCrossing' ? `${r.texts[k]} (onarılmaz)` : r.texts[k])).join(', ');
      rows.push([`#${input[i].id}`, state(r, 0, ctx.units.areaDecimals), state(r, 1, ctx.units.areaDecimals), change]);
    });
    if (crossing) feedback.warn(`${crossing} kendini kesen yol onarılmadı; olduğu gibi yazıldı.`);
    emptyNote(input.length - add.length, feedback);
    const report: TableOutput = { columns: REPAIR_COLUMNS, rows };
    return {
      changes: { add },
      outputs: rows.length ? { report, count: repaired } : { count: repaired },
      summary: `${add.length} nesne yazıldı; ${repaired} nesne onarıldı.`,
    };
  },
});
