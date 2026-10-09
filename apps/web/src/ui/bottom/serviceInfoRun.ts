import type { AppContext } from '../../app/context';
import { Signal } from '../../core/signal';
import { serviceText, type Wire } from '../../io/services/fetch';
import { loadServices } from '../../io/services/module';
import type { Vec2 } from '../../model/geometry';
import { crsSettings } from '../../model/projectCrs';
import { servicePair, toService } from '../../model/serviceSystems';
import { PickPointTool } from '../../tools/pickPointTool';

/**
 * Servis bilgisi (docs/adr/0208 §11; the desktop's `services/info.rs`): a point picked on the drawing, then every WMS
 * layer shown is asked with GetFeatureInfo and every ArcGIS layer with `identify`, each with its connection's proof.
 * A small map of 101 pixels around the point is asked for, in the service's own system, so the answer does not depend
 * on the view's size. The answers are listed in the bottom panel's Servis bilgisi tab (ServiceInfoPanel.ts),
 * layer by layer, field and value.
 */

/** A record of one layer of a service's answer: its fields and values, as the service gave them. */
export interface InfoRow {
  layer: string;
  fields: [string, string][];
}

export interface InfoAnswer {
  /** The drawing layer's name. */
  layer: string;
  rows: InfoRow[] | { error: string };
}

/** What the tab shows: the point asked about, whether the services are being asked, their answers. */
export const serviceInfo = {
  at: new Signal<Vec2 | null>(null),
  busy: new Signal(false),
  answers: new Signal<readonly InfoAnswer[]>([]),
};

/** The shown WMS and ArcGIS layers, top first. */
function askable(ctx: AppContext) {
  return ctx.doc.layers.leaves().filter((l) => l.service && (l.service.kind === 'wms' || l.service.kind === 'arcgis') && ctx.doc.layers.isVisible(l.id));
}

/** Servis bilgisi: a point is asked for on the drawing. */
export function startServiceInfo(ctx: AppContext): void {
  if (!askable(ctx).length) {
    ctx.log.info('Sorulabilecek servis yok: görünen bir WMS ya da ArcGIS katmanı ekleyin.');
    return;
  }
  ctx.tools.run(
    new PickPointTool(ctx, 'Sorulacak nokta', (p) => {
      if (p) void ask(ctx, p);
    }),
    'Servis bilgisi',
  );
}

/** The services shown asked about `p`, one after another. */
async function ask(ctx: AppContext, p: Vec2): Promise<void> {
  const settings = crsSettings(ctx.doc.settings);
  // A screen pixel in the project's units.
  const px = 1 / ctx.view.camera.scale;
  const proxy = ctx.cloud.auth.value === 'signedIn';
  const jobs = askable(ctx).flatMap((n) => {
    const s = n.service!;
    const srid = s.srid ?? s.grid?.srid ?? 3857;
    const pair = servicePair(settings, srid);
    if ('error' in pair) return [];
    const q = toService(pair, p);
    if (!q) return [];
    // A pixel in the service's units, from two points a pixel apart.
    const q2 = toService(pair, { x: p.x + px, y: p.y });
    const d = q2 ? Math.hypot(q2.x - q.x, q2.y - q.y) : NaN;
    const units = Number.isFinite(d) && d > 0 ? d : px;
    const conn = s.connection ? (ctx.doc.settings.connections.value.find((c) => c.id === s.connection) ?? null) : null;
    const proof = conn ? { connection: conn, secret: ctx.secrets.get(conn.origin, conn.id) } : null;
    return [{ name: n.name, service: s, srid, q, units, proof }];
  });
  const at = { x: p.x, y: p.y };
  serviceInfo.at.set(at);
  serviceInfo.answers.set([]);
  serviceInfo.busy.set(true);
  ctx.ui.bottomTab.set('serviceInfo');
  ctx.ui.bottomExpanded.set(true);
  const answers: InfoAnswer[] = [];
  try {
    const m = await loadServices();
    for (const j of jobs) {
      let last: InfoAnswer['rows'] = { error: 'Servis bu noktada bilgi vermedi.' };
      const asked = JSON.parse(m.infoRequests(JSON.stringify(j.service), j.srid, j.q.x, j.q.y, j.units)) as { media: string; request: Wire }[];
      for (const a of asked) {
        try {
          const body = await serviceText(m, a.request, j.proof, { proxy, referer: j.service.url });
          last = JSON.parse(m.infoRead(a.media, body)) as InfoRow[];
          break;
        } catch (e) {
          last = { error: e instanceof Error ? e.message : String(e) };
        }
      }
      answers.push({ layer: j.name, rows: last });
    }
  } catch (e) {
    answers.push({ layer: 'Servis bilgisi', rows: { error: e instanceof Error ? e.message : String(e) } });
  }
  // A later point asked meanwhile has the tab.
  if (serviceInfo.at.value !== at) return;
  serviceInfo.answers.set(answers);
  serviceInfo.busy.set(false);
}
