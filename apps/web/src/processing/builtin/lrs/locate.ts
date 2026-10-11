import { entityGeometry, type Entity, type EntityKind } from '../../../model/entities';
import { lrsLocate } from '../../../model/ops/lrs';
import { defineTool } from '../../types';
import { withAttr } from '../attributeWrites';
import { LRS_SCOPES, ROUTE_KINDS, againText, measureText, routesById } from './shared';

/**
 * Rota boyunca konumla (docs/adr/0224 §5; ArcGIS Locate Features Along Routes): each object's places along the routes
 * within a radius, a point by its closest place, a line when every vertex is within the radius, an area by the routes'
 * pieces inside it, as a table and, when asked, the first place written into the object's attributes. The search is
 * the core's (`ops::lrs::locate_run`, its routes' edges in a packed R-tree); the desktop's twin is
 * `builtin/lrs/locate.rs`.
 */

/** What can be located: points, lines and areas. */
const LOCATE_KINDS: readonly EntityKind[] = ['point', 'line', 'polyline', 'arc', 'polygon'];

export const locateAlong = defineTool({
  id: 'linearRef.locateAlong',
  label: 'Rota boyunca konumla',
  category: 'linearRef',
  icon: 'routeLocate',
  description: 'Nesnelerin rotalardaki yerini ölçüleriyle bulur: noktanın ölçüsü ve uzaklığı, çizginin ve alanın başlangıç ve bitiş ölçüsü.',
  help: [
    'Örnekler: kazaların hangi kilometrede olduğu, bir kaplama çizgisinin yolun hangi kesimine düştüğü, bir parselin yolun hangi kesimini kapladığı.',
    'Nokta rotadaki en yakın yerine konumlanır; arama yarıçapı içinde değilse konumlanmaz. Uzaklık ölçü yönünün sağına artıdır (ya da seçilirse soluna). Yarıçaptaki bütün rotalar istenirse her biri bir satırdır, istenmezse yalnız en yakını. Çizgi bütün köşeleri yarıçap içindeyse ilk ve son köşesinin ölçüleriyle, alan içinden geçen rotaların parçalarıyla konumlanır.',
    'Sonuç tablodadır. Özniteliklere yaz açıkken ilk satır nesnenin Rota, Ölçü ya da Başlangıç ve Bitiş, Uzaklık alanlarına yazılır.',
  ].join('\n\n'),
  keywords: ['konumla', 'locate features along routes', 'rota', 'ölçü', 'kilometre', 'doğrusal referans', 'olay tablosu'],
  aliases: ['ROTAKONUM', 'LOCATEALONG'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Konumlanacak nesneler', type: 'features', kinds: LOCATE_KINDS, scopes: LRS_SCOPES, writes: true },
    { name: 'routes', label: 'Rotalar', type: 'features', kinds: ROUTE_KINDS, scopes: LRS_SCOPES },
    { name: 'routeField', label: 'Rota kimliği alanı', type: 'field', of: 'routes' },
    { name: 'radius', label: 'Arama yarıçapı', type: 'number', unit: 'm', min: 0, default: 10 },
    { name: 'all', label: 'Yarıçaptaki bütün rotalar', type: 'boolean', default: false, description: 'Kapalıyken yalnız en yakın rota.' },
    {
      name: 'side',
      label: 'Artı uzaklık',
      type: 'enum',
      options: [
        { value: 'right', label: 'Sağa', hint: 'Ölçü yönünün sağı' },
        { value: 'left', label: 'Sola', hint: 'Ölçü yönünün solu' },
      ],
      default: 'right',
    },
    { name: 'write', label: 'Özniteliklere yaz', type: 'boolean', default: false, description: 'İlk satırı Rota, Ölçü (Başlangıç, Bitiş) ve Uzaklık alanlarına yazar.' },
  ] as const,
  outputs: [
    { name: 'count', label: 'Konumlanan nesne sayısı', type: 'number' },
    { name: 'table', label: 'Konumlar', type: 'table' },
  ],
  run: (v, ctx, feedback) => {
    const routes = routesById(v.routes.entities, v.routeField ?? '');
    if (routes.again) feedback.warn(againText(routes.again));
    const objects = v.input.entities;
    feedback.progress(0, 'Rotalar aranıyor');
    const found = lrsLocate(
      routes.routes,
      objects.map(entityGeometry),
      v.radius,
      v.all,
    );
    const turn = v.side === 'left' ? -1 : 1;
    // A field the core leaves out is none (`Located`'s nones are not written).
    const text = (n: number | null | undefined) => (n == null ? '' : measureText(ctx.units, n));
    const rows: string[][] = [];
    const update: { id: number; patch: Partial<Entity> }[] = [];
    let located = 0;
    objects.forEach((e, i) => {
      const places = found[i];
      if (!places.length) return;
      located++;
      for (const q of places) {
        // A point's distance is signed (+ right of the measure direction); a line's farthest vertex's is not.
        const d = q.distance == null ? null : e.kind === 'point' ? q.distance * turn : q.distance;
        rows.push([`#${e.id}`, routes.ids[q.route], text(q.m), text(q.from), text(q.to), text(d)]);
      }
      if (!v.write) return;
      const q = places[0];
      let current: Entity = e;
      const put = (name: string, value: string | null) => {
        const attrs = withAttr(ctx, current, name, value);
        if (attrs) current = { ...current, attrs };
      };
      put('Rota', routes.ids[q.route]);
      if (q.m != null) put('Ölçü', text(q.m));
      if (q.from != null && q.to != null) {
        put('Başlangıç', text(q.from));
        put('Bitiş', text(q.to));
      }
      if (q.distance != null) put('Uzaklık', text(e.kind === 'point' ? q.distance * turn : q.distance));
      if (current !== e) update.push({ id: e.id, patch: { attrs: current.attrs } });
    });
    return {
      changes: { update },
      outputs: { count: located, table: { columns: ['Nesne', 'Rota', 'Ölçü', 'Başlangıç', 'Bitiş', 'Uzaklık'], rows } },
      summary: `${objects.length} nesneden ${located} nesne konumlandı; ${rows.length} satır.`,
    };
  },
});
