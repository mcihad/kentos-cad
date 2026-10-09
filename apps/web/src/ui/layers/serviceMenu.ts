import type { AppContext } from '../../app/context';
import type { LayerNode } from '../../model/layers';
import { crsSettings } from '../../model/projectCrs';
import { fromService, servicePair } from '../../model/serviceSystems';
import { layersService } from '../../product/layersService';
import { serviceHub, serviceKey } from '../../render/serviceHub';
import { connectionOf } from '../../viewport/serviceLayers';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * A map service layer's items in the layer tree (docs/adr/0208 §14; the desktop's `services/app.rs` `service_menu`):
 * Servis ayarları, Yeniden yükle, Önbelleği temizle, Servisin kapsamına yakınlaştır and Saydamlık ▸; a layer whose
 * objects came from a service has Yenile. Why a service shows nothing is the row's badge.
 */

const OPACITIES = [1, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4];

/** The layer's service's key in the hub. */
const keyOf = (ctx: AppContext, n: LayerNode) => serviceKey(n.service!, connectionOf(ctx.doc, n));

/** Why the layer's service shows nothing, when it failed. */
export function serviceFailure(ctx: AppContext, n: LayerNode): string | null {
  return n.service ? serviceHub().failure(keyOf(ctx, n)) : null;
}

/** Servisin kapsamına yakınlaştır: the service's extent (WGS 84 degrees) brought into the project's system. */
function zoomToService(ctx: AppContext, n: LayerNode): void {
  const b = n.service?.bbox;
  if (!b) return void ctx.log.warn(`“${n.name}” servisinin kapsamı bilinmiyor: servisi Harita servisi penceresinden yeniden ekleyin.`);
  const pair = servicePair(crsSettings(ctx.doc.settings), 4326);
  if ('error' in pair) return void ctx.log.warn(`“${n.name}”: ${pair.error}`);
  const [w, s, e, nn] = b;
  const box = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
  for (let i = 0; i <= 4; i++) {
    const t = i / 4;
    for (const [x, y] of [
      [w + (e - w) * t, s],
      [w + (e - w) * t, nn],
      [w, s + (nn - s) * t],
      [e, s + (nn - s) * t],
    ]) {
      const p = fromService(pair, { x, y });
      if (!p) continue;
      box.minX = Math.min(box.minX, p.x);
      box.minY = Math.min(box.minY, p.y);
      box.maxX = Math.max(box.maxX, p.x);
      box.maxY = Math.max(box.maxY, p.y);
    }
  }
  if (!(Number.isFinite(box.minX) && box.maxX > box.minX && box.maxY > box.minY))
    return void ctx.log.warn(`“${n.name}” servisinin kapsamı projenin sisteminde gösterilemiyor.`);
  ctx.view.zoomToBox(box, 24);
}

/** A service layer's opacity, written by `cad.layers.service` as one undo step. */
function setOpacity(ctx: AppContext, n: LayerNode, opacity: number): void {
  const service = structuredClone(n.service!);
  if (opacity < 1) service.opacity = opacity;
  else delete service.opacity;
  const r = layersService.execute({ doc: ctx.doc }, { operation: 'update', layer: n.id, service });
  if (r.status !== 'completed' && 'error' in r) ctx.log.warn(r.error.message);
}

/** The service's own items, ahead of Yeniden adlandır and Sil. */
export function serviceItems(ctx: AppContext, n: LayerNode): MenuItem[] {
  const now = n.service?.opacity ?? 1;
  return [
    { label: 'Servis ayarları…', icon: 'serviceSettings', run: () => void import('../services/ServiceDialog').then((m) => m.openServiceDialog(ctx, n.id)) },
    {
      label: 'Yeniden yükle',
      icon: 'serviceReload',
      run: () => {
        serviceHub().reload(keyOf(ctx, n));
        ctx.log.info(`“${n.name}” servisi yeniden okunuyor.`);
      },
    },
    {
      label: 'Önbelleği temizle',
      icon: 'serviceCache',
      run: () => {
        serviceHub().reload(keyOf(ctx, n), true);
        ctx.log.success(`“${n.name}” servisinin önbelleği temizlendi: karoları tarayıcının önbelleğini atlayarak yeniden alınıyor.`);
      },
    },
    { label: 'Servisin kapsamına yakınlaştır', icon: 'serviceExtent', disabled: !n.service?.bbox, run: () => zoomToService(ctx, n) },
    {
      label: 'Saydamlık',
      icon: 'basemap',
      items: () => OPACITIES.map((o): MenuItem => ({ label: `% ${Math.round(o * 100)}`, radio: true, checked: Math.abs(now - o) < 1e-9, run: () => setOpacity(ctx, n, o) })),
    },
  ];
}

/** Yenile on a layer whose objects came from a service. */
export function feedItem(ctx: AppContext, n: LayerNode): MenuItem {
  return { label: 'Yenile', icon: 'feedRefresh', run: () => void import('../services/FeedDialog').then((m) => m.refreshFeed(ctx, n.id)) };
}
