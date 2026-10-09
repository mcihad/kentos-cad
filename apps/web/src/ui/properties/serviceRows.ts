import type { AppContext } from '../../app/context';
import type { ServiceConnection } from '../../contracts/generated/ServiceConnection';
import { crsBySrid } from '../../geo/crs';
import type { LayerNode } from '../../model/layers';
import { AUTH_KIND_LABELS, FEED_KIND_LABELS, SERVICE_KIND_LABELS } from '../../model/serviceRules';
import type { PropRow, PropSection } from '../widgets/PropertyGrid';

/**
 * Öznitelikler's section for a layer drawn from a map service (docs/adr/0208 §14; the desktop's
 * `properties::rows::service_section`): its kind, address, layers, system, levels, connection, opacity, credits and
 * how it is going; for a layer whose objects came from a service: where from and when.
 */

const systemText = (srid: number) => {
  const c = crsBySrid(srid);
  return c ? `${c.name} (EPSG:${srid})` : `EPSG:${srid}`;
};

function connectionText(ctx: AppContext, id: string | undefined): string | null {
  if (!id) return null;
  const c: ServiceConnection | undefined = ctx.doc.settings.connections.value.find((x) => x.id === id);
  if (!c) return id;
  const here = ctx.secrets.get(c.origin, c.id) !== null;
  return `${c.name} (${AUTH_KIND_LABELS[c.auth]})${c.auth === 'none' || here ? '' : ', değerleri bu cihazda yok'}`;
}

/** The section for `node`, `failure` why its service shows nothing; none for a layer of neither kind. */
export function serviceSection(ctx: AppContext, node: LayerNode, failure: string | null): PropSection | null {
  const s = node.service;
  if (s) {
    const rows: PropRow[] = [
      { label: 'Tür', value: SERVICE_KIND_LABELS[s.kind] },
      { label: 'Adres', value: s.url || 'Google Map Tiles API' },
    ];
    if (s.layers?.length) rows.push({ label: 'Katmanlar', value: s.layers.join(', ') });
    if (s.style !== undefined) rows.push({ label: s.kind === 'google' ? 'Harita türü' : 'Stil', value: s.style });
    const srid = s.srid ?? s.grid?.srid;
    if (srid !== undefined) rows.push({ label: 'Sistem', value: systemText(srid) });
    else if (s.kind === 'xyz' || s.kind === 'google' || s.kind === 'vector') rows.push({ label: 'Sistem', value: systemText(3857) });
    rows.push({ label: 'Katlar', value: `${s.minZoom ?? 0}–${s.maxZoom ?? '—'}`, numeric: true });
    const conn = connectionText(ctx, s.connection);
    if (conn) rows.push({ label: 'Bağlantı', value: conn });
    rows.push({ label: 'Saydamlık', value: `% ${Math.round((s.opacity ?? 1) * 100)}`, numeric: true });
    if (s.attribution) rows.push({ label: 'Atıf', value: s.attribution });
    rows.push({ label: 'Durum', value: failure ?? 'Çiziliyor' });
    return { id: 'service', title: 'Servis katmanı', rows };
  }
  const f = node.feed;
  if (!f) return null;
  const rows: PropRow[] = [
    { label: 'Tür', value: FEED_KIND_LABELS[f.kind] },
    { label: 'Adres', value: f.url },
  ];
  if (f.name !== undefined) rows.push({ label: 'Tür adı', value: f.name });
  if (f.srid !== undefined) rows.push({ label: 'İstenen sistem', value: systemText(f.srid) });
  if (f.filter !== undefined) rows.push({ label: 'Süzgeç', value: f.filter });
  rows.push({ label: 'Alan', value: f.bbox ? 'İstenen alan' : 'Hepsi' });
  if (f.key !== undefined) rows.push({ label: 'Anahtar alan', value: f.key });
  const conn = connectionText(ctx, f.connection);
  if (conn) rows.push({ label: 'Bağlantı', value: conn });
  if (f.fetched) rows.push({ label: 'Son alınış', value: `${f.fetched.replace('T', ' ').replace(/Z$/, '')} UTC` });
  return { id: 'feed', title: 'Veri kaynağı', rows };
}
