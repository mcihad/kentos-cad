/**
 * Map services' icons (docs/adr/0208 §14; chosen without asking, the owner's word of 8 October): the 20×20 stroke set's
 * way. The commands: Altlık a folded map; Harita servisi a globe and a plus; Servisten veri al a globe over an arrow
 * into a layer; Servis bilgisi a pointer over a globe's "i"; Bağlantılar two links of a chain; Altlığı kaldır the map
 * crossed out; a layer's service menu: its settings, reloading, the cache let go, the service's extent. The ready
 * basemaps each their own picture in a tile's frame: OpenStreetMap's streets, contours, a cross, a bicycle's wheels,
 * vector nodes (plain, bright and light), a satellite over a photograph, a survey pillar and a photograph of HGM, a
 * pin over a road, a photograph, hills and both for Google, and MapTiler's vector and photo tiles with a key.
 */

/** A basemap's tile: the frame its picture is drawn in. */
const tile = '<rect x="2.5" y="2.5" width="15" height="15" rx="1.4" stroke-width="1.2"/>';
/** A small key in the lower right corner: the basemap asks for one. */
const key = '<circle cx="14.3" cy="14.3" r="1.7" stroke-width="1.1"/><path d="M15.5 15.5 18 18M17 17l1-1" stroke-width="1.1"/>';
/** A pin in the upper left corner: Google's basemaps. */
const pin = '<path d="M6.5 11.2C4.8 9.3 4 8 4 6.9a2.5 2.5 0 0 1 5 0c0 1.1-.8 2.4-2.5 4.3z" fill="currentColor" fill-opacity=".25" stroke-width="1.1"/>';
const dot = (x: number, y: number, r = 0.9) => `<circle cx="${x}" cy="${y}" r="${r}" fill="currentColor" stroke="none"/>`;

export const SERVICE_ICONS = {
  basemap: '<path d="M2.5 5.2 7 3.2l6 2.2 4.5-2v11.4l-4.5 2-6-2.2-4.5 2z"/><path d="M7 3.2v11.4M13 5.4v11.4" stroke-width="1.1"/>',
  basemapRemove:
    '<path d="M2.5 5.2 7 3.2l6 2.2 4.5-2v6.6M10.5 15.9 7 14.6l-4.5 2V5.2" stroke-width="1.3"/><path d="M7 3.2v11.4M13 5.4v4" stroke-width="1.1"/><path d="m12.5 12.5 5 5M17.5 12.5l-5 5" stroke-width="1.5"/>',
  serviceAdd:
    '<circle cx="8.5" cy="8.5" r="6"/><path d="M2.5 8.5h12M8.5 2.5c-1.8 1.8-2.4 3.8-2.4 6s.6 4.2 2.4 6M8.5 2.5c1.8 1.8 2.4 3.8 2.4 6s-.6 4.2-2.4 6" stroke-width="1.1"/><path d="M15.5 12.5v6M12.5 15.5h6" stroke-width="1.6"/>',
  serviceFeed:
    '<circle cx="6.5" cy="6" r="4"/><path d="M2.5 6h8M6.5 2c-1.2 1.2-1.6 2.5-1.6 4s.4 2.8 1.6 4M6.5 2c1.2 1.2 1.6 2.5 1.6 4s-.4 2.8-1.6 4" stroke-width="1"/><path d="M14.5 2.5v7M12 7l2.5 2.5L17 7" stroke-width="1.3"/><path d="m10 12 7.5 3-7.5 3-7.5-3z" fill="currentColor" fill-opacity=".25"/>',
  serviceInfo:
    '<circle cx="12.5" cy="12" r="5.5" stroke-width="1.2"/><path d="M12.5 11v4M12.5 8.5v.2" stroke-width="1.5"/><path d="M2.5 2.5l6 2.6-2.7.9-.9 2.7z" fill="currentColor" stroke-width="1.1"/>',
  serviceConnections:
    '<path d="M8.3 11.7l3.4-3.4" stroke-width="1.5"/><path d="M9.8 6.2l1.6-1.6a3.1 3.1 0 0 1 4.4 4.4l-1.6 1.6"/><path d="M10.2 13.8l-1.6 1.6a3.1 3.1 0 0 1-4.4-4.4l1.6-1.6"/>',
  serviceSettings:
    '<circle cx="7.5" cy="7.5" r="4.5"/><path d="M3 7.5h9M7.5 3c-1.4 1.4-1.8 2.9-1.8 4.5s.4 3.1 1.8 4.5M7.5 3c1.4 1.4 1.8 2.9 1.8 4.5s-.4 3.1-1.8 4.5" stroke-width="1"/><path d="M11.5 15.5h6M11.5 12.5h6M11.5 18.5h6" stroke-width="1.2"/><circle cx="13.5" cy="12.5" r="1.1" fill="currentColor" stroke="none"/><circle cx="16" cy="15.5" r="1.1" fill="currentColor" stroke="none"/><circle cx="14" cy="18.5" r="1.1" fill="currentColor" stroke="none"/>',
  serviceReload:
    '<circle cx="8" cy="8" r="5"/><path d="M3 8h10M8 3c-1.5 1.5-2 3.2-2 5s.5 3.5 2 5M8 3c1.5 1.5 2 3.2 2 5s-.5 3.5-2 5" stroke-width="1"/><path d="M17.5 13.5a4 4 0 1 1-1.2-2.8" stroke-width="1.3"/><path d="M16.8 8.6v2.4h-2.4" stroke-width="1.3"/>',
  serviceCache:
    '<path d="M3.5 6.5h13l-1.2 11h-10.6z" stroke-width="1.2"/><path d="M2.5 6.5h15M7.5 6.5V4.2c0-.7.5-1.2 1.2-1.2h2.6c.7 0 1.2.5 1.2 1.2v2.3" stroke-width="1.2"/><rect x="7" y="9.5" width="2.6" height="2.6" stroke-width="1"/><rect x="10.4" y="9.5" width="2.6" height="2.6" stroke-width="1"/><rect x="7" y="12.9" width="2.6" height="2.6" stroke-width="1"/><rect x="10.4" y="12.9" width="2.6" height="2.6" fill="currentColor" fill-opacity=".35" stroke-width="1"/>',
  serviceExtent:
    '<path d="M2.5 6.5v-4h4M13.5 2.5h4v4M17.5 13.5v4h-4M6.5 17.5h-4v-4" stroke-width="1.3"/><circle cx="10" cy="10" r="4.2"/><path d="M5.8 10h8.4M10 5.8c-1 1-1.4 2.6-1.4 4.2s.4 3.2 1.4 4.2M10 5.8c1 1 1.4 2.6 1.4 4.2s-.4 3.2-1.4 4.2" stroke-width="1"/>',
  feedRefresh:
    '<path d="m9 9.5 7 2.8-7 2.8-7-2.8z" fill="currentColor" fill-opacity=".25"/><path d="M2 15.5l7 2.8 7-2.8" stroke-width="1.1"/><path d="M17.8 6.5a4 4 0 1 1-1.2-2.8" stroke-width="1.3"/><path d="M17.1 1.6V4h-2.4" stroke-width="1.3"/>',
  // The ready basemaps (fixtures/services/v1/presets.json names them).
  basemapOsm: `${tile}<path d="M2.5 11.5 8 9l9.5 1.5M8.5 2.5 7.4 17.5M12 9.8 14 17.5" stroke-width="1.3"/><path d="M2.5 6.5 6 7.5" stroke-width=".9"/>`,
  basemapTopo: `${tile}<path d="M4.5 13.5c1.5-3 3.6-5 6-5.2 2.6-.2 4.2 1.6 5 4M6.8 13.5c1-1.8 2.2-2.9 3.7-3 1.6-.1 2.5.9 3 2.4" stroke-width="1.1"/><path d="M2.5 15.5c3-1 6.2-.8 9.5.2s4.4.7 5.5.3" stroke-width="1"/>${dot(10.4, 11.6, 0.9)}`,
  basemapHot: `${tile}<path d="M8.5 5.5h3v3h3v3h-3v3h-3v-3h-3v-3h3z" fill="currentColor" fill-opacity=".3" stroke-width="1.2"/>`,
  basemapCycle: `${tile}<circle cx="6.5" cy="12.5" r="2.6" stroke-width="1.1"/><circle cx="13.5" cy="12.5" r="2.6" stroke-width="1.1"/><path d="M6.5 12.5 9 7.5h3.5l1 5M9 7.5 10.4 12.5h3.1M8.2 6h2" stroke-width="1.1"/>`,
  basemapVector: `${tile}<path d="M5 14.5 8.5 6l3.5 5 3-6.5" stroke-width="1.2"/><rect x="3.6" y="13.1" width="2.8" height="2.8" fill="currentColor" stroke="none"/><rect x="7.1" y="4.6" width="2.8" height="2.8" fill="currentColor" stroke="none"/><rect x="10.6" y="9.6" width="2.8" height="2.8" fill="currentColor" stroke="none"/><rect x="13.6" y="3.1" width="2.8" height="2.8" fill="currentColor" stroke="none"/>`,
  basemapVectorBright: `${tile}<path d="M5 15 9 9.5l3.5 3 2.5-3.5" stroke-width="1.2"/><circle cx="12" cy="6" r="1.8" fill="currentColor" stroke="none"/><path d="M12 2.9v-.4M12 9.5v-.4M8.9 6h-.4M15.5 6h-.4M9.8 3.8l-.3-.3M14.5 8.5l-.3-.3M14.2 3.8l.3-.3" stroke-width="1"/><rect x="3.8" y="13.8" width="2.4" height="2.4" fill="currentColor" stroke="none"/><rect x="13.8" y="7.8" width="2.4" height="2.4" fill="currentColor" stroke="none"/>`,
  basemapVectorLight: `${tile}<path d="M5 14.5 8.5 6l3.5 5 3-6.5" stroke-width="1.1" stroke-dasharray="1.6 1.3"/><rect x="3.9" y="13.4" width="2.2" height="2.2" stroke-width="1"/><rect x="7.4" y="4.9" width="2.2" height="2.2" stroke-width="1"/><rect x="10.9" y="9.9" width="2.2" height="2.2" stroke-width="1"/><rect x="13.9" y="3.4" width="2.2" height="2.2" stroke-width="1"/>`,
  basemapImagery: `${tile}<path d="M2.5 13.5 7 9l3 3 3.2-3.2 4.3 4.7v3.5h-15z" fill="currentColor" fill-opacity=".3" stroke="none"/><path d="M11 3.6l2.2 2.2-1.4 1.4-2.2-2.2zM12.5 5.1l2-2M9.9 6.5l-2 2" stroke-width="1.1"/><path d="M14.6 4.2l1.4 1.4M16.4 3l1.4 1.4" stroke-width=".9"/>`,
  basemapHgm: `${tile}<path d="M7.5 15.5 9 7.5h2l1.5 8z" fill="currentColor" fill-opacity=".25" stroke-width="1.2"/><path d="M6 15.5h8" stroke-width="1.2"/><circle cx="10" cy="5.4" r="1.4" stroke-width="1.1"/><path d="M10 3.2v-.9" stroke-width="1"/>`,
  basemapHgmPhoto: `${tile}<path d="M2.5 15 6.5 11l2.4 2.2 3.6-3.7 5 5.5v2.5h-15z" fill="currentColor" fill-opacity=".3" stroke="none"/><path d="M11 9.5 11.8 4.5h1.4l.8 5z" stroke-width="1.1"/><circle cx="12.5" cy="3.4" r=".9" stroke-width="1"/>`,
  basemapGoogleRoad: `${tile}${pin}<path d="M11.5 17.5 13 10M17.5 13.5 11.8 12" stroke-width="1.4"/><path d="M2.5 15h6" stroke-width="1"/>`,
  basemapGoogleSatellite: `${tile}${pin}<path d="M2.5 14 6 12l3 2 3.5-4 5 4.5v3h-15z" fill="currentColor" fill-opacity=".35" stroke="none"/><path d="M13.5 4.5l2 2-1.2 1.2-2-2z" stroke-width="1"/>`,
  basemapGoogleTerrain: `${tile}${pin}<path d="M4.5 16 10 9.5l2.5 3 2-2.2 3 4.2" stroke-width="1.2"/><path d="M8.6 11.2 10 9.5l1.2 1.4" stroke-width="1"/>`,
  basemapGoogleHybrid: `${tile}${pin}<path d="M2.5 14 6 12l3 2 3.5-4 5 4.5v3h-15z" fill="currentColor" fill-opacity=".3" stroke="none"/><path d="M11.5 17.5 13.2 8.5M17.5 11.5l-8 2.6" stroke-width="1.3"/>`,
  basemapMaptiler: `${tile}<path d="M2.5 8.5h15M8.5 2.5v15" stroke-width="1"/><path d="M4.5 6.5 6.7 4.6M10.5 5.8l4.5-1.6M4.2 14 6.5 11" stroke-width="1.2"/>${key}`,
  basemapMaptilerSatellite: `${tile}<path d="M2.5 12 6 9l2.8 2.4 3-3.4 5.7 5.5" stroke-width="1.1"/><path d="M2.5 12 6 9l2.8 2.4 3-3.4 5.7 5.5V17.5h-15z" fill="currentColor" fill-opacity=".25" stroke="none"/>${key}`,
} as const;
