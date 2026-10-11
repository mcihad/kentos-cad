import type { NewEntity } from '../../../model/entities';
import { lrsEvents, lrsOverlay, type EventIn, type Fault, type LineEvent } from '../../../model/ops/lrs';
import type { DefaultsContext } from '../../types';
import { defineTool } from '../../types';
import { missing } from '../joinSource';
import { LRS_SCOPES, ROUTE_KINDS, againText, cellReason, eventTable, measureText, outputLayer, polylineOf, readMeasure, routesById, sourceParams } from './shared';

/**
 * Olayları rotada göster and Olayları çakıştır (docs/adr/0224 §5; ArcGIS Make Route Event Layer and Overlay Route
 * Events): a table's point and line events put on their routes as objects, and two tables of line events cut where any
 * begins or ends. The routes' rules are the core's (`ops::lrs`); the desktop's twin is `builtin/lrs/events.rs`.
 */

/** Why an event gives no place, in the table. */
export function faultText(units: DefaultsContext, f: Fault): string {
  switch (f.fault) {
    case 'outside':
      return `Ölçü rotanın dışında (${measureText(units, f.least)} – ${measureText(units, f.most)})`;
    case 'noMeasures':
      return 'Rotanın ölçüsü yok';
    case 'same':
      return 'Başlangıç ve bitiş ölçüsü aynı';
    case 'offset':
      return 'Öteleme yapılamadı';
    case 'noRoute':
      return 'Rota yok';
    case 'noMeasure':
      return 'Ölçü yok';
    case 'fewPoints':
      return 'İkiden az nokta';
  }
}

const column = (name: string, label: string, of: string, description?: string, optional = false) =>
  ({
    name,
    label,
    type: 'field',
    of: of ? [`${of}Source`, `${of}File`] : ['source', 'file'],
    ...(optional && { optional: true }),
    ...(description && { description }),
  }) as const;

export const routeEvents = defineTool({
  id: 'linearRef.routeEvents',
  label: 'Olayları rotada göster',
  category: 'linearRef',
  icon: 'routeEvents',
  description: 'Bir tablonun rota ve ölçüyle verilen nokta ve çizgi olaylarını rotaların üstünde nesne olarak yazar.',
  help: [
    'Örnekler: kaza kayıtlarının kilometreleri, yol kaplamasının başlangıç ve bitiş kilometreleri, trafik levhaları, bakım kesimleri.',
    'Nokta olayı rotanın o ölçüdeki yerine, çizgi olayı başlangıç ölçüsünden bitiş ölçüsüne rotanın parçası olarak yazılır (ters verilen olay ters çizilir; rotanın boşluğunda parçalara ayrılır). Öteleme sütunu verilirse olay rotaya dik o kadar kaydırılır: artı öteleme ölçü yönünün sağına (ya da seçilirse soluna).',
    'Kaynak bir dosya (ilk satırı sütun adları) ya da nesneler olabilir; nesnelerde öznitelikler satırdır. Olay satırın dolu hücrelerini öznitelik olarak alır. Rotası olmayan, ölçüsü sayı olmayan ya da rotanın ölçüleri dışında kalan satır atlanır ve tabloda nedeniyle söylenir.',
  ].join('\n\n'),
  keywords: ['olay', 'event', 'route event', 'rota', 'ölçü', 'kilometre', 'doğrusal referans', 'make route event layer'],
  aliases: ['OLAYGOSTER', 'ROUTEEVENTS'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'routes', label: 'Rotalar', type: 'features', kinds: ROUTE_KINDS, scopes: LRS_SCOPES },
    { name: 'routeField', label: 'Rota kimliği alanı', type: 'field', of: 'routes' },
    ...sourceParams('sourceKind', 'file', 'source', 'Olay'),
    column('eventRoute', 'Olayın rota sütunu', ''),
    {
      name: 'eventKind',
      label: 'Olay türü',
      type: 'enum',
      options: [
        { value: 'point', label: 'Nokta', hint: 'Bir ölçü' },
        { value: 'line', label: 'Çizgi', hint: 'Başlangıç ve bitiş ölçüsü' },
      ],
      default: 'point',
    },
    { ...column('measure', 'Ölçü sütunu', ''), visibleWhen: (v: { eventKind?: unknown }) => v.eventKind !== 'line' },
    { ...column('from', 'Başlangıç ölçüsü sütunu', ''), visibleWhen: (v: { eventKind?: unknown }) => v.eventKind === 'line' },
    { ...column('to', 'Bitiş ölçüsü sütunu', ''), visibleWhen: (v: { eventKind?: unknown }) => v.eventKind === 'line' },
    column('offset', 'Öteleme sütunu', '', 'Rotaya dik uzaklık (m); boşsa olay rotanın üstündedir.', true),
    {
      name: 'side',
      label: 'Artı öteleme',
      type: 'enum',
      options: [
        { value: 'right', label: 'Sağa', hint: 'Ölçü yönünün sağı' },
        { value: 'left', label: 'Sola', hint: 'Ölçü yönünün solu' },
      ],
      default: 'right',
    },
    outputLayer('Olaylar', '#8E4EC6'),
  ] as const,
  outputs: [
    { name: 'count', label: 'Yazılan olay sayısı', type: 'number' },
    { name: 'table', label: 'Atlanan satırlar', type: 'table' },
  ],
  run: (v, ctx, feedback) => {
    const t = eventTable(v.sourceKind, v.file, v.source);
    const line = v.eventKind === 'line';
    const columns = [v.eventRoute ?? '', ...(line ? [v.from ?? '', v.to ?? ''] : [v.measure ?? ''])];
    const offsetColumn = (v.offset ?? '').trim();
    for (const c of [...columns, ...(offsetColumn ? [offsetColumn] : [])]) if (!t.header.includes(c)) return { refused: missing(c, t.header) };
    const at = columns.map((c) => t.header.indexOf(c));
    const off = offsetColumn ? t.header.indexOf(offsetColumn) : -1;
    const routes = routesById(v.routes.entities, v.routeField ?? '');
    if (routes.again) feedback.warn(againText(routes.again));
    const turn = v.side === 'left' ? -1 : 1;
    // Why a row gave nothing, by its place: a cell's first, then the core's.
    const reasons = new Map<number, string>();
    const events: EventIn[] = [];
    const rowOf: number[] = [];
    t.rows.forEach((r, i) => {
      const cell = (k: number) => r[k] ?? '';
      const id = cell(at[0]).trim();
      const nums: number[] = [];
      let why: string | null = null;
      for (let k = 1; k < at.length && why === null; k++) {
        const n = readMeasure(cell(at[k]));
        if (n === null) why = cellReason(columns[k], cell(at[k]));
        else nums.push(n);
      }
      let offset = 0;
      if (why === null && off >= 0 && cell(off).trim()) {
        const n = readMeasure(cell(off));
        if (n === null) why = cellReason(offsetColumn, cell(off));
        else offset = n;
      }
      const route = routes.index.get(id);
      if (why === null && route === undefined) why = `“${id}” rotası yok`;
      if (why !== null) {
        reasons.set(i, why);
        return;
      }
      events.push(line ? { route: route!, from: nums[0], to: nums[1], offset: offset * turn } : { route: route!, m: nums[0], offset: offset * turn });
      rowOf.push(i);
    });
    const placed = lrsEvents(routes.routes, events);
    const add: NewEntity[] = [];
    placed.forEach((out, k) => {
      const i = rowOf[k];
      if (out.fault != null) {
        reasons.set(i, faultText(ctx.units, out.fault));
        return;
      }
      const attrs: Record<string, string> = {};
      t.header.forEach((h, c) => {
        const value = t.rows[i][c] ?? '';
        if (value.trim()) attrs[h] = value;
      });
      add.push(out.p != null ? ({ kind: 'point', layerId: v.layer.id, attrs, p: out.p } as NewEntity) : polylineOf(out.parts ?? [], v.layer.id, attrs));
    });
    const rows = [...reasons.entries()].sort((a, b) => a[0] - b[0]).map(([i, why]) => [t.labels[i], why]);
    if (rows.length) feedback.warn(`${rows.length} satır atlandı; nedenleri tabloda.`);
    return {
      changes: { add },
      outputs: rows.length ? { count: add.length, table: { columns: ['Satır', 'Neden'], rows } } : { count: add.length },
      summary: `${t.rows.length} satırdan ${add.length} olay yazıldı.`,
    };
  },
});

export const overlayEvents = defineTool({
  id: 'linearRef.overlayEvents',
  label: 'Olayları çakıştır',
  category: 'linearRef',
  icon: 'routeOverlay',
  description: 'İki çizgi olayı tablosunu aynı rotalarda keser; ortak (Kesişim) ya da bütün (Birleşim) parçaların tablosunu verir.',
  help: [
    'Örnekler: kaplama türü ile hız sınırının birlikte geçerli olduğu kesimler, kaza kesimleriyle bakım kesimlerinin ortak parçaları, iki envanterin birleşimi.',
    'Olaylar bir olayın başladığı ya da bittiği her ölçüde bölünür. Kesişim iki tablonun da kapladığı, Birleşim herhangi birinin kapladığı parçaları verir; bir parçayı kaplayan her olay çifti bir satırdır, aynı çiftin komşu parçaları birleşir.',
    'Tablo Rota, Başlangıç ve Bitiş ile birinci tablonun, sonra ikinci tablonun öbür sütunlarını taşır (birincide de olan ad “ (2)” ekiyle). Rotası boş ya da ölçüsü sayı olmayan satır alınmaz. Tablo Kaydet ile XLSX, ODS ya da CSV olarak kaydedilir ve yeniden Olayları rotada göster\'e verilebilir.',
  ].join('\n\n'),
  keywords: ['olay', 'çakıştır', 'kesişim', 'birleşim', 'overlay route events', 'doğrusal referans', 'kesim'],
  aliases: ['OLAYCAKISTIR', 'OVERLAYEVENTS'],
  targets: ['client', 'worker'],
  parameters: [
    ...sourceParams('aSourceKind', 'aFile', 'aSource', 'Birinci tablo'),
    column('aRoute', 'Birinci tablonun rota sütunu', 'a'),
    column('aFrom', 'Birinci tablonun başlangıç sütunu', 'a'),
    column('aTo', 'Birinci tablonun bitiş sütunu', 'a'),
    ...sourceParams('bSourceKind', 'bFile', 'bSource', 'İkinci tablo'),
    column('bRoute', 'İkinci tablonun rota sütunu', 'b'),
    column('bFrom', 'İkinci tablonun başlangıç sütunu', 'b'),
    column('bTo', 'İkinci tablonun bitiş sütunu', 'b'),
    {
      name: 'how',
      label: 'İşlem',
      type: 'enum',
      options: [
        { value: 'intersect', label: 'Kesişim', hint: 'İki tablonun da kapladığı parçalar' },
        { value: 'union', label: 'Birleşim', hint: 'Herhangi birinin kapladığı parçalar' },
      ],
      default: 'intersect',
    },
  ] as const,
  outputs: [
    { name: 'count', label: 'Parça sayısı', type: 'number' },
    { name: 'table', label: 'Parçalar', type: 'table' },
  ],
  run: (v, ctx, feedback) => {
    const sides = [
      { t: eventTable(v.aSourceKind, v.aFile, v.aSource), keys: [v.aRoute ?? '', v.aFrom ?? '', v.aTo ?? ''] },
      { t: eventTable(v.bSourceKind, v.bFile, v.bSource), keys: [v.bRoute ?? '', v.bFrom ?? '', v.bTo ?? ''] },
    ];
    for (const { t, keys } of sides) for (const c of keys) if (!t.header.includes(c)) return { refused: missing(c, t.header) };
    let dropped = 0;
    const read = sides.map(({ t, keys }) => {
      const at = keys.map((c) => t.header.indexOf(c));
      const events: LineEvent[] = [];
      const kept: (readonly string[])[] = [];
      for (const r of t.rows) {
        const id = (r[at[0]] ?? '').trim();
        const from = readMeasure(r[at[1]] ?? '');
        const to = readMeasure(r[at[2]] ?? '');
        if (!id || from === null || to === null) {
          dropped++;
          continue;
        }
        events.push({ route: id, from, to });
        kept.push(r);
      }
      const others = t.header.filter((h) => !keys.includes(h));
      return { events, kept, header: t.header, others };
    });
    if (dropped) feedback.warn(`${dropped} satırın rotası ya da ölçüsü yok; alınmadı.`);
    const [a, b] = read;
    const pieces = lrsOverlay(a.events, b.events, v.how === 'union');
    const columns = ['Rota', 'Başlangıç', 'Bitiş', ...a.others, ...b.others.map((h) => (a.others.includes(h) ? `${h} (2)` : h))];
    const rows = pieces.map((g) => {
      const row = [g.route, measureText(ctx.units, g.from), measureText(ctx.units, g.to)];
      for (const [idx, side] of [
        [g.a, a],
        [g.b, b],
      ] as const)
        for (const h of side.others) row.push(idx == null ? '' : (side.kept[idx][side.header.indexOf(h)] ?? ''));
      return row;
    });
    return {
      outputs: { count: rows.length, table: { columns, rows } },
      summary: `${a.events.length} ve ${b.events.length} olaydan ${rows.length} parça.`,
    };
  },
});
