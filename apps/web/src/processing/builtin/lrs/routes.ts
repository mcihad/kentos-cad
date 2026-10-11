import type { Entity } from '../../../model/entities';
import { lrsCalibrate, lrsCreate, lrsSummaries, type Piece, type Route, type RoutePart } from '../../../model/ops/lrs';
import { carryElevations } from '../../../model/ops/elevation';
import { elevatedPaths, hasElevation } from '../../../product/elevation';
import type { Feedback, RunContext, RunResult } from '../../types';
import { defineTool } from '../../types';
import { withAttr } from '../attributeWrites';
import { LRS_SCOPES, ROUTE_KINDS, againText, cellReason, measureText, outputLayer, polylineOf, readMeasure, routeOf, routesById } from './shared';

/**
 * Rota oluştur, Rotayı kalibre et and M değerlerini çıkar (docs/adr/0224 §5; ArcGIS Create Routes and Calibrate Routes,
 * QGIS Extract M values): routes made of pieces by their id, routes' measures made anew from points along them, and a
 * route's measures written into its attributes. The routes' rules are the core's (`ops::lrs`); the desktop's twin is
 * `builtin/lrs/routes.rs`.
 */

/** A piece object's paths: a line's ends, a polyline's own part and its parts, an arc from its start to its end. */
function piecePaths(e: Entity): { pts: { x: number; y: number }[]; bulges?: number[] }[] {
  if (e.kind === 'line') return [{ pts: [e.a, e.b] }];
  if (e.kind === 'polyline') return [e, ...(e.parts ?? [])].map((q) => ({ pts: q.pts, ...(q.bulges && { bulges: q.bulges }) }));
  if (e.kind === 'arc') {
    let sweep = e.a1 - e.a0;
    if (sweep <= 0) sweep += 2 * Math.PI;
    const at = (a: number) => ({ x: e.c.x + e.r * Math.cos(a), y: e.c.y + e.r * Math.sin(a) });
    return [{ pts: [at(e.a0), at(e.a1)], bulges: [Math.tan(sweep / 4)] }];
  }
  return [];
}

export const createRoutes = defineTool({
  id: 'linearRef.createRoutes',
  label: 'Rota oluştur',
  category: 'linearRef',
  icon: 'routeCreate',
  description: 'Çizgileri kimliklerine göre uç uca birleştirip köşelerine ölçü (M) veren rotalar yazar.',
  help: [
    'Örnekler: yol parçalarından karayolu rotaları, boru hattı parçalarından hat rotaları, kanal ve dere eksenlerinden ölçülü rotalar.',
    'Aynı kimlikli çizgiler bir rotadır. Uzunluk ve Tek alan yöntemlerinde parçalar başlangıç köşesine en yakın uçtan başlayarak uç uca eklenir; ters çizilen parça çevrilir; değmeyen parçalar rotanın ayrı parçalarıdır ve aralarındaki boşluk istenirse ölçüye katılır. Ölçü çizginin uzunluğundan (Uzunluk) ya da her parçanın kendi uzunluk alanından (Tek alan) gelir; çarpanla çarpılıp başlangıç ölçüsüne eklenir. İki alan yönteminde her parça başlangıç ve bitiş ölçüsü alanlarıyla, başlangıcına göre sırada yazılır.',
    'Yaylar yay olarak kalır; ölçü yay boyunca açıyla doğrusaldır. Kimliği boş, ölçü alanı sayı olmayan nesne atlanır ve tabloda nedeniyle söylenir. Rotalar yeni katmana çoklu çizgi olarak yazılır.',
  ].join('\n\n'),
  keywords: ['rota', 'route', 'doğrusal referans', 'linear referencing', 'ölçü', 'M değeri', 'kilometre', 'create routes'],
  aliases: ['ROTAOLUSTUR', 'CREATEROUTES'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Girdi çizgileri', type: 'features', kinds: ['line', 'polyline', 'arc'], scopes: LRS_SCOPES, description: 'Rotaların parçaları: çizgiler, çoklu çizgiler ve yaylar.' },
    { name: 'idField', label: 'Rota kimliği alanı', type: 'field', of: 'input', description: 'Aynı değerli parçalar bir rotadır.' },
    {
      name: 'method',
      label: 'Ölçü kaynağı',
      type: 'enum',
      options: [
        { value: 'length', label: 'Uzunluk', hint: 'Çizginin uzunluğu' },
        { value: 'oneField', label: 'Tek alan', hint: 'Her parçanın uzunluk alanı' },
        { value: 'twoFields', label: 'İki alan', hint: 'Her parçanın başlangıç ve bitiş ölçüsü' },
      ],
      default: 'length',
    },
    { name: 'lengthField', label: 'Uzunluk alanı', type: 'field', of: 'input', visibleWhen: (v: { method?: unknown }) => v.method === 'oneField', description: 'Parçanın rotadaki uzunluğu.' },
    { name: 'fromField', label: 'Başlangıç ölçüsü alanı', type: 'field', of: 'input', visibleWhen: (v: { method?: unknown }) => v.method === 'twoFields' },
    { name: 'toField', label: 'Bitiş ölçüsü alanı', type: 'field', of: 'input', visibleWhen: (v: { method?: unknown }) => v.method === 'twoFields' },
    {
      name: 'priority',
      label: 'Başlangıç köşesi',
      type: 'enum',
      options: [
        { value: 'upperLeft', label: 'Sol üst' },
        { value: 'lowerLeft', label: 'Sol alt' },
        { value: 'upperRight', label: 'Sağ üst' },
        { value: 'lowerRight', label: 'Sağ alt' },
      ],
      default: 'upperLeft',
      visibleWhen: (v: { method?: unknown }) => v.method !== 'twoFields',
      description: 'Ölçü, rotanın kutusunun bu köşesine en yakın uçtan başlar.',
    },
    { name: 'factor', label: 'Ölçü çarpanı', type: 'number', default: 1, visibleWhen: (v: { method?: unknown }) => v.method !== 'twoFields', description: 'Uzunluk bununla çarpılır: 0,001 ile metreden kilometreye.' },
    { name: 'start', label: 'Başlangıç ölçüsü', type: 'number', default: 0, visibleWhen: (v: { method?: unknown }) => v.method !== 'twoFields' },
    { name: 'ignoreGaps', label: 'Boşlukları yok say', type: 'boolean', default: true, visibleWhen: (v: { method?: unknown }) => v.method !== 'twoFields', description: 'Kapalıyken parçalar arası boşluk ölçüye katılır.' },
    outputLayer('Rotalar', '#E5484D'),
  ] as const,
  outputs: [
    { name: 'count', label: 'Yazılan rota sayısı', type: 'number' },
    { name: 'table', label: 'Atlanan nesneler', type: 'table' },
  ],
  run: (v, _ctx, feedback) => {
    const objects = v.input.entities;
    const idField = v.idField ?? '';
    const pieces: Piece[] = [];
    const owner: Entity[] = [];
    const cell = (e: Entity, f: string | null | undefined) => (f ? (e.attrs[f] ?? '') : '');
    for (const e of objects) {
      const id = cell(e, idField).trim();
      const value = v.method === 'oneField' ? readMeasure(cell(e, v.lengthField)) : null;
      const from = v.method === 'twoFields' ? readMeasure(cell(e, v.fromField)) : null;
      const to = v.method === 'twoFields' ? readMeasure(cell(e, v.toField)) : null;
      for (const p of piecePaths(e)) {
        pieces.push({ id, ...p, value, from, to });
        owner.push(e);
      }
    }
    const made = lrsCreate(pieces, {
      method: v.method,
      priority: v.priority,
      factor: v.factor ?? 1,
      start: v.start ?? 0,
      ignoreGaps: v.ignoreGaps,
    });
    const add = made.routes.map((r) => polylineOf(r.parts, v.layer.id, { [idField]: r.id }));
    const seen = new Set<number>();
    const report: string[][] = [];
    for (const f of made.faults) {
      const e = owner[f.piece];
      if (seen.has(e.id)) continue;
      seen.add(e.id);
      let why = `“${idField}” boş`;
      if (f.why === 'noValue') why = cellReason(v.lengthField ?? '', cell(e, v.lengthField));
      else if (f.why === 'noFromTo') {
        const bad = readMeasure(cell(e, v.fromField)) === null ? (v.fromField ?? '') : (v.toField ?? '');
        why = cellReason(bad, cell(e, bad));
      } else if (f.why === 'noPath') why = 'İki köşesi yok';
      report.push([`#${e.id}`, why]);
    }
    if (report.length) feedback.warn(`${report.length} nesne atlandı; nedenleri tabloda.`);
    return {
      changes: { add },
      outputs: report.length ? { count: add.length, table: { columns: ['Nesne', 'Neden'], rows: report } } : { count: add.length },
      summary: `${objects.length} nesneden ${add.length} rota yazıldı.`,
    };
  },
});

/** A polyline's paths with their elevations carried to a calibrated route's vertices (a new one linear along its edge). */
function carriedElevations(e: Entity, parts: readonly RoutePart[]): (number | null)[][] | null {
  const before = elevatedPaths(e);
  if (!hasElevation(before)) return null;
  return parts.map((q) => carryElevations(q.pts, false, null, before, false));
}

export const calibrateRoutes = defineTool({
  id: 'linearRef.calibrateRoutes',
  label: 'Rotayı kalibre et',
  category: 'linearRef',
  icon: 'routeCalibrate',
  description: 'Rotaların ölçülerini (M) üzerlerindeki ölçüsü bilinen noktalardan yeniden verir.',
  help: [
    'Örnekler: kilometre taşlarının ölçüleriyle karayolu rotası, vana ve muayene bacalarının bilinen kilometreleriyle boru hattı.',
    'Her rotanın noktaları, nokta rota alanı rotanın kimliğine eşit olanlardır; arama yarıçapındaki nokta rotadaki en yakın yerine konur. En az iki nokta gerekir. Çoklu çizgide noktaların yerine köşe eklenir; çizgi iki köşesini korur, uçlarının ölçüsü verilir. Her köşenin ölçüsü çevresindeki iki noktanın arasında rota uzunluğuyla doğrusaldır; ilk noktadan önce ve sonuncudan sonra istenirse uç aralığın oranıyla uzatılır, istenmezse eski ölçüsü kalır.',
    'Rotalar yerinde, tek adımda değişir; yeni köşenin kotu kenarı boyunca doğrusaldır. Tabloda her rotanın kullanılan nokta sayısı ve durumu söylenir.',
  ].join('\n\n'),
  keywords: ['kalibrasyon', 'calibrate routes', 'rota', 'ölçü', 'M değeri', 'kilometre taşı'],
  aliases: ['ROTAKALIBRE', 'CALIBRATEROUTES'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'routes', label: 'Rotalar', type: 'features', kinds: ROUTE_KINDS, scopes: LRS_SCOPES, writes: true, description: 'Ölçüleri yeniden verilecek çizgi ve çoklu çizgiler.' },
    { name: 'routeField', label: 'Rota kimliği alanı', type: 'field', of: 'routes' },
    { name: 'points', label: 'Kalibrasyon noktaları', type: 'features', kinds: ['point'], scopes: LRS_SCOPES, description: 'Ölçüsü bilinen noktalar.' },
    { name: 'pointRoute', label: 'Noktanın rota alanı', type: 'field', of: 'points' },
    { name: 'measure', label: 'Ölçü alanı', type: 'field', of: 'points' },
    { name: 'radius', label: 'Arama yarıçapı', type: 'number', unit: 'm', min: 0, default: 1, description: 'Rotaya bundan uzak nokta alınmaz.' },
    { name: 'before', label: 'İlk noktadan öncesini uzat', type: 'boolean', default: true },
    { name: 'after', label: 'Son noktadan sonrasını uzat', type: 'boolean', default: true },
    { name: 'ignoreGaps', label: 'Boşlukları yok say', type: 'boolean', default: true, description: 'Kapalıyken rotanın parçaları arası boşluk uzunluğa katılır.' },
  ] as const,
  outputs: [
    { name: 'count', label: 'Kalibre edilen rota sayısı', type: 'number' },
    { name: 'table', label: 'Rotalar', type: 'table' },
  ],
  run: (v, _ctx, feedback) => {
    const routes = routesById(v.routes.entities, v.routeField ?? '');
    if (routes.again) feedback.warn(againText(routes.again));
    const pointRoute = v.pointRoute ?? '';
    const measure = v.measure ?? '';
    let bad = 0;
    const marks = routes.ids.map(() => [] as { p: { x: number; y: number }; m: number }[]);
    for (const s of v.points.entities) {
      if (s.kind !== 'point') continue;
      const id = (s.attrs[pointRoute] ?? '').trim();
      if (!id) continue;
      const m = readMeasure(s.attrs[measure] ?? '');
      if (m === null) {
        bad++;
        continue;
      }
      const k = routes.index.get(id);
      if (k !== undefined) marks[k].push({ p: s.p, m });
    }
    if (bad) feedback.warn(`${bad} noktanın ölçüsü sayı değil; kullanılmadı.`);
    const rules = { radius: v.radius, before: v.before, after: v.after, ignoreGaps: v.ignoreGaps };
    const update: { id: number; patch: Partial<Entity> }[] = [];
    const rows: string[][] = [];
    let used = 0;
    routes.ids.forEach((id, k) => {
      const e = routes.objects[k];
      // A line keeps its two vertices; a polyline takes one where each point falls.
      const [out] = lrsCalibrate([routes.routes[k]], [marks[k]], { ...rules, split: e.kind !== 'line' });
      const n = out.used.filter(Boolean).length;
      if (out.parts == null) {
        rows.push([id, String(n), 'İkiden az nokta']);
        return;
      }
      used += n;
      rows.push([id, String(n), 'Kalibre edildi']);
      if (e.kind === 'line') {
        const [ma, mb] = out.parts[0].ms;
        update.push({ id: e.id, patch: { ma: ma ?? undefined, mb: mb ?? undefined } as Partial<Entity> });
        return;
      }
      const shaped = polylineOf(out.parts, e.layerId, e.attrs) as Partial<Entity> & { pts: unknown; parts?: unknown };
      const zs = carriedElevations(e, out.parts);
      const [first, ...rest] = zs ?? [];
      update.push({
        id: e.id,
        patch: {
          pts: shaped.pts,
          bulges: (shaped as { bulges?: number[] }).bulges,
          ms: (shaped as { ms?: (number | null)[] }).ms,
          zs: first?.some((z) => z !== null) ? first : undefined,
          parts: (shaped.parts as RoutePart[] | undefined)?.map((q, i) => ({ ...q, ...(rest[i]?.some((z) => z !== null) && { zs: rest[i] }) })),
        } as Partial<Entity>,
      });
    });
    const done = rows.filter((r) => r[2] === 'Kalibre edildi').length;
    return {
      changes: { update },
      outputs: { count: done, table: { columns: ['Rota', 'Kullanılan nokta', 'Durum'], rows } },
      summary: `${routes.ids.length} rotadan ${done} rota kalibre edildi; ${used} nokta kullanıldı.`,
    };
  },
});

export const extractMeasures = defineTool({
  id: 'linearRef.extractMeasures',
  label: 'M değerlerini çıkar',
  category: 'linearRef',
  icon: 'routeMeasures',
  description: 'Rotaların ilk, son, en küçük ve en büyük ölçüsünü (M) özniteliklerine yazar.',
  help: [
    'Örnekler: her yol rotasının başlangıç ve bitiş kilometresi, ölçü aralıklarının bir tabloda listelenmesi, ölçüsüz rotaların bulunması.',
    'Yazılan alanlar önekle adlanır: “<önek>bas” ilk ölçülü köşenin, “<önek>son” sonuncunun ölçüsü, “<önek>min” en küçüğü, “<önek>max” en büyüğü; sayılar projenin uzunluk basamağıyla. Ölçüsü olmayan nesneye yazılmaz.',
  ].join('\n\n'),
  keywords: ['M değeri', 'ölçü', 'rota', 'extract m values', 'kilometre'],
  aliases: ['MCIKAR', 'EXTRACTM'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Rotalar', type: 'features', kinds: ROUTE_KINDS, scopes: LRS_SCOPES, writes: true },
    { name: 'prefix', label: 'Önek', type: 'string', default: 'M_', allowEmpty: true, maxLength: 24, description: 'Yazılan alanların adlarının başı: “M_bas”, “M_son”.' },
  ] as const,
  outputs: [{ name: 'count', label: 'Yazılan rota sayısı', type: 'number' }],
  preview: (v) => `“${v.prefix}bas”, “${v.prefix}son”, “${v.prefix}min”, “${v.prefix}max”`,
  run: (v, ctx, feedback) => write(v.input.entities, v.prefix, ctx, feedback),
});

function write(objects: readonly Entity[], prefix: string, ctx: RunContext, feedback: Feedback): RunResult {
  const routes = objects.map(routeOf);
  const sums = lrsSummaries(routes.filter((r): r is Route => r !== null));
  const update: { id: number; patch: Partial<Entity> }[] = [];
  let k = 0;
  let without = 0;
  objects.forEach((e, i) => {
    if (!routes[i]) return;
    const s = sums[k++];
    if (!s.count || s.first == null || s.last == null || s.least == null || s.most == null) {
      without++;
      return;
    }
    let current: Entity = e;
    for (const [key, value] of [
      ['bas', s.first],
      ['son', s.last],
      ['min', s.least],
      ['max', s.most],
    ] as const) {
      const attrs = withAttr(ctx, current, `${prefix}${key}`, measureText(ctx.units, value));
      if (attrs) current = { ...current, attrs };
    }
    if (current !== e) update.push({ id: e.id, patch: { attrs: current.attrs } });
  });
  if (without) feedback.info(`${without} nesnenin ölçüsü yok; yazılmadı.`);
  const written = routes.filter(Boolean).length - without;
  return {
    changes: { update },
    outputs: { count: written },
    summary: `${routes.filter(Boolean).length} rotadan ${written} rotanın ölçüleri yazıldı.`,
  };
}
