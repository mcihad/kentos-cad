import type { NewEntity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import { orderOf, type OrderKey } from '../../../model/ops/order';
import { defineTool } from '../../types';
import { CONVERT_SCOPES, outputLayer } from './shared';

/**
 * Noktalardan çizgi (docs/adr/0218 §4; ArcGIS Points To Line, QGIS Points to path): points joined into a line in
 * an expression's order (the core's `ops::order`, ascending, empty values last, ties in the input's order), a line a
 * value of the group field; a point in the same place as the one before it is left out, a group with too few points
 * gives none; with Kapat an area. The line's attributes: the group field and its value, Nokta sayısı and the first and
 * last points' order values; its vertices' elevations the points'.
 */

export const pointsToPath = defineTool({
  id: 'convert.pointsToPath',
  label: 'Noktalardan çizgi',
  category: 'conversion',
  icon: 'pointsToPath',
  description: 'Noktaları bir ifadenin sırasıyla birleştirip çizgi (isterse kapalı alan) yapar; bir alanın her değeri ayrı bir çizgi olur.',
  help: [
    'Örnekler: GNSS ile alınmış saha noktalarından zaman sırasıyla güzergâh çizmek, numaralı sınır noktalarından parsel sınırı kurmak, her hat kodunun noktalarını ayrı çizgi yapmak.',
    'Sıra boşsa noktalar girdinin sırasıyla birleşir; değeri olmayan noktalar sona kalır, eşitler girdinin sırasıyla. Grup boşsa bütün noktalar tek çizgidir. Çok noktalı nesnenin noktaları kendi sırasıyla aynı yerdedir.',
    'Bir öncekiyle aynı yerdeki nokta atlanır; iki noktadan (Kapat’ta üçten) az noktalı grup çizgi vermez; ikisi de söylenir. Çizgi grup alanını, Nokta sayısı’nı ve Sıra verilmişse ilk ve son noktanın sıra değerini (Başlangıç, Bitiş) alır; köşelerin kotları noktalarınkidir.',
  ].join('\n\n'),
  keywords: ['nokta', 'çizgi', 'yol', 'güzergâh', 'birleştir', 'sırala', 'points to line', 'points to path'],
  aliases: ['NOKTADANCIZGI'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Noktalar', type: 'features', kinds: ['point'], scopes: CONVERT_SCOPES, description: 'Birleştirilecek noktalar.' },
    { name: 'order', label: 'Sıra', type: 'expression', returns: 'value', of: 'input', optional: true, placeholder: 'No', description: 'Noktaların sırası; boşsa girdinin sırası.' },
    { name: 'group', label: 'Grup', type: 'field', of: 'input', optional: true, description: 'Her değeri ayrı bir çizgi; boşsa tek çizgi.' },
    { name: 'close', label: 'Kapat', type: 'boolean', default: false, description: 'Son nokta ilkine bağlanır; kapalı alan yazılır.' },
    outputLayer('Yollar', '#30A46C'),
  ] as const,
  outputs: [
    { name: 'paths', label: 'Yazılan çizgiler', type: 'features' },
    { name: 'count', label: 'Çizgi sayısı', type: 'number' },
  ],
  run: (v, ctx, feedback) => {
    const inputs = v.input.entities;
    feedback.progress(0, 'Sıralanıyor');
    const objects = { entities: inputs, layerName: ctx.layerName, geometry: ctx.geometry, layers: ctx.layers };
    const values = v.order ? v.order.evaluateAll(objects, 'value') : null;
    const texts = v.order ? v.order.evaluateAll(objects, 'text') : null;
    const order = values ? orderOf(inputs.map((_, i): OrderKey => values.value(i)), false, false) : inputs.map((_, i) => i);
    const field = (v.group ?? '').trim();
    // The groups in the order their values first come in the input.
    const groupOf = inputs.map((e) => (field ? (Object.hasOwn(e.attrs, field) ? e.attrs[field] : '').trim() || '(boş)' : ''));
    const groups = new Map<string, { pts: Vec2[]; zs: (number | null)[]; first: number; last: number }>();
    for (const g of groupOf) if (!groups.has(g)) groups.set(g, { pts: [], zs: [], first: -1, last: -1 });
    let repeats = 0;
    for (const i of order) {
      const e = inputs[i];
      if (e.kind !== 'point') continue;
      const g = groups.get(groupOf[i])!;
      for (const [p, z] of [[e.p, e.z], ...(e.parts ?? []).map((q) => [q.p, q.z] as const)] as const) {
        const prev = g.pts[g.pts.length - 1];
        if (prev && prev.x === p.x && prev.y === p.y) {
          repeats++;
          continue;
        }
        if (g.first < 0) g.first = i;
        g.last = i;
        g.pts.push(p);
        g.zs.push(z ?? null);
      }
    }
    const need = v.close ? 3 : 2;
    const add: NewEntity[] = [];
    let short = 0;
    for (const [value, g] of groups) {
      if (g.pts.length < need) {
        short++;
        continue;
      }
      const attrs: Record<string, string> = {};
      if (field) attrs[field] = value;
      attrs['Nokta sayısı'] = String(g.pts.length);
      if (texts) {
        attrs['Başlangıç'] = String(texts.value(g.first) ?? '');
        attrs['Bitiş'] = String(texts.value(g.last) ?? '');
      }
      const zs = g.zs.some((z) => z !== null) ? { zs: g.zs } : {};
      add.push({ kind: v.close ? 'polygon' : 'polyline', layerId: v.layer.id, attrs, pts: g.pts, ...zs } as NewEntity);
    }
    if (repeats) feedback.warn(`${repeats} nokta bir öncekiyle aynı yerde olduğu için atlandı.`);
    if (short) feedback.warn(`${short} grupta çizgi için yeterli nokta yok (en az ${need}).`);
    const points = inputs.reduce((n, e) => n + (e.kind === 'point' ? 1 + (e.parts?.length ?? 0) : 0), 0);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${points} noktadan ${add.length} çizgi yazıldı.`,
    };
  },
});
