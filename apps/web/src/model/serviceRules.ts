/**
 * The rules of a layer's map service, of where its objects came from and of the project's connections
 * (docs/adr/0208 §2), as the contract says them (`kentos_contracts::service`): the same checks in the same order and
 * the same words, so a command refuses in the browser what it refuses on the desktop and the files refuse. Both are
 * held to fixtures/services/v1/rules.json; tools/kcad/kcad.py reads the files by its own copy.
 */
import type { AuthKind } from '../contracts/generated/AuthKind';
import type { FeatureFeed } from '../contracts/generated/FeatureFeed';
import type { FeedKind } from '../contracts/generated/FeedKind';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { ServiceKind } from '../contracts/generated/ServiceKind';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';
import type { TileGrid } from '../contracts/generated/TileGrid';

export const MAX_SERVICE_URL = 4096;
export const MAX_SERVICE_LAYERS = 64;
export const MAX_TILE_MATRICES = 40;
export const MAX_SERVICE_PARAMS = 32;
export const MAX_SUBDOMAINS = 16;
export const MAX_ZOOM = 30;
export const MIN_SERVICE_OPACITY = 0.1;
export const MAX_SERVICE_OPACITY = 1;
export const MAX_FEED_LIMIT = 500_000;
export const MAX_CONNECTIONS = 256;
export const MAX_AUTH_NAMES = 8;
export const GOOGLE_MAP_TYPES = ['roadmap', 'satellite', 'terrain', 'hybrid'] as const;
export const WMS_VERSIONS = ['1.1.1', '1.3.0'] as const;
export const WFS_VERSIONS = ['1.0.0', '1.1.0', '2.0.0'] as const;

export const SERVICE_KINDS: readonly ServiceKind[] = ['xyz', 'wms', 'wmts', 'ogcTiles', 'arcgis', 'google', 'vector'];
export const FEED_KINDS: readonly FeedKind[] = ['wfs', 'ogcFeatures', 'arcgis', 'geojson'];
export const AUTH_KINDS: readonly AuthKind[] = ['none', 'query', 'header', 'basic', 'bearer', 'arcgis', 'oauth2', 'google'];

/** A service kind's name in the windows. */
export const SERVICE_KIND_LABELS: Record<ServiceKind, string> = {
  xyz: 'XYZ / TMS',
  wms: 'WMS',
  wmts: 'WMTS',
  ogcTiles: 'OGC API Tiles',
  arcgis: 'ArcGIS REST',
  google: 'Google Haritalar',
  vector: 'Vektör karo',
};

/** A feed kind's name in the windows. */
export const FEED_KIND_LABELS: Record<FeedKind, string> = {
  wfs: 'WFS',
  ogcFeatures: 'OGC API Features',
  arcgis: 'ArcGIS REST',
  geojson: 'GeoJSON adresi',
};

/** A proof's name in the windows. */
export const AUTH_KIND_LABELS: Record<AuthKind, string> = {
  none: 'Yok',
  query: 'Adreste parametre',
  header: 'Başlıkta değer',
  basic: 'Kullanıcı adı ve parola',
  bearer: 'Belirteç (Bearer)',
  arcgis: 'ArcGIS belirteci',
  oauth2: 'OAuth 2 (istemci kimliği)',
  google: 'Google API anahtarı',
};

/** Rust's `char::is_control`: Unicode's Cc, C0, DEL and C1. */
function control(text: string): boolean {
  for (const ch of text) {
    const c = ch.codePointAt(0) ?? 0;
    if (c < 0x20 || (c >= 0x7f && c <= 0x9f)) return true;
  }
  return false;
}

/** Letters as Rust counts them: Unicode scalar values. */
function letters(text: string): number {
  let n = 0;
  for (const _ of text) n++;
  return n;
}

function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (c) => c.toLowerCase());
}

/** Why `text` is no HTTP or HTTPS address of at most MAX_SERVICE_URL letters without control characters. */
export function httpProblem(what: string, text: string): string | null {
  const lower = asciiLower(text.trim());
  const rest = lower.startsWith('https://') ? lower.slice(8) : lower.startsWith('http://') ? lower.slice(7) : null;
  if (rest === null || rest === '' || rest.startsWith('/'))
    return `${what} bir HTTP ya da HTTPS adresi olmalı (http:// ya da https:// ile başlamalı); “${text}” değil.`;
  if (letters(text) > MAX_SERVICE_URL || control(text)) return `${what} en çok ${MAX_SERVICE_URL} harf olmalı ve denetim karakteri içermemeli.`;
  return null;
}

/** A parameter's or a header's name: 1 to 64 letters, digits, `-`, `_`, `.`. */
function tokenName(name: string): boolean {
  return name.length > 0 && name.length <= 64 && /^[A-Za-z0-9._-]+$/.test(name);
}

function plain(text: string, most: number): boolean {
  return letters(text) <= most && !control(text);
}

export function gridProblem(g: TileGrid): string | null {
  if (g.srid === 0) return 'Karo ızgarasının sistemi bir EPSG kodu olmalı.';
  if (g.matrices.length === 0 || g.matrices.length > MAX_TILE_MATRICES) return `Karo ızgarasının 1 ile ${MAX_TILE_MATRICES} arasında matrisi olmalı.`;
  for (let i = 0; i < g.matrices.length; i++) {
    const m = g.matrices[i];
    const n = i + 1;
    if (m.id === '' || !plain(m.id, 128)) return `${n}. matrisin adı 1 ile 128 harf arasında olmalı.`;
    if (g.matrices.slice(0, i).some((o) => o.id === m.id)) return `“${m.id}” matrisi iki kez var.`;
    if (!(Number.isFinite(m.resolution) && m.resolution > 0)) return `${n}. matrisin pikseli sıfırdan büyük, sonlu olmalı.`;
    if (!(Number.isFinite(m.x0) && Number.isFinite(m.y0))) return `${n}. matrisin köşesi sonlu olmalı.`;
    const side = (v: number) => v >= 1 && v <= 4096;
    if (!side(m.tileWidth) || !side(m.tileHeight)) return `${n}. matrisin karosu 1 ile 4096 piksel arasında olmalı.`;
    const count = (v: number) => v >= 1 && v <= 2 ** 40;
    if (!count(m.matrixWidth) || !count(m.matrixHeight)) return `${n}. matrisin karo sayısı 1 ile 2⁴⁰ arasında olmalı.`;
  }
  return null;
}

/** Rust's `{}` of an f64: the shortest round trip, integers without a point. */
function rustFloat(x: number): string {
  if (Number.isNaN(x)) return 'NaN';
  if (!Number.isFinite(x)) return x > 0 ? 'inf' : '-inf';
  return String(x);
}

/** Why the fields do not make a service layer, in the commands' words; null when they do. */
export function serviceProblem(s: ServiceLayer): string | null {
  const layers = s.layers ?? [];
  const subdomains = s.subdomains ?? [];
  const params = s.params ?? [];
  if (s.kind === 'google') {
    if (s.url !== '') return 'Google katmanının adresi boş olmalı; karoların adresi oturumdan gelir.';
    if (!(s.style !== undefined && (GOOGLE_MAP_TYPES as readonly string[]).includes(s.style)))
      return `Google katmanının harita türü ${GOOGLE_MAP_TYPES.join(', ')} türlerinden biri olmalı.`;
    if (s.connection === undefined) return 'Google katmanının API anahtarı bir bağlantıda olmalı.';
  } else {
    const p = httpProblem('Servisin adresi', s.url);
    if (p) return p;
  }
  switch (s.kind) {
    case 'xyz': {
      const u = s.url;
      const rows = u.includes('{y}') || u.includes('{-y}');
      if (!(u.includes('{quadkey}') || (u.includes('{z}') && u.includes('{x}') && rows)))
        return 'XYZ şablonunda {z}, {x} ve {y} (ya da {-y}) ya da {quadkey} olmalı.';
      if (u.includes('{s}') && subdomains.length === 0) return 'Şablonda {s} var; alt alanları verin (a, b, c gibi).';
      break;
    }
    case 'wms':
      if (layers.length === 0) return 'WMS katmanı en az bir katman adı istemeli.';
      if (s.srid === undefined) return 'WMS katmanının istenen sistemi (srid) olmalı.';
      if (s.version !== undefined && !(WMS_VERSIONS as readonly string[]).includes(s.version))
        return `WMS sürümü ${WMS_VERSIONS.join(' ya da ')} olmalı; “${s.version}” değil.`;
      break;
    case 'wmts':
      if (layers.length !== 1) return 'WMTS katmanı tek bir katman göstermeli.';
      if (s.grid === undefined || s.matrixSet === undefined) return 'WMTS katmanının karo ızgarası (grid) ve matris kümesinin adı (matrixSet) olmalı.';
      break;
    case 'ogcTiles':
      if (s.grid === undefined || s.template === undefined) return 'OGC API Tiles katmanının ızgarası ve karo şablonu olmalı.';
      break;
    case 'arcgis':
      if (s.srid === undefined && s.grid === undefined) return 'ArcGIS katmanının ızgarası ya da istenen sistemi (srid) olmalı.';
      break;
    default:
      break;
  }
  if (layers.length > MAX_SERVICE_LAYERS || layers.some((l) => l === '' || !plain(l, 256)))
    return `Katman adları en çok ${MAX_SERVICE_LAYERS} tane ve 1 ile 256 harf arasında olmalı.`;
  const texts: [string, string | undefined, number][] = [
    ['Stil', s.style, 256],
    ['Biçim', s.format, 128],
    ['Atıf', s.attribution, 512],
    ['Sürüm', s.version, 16],
    ['Hazır altlık', s.preset, 64],
    ['Bağlantı', s.connection, 64],
    ['Matris kümesi', s.matrixSet, 256],
  ];
  for (const [what, t, most] of texts)
    if (t !== undefined && (t === '' || !plain(t, most))) return `${what} 1 ile ${most} harf arasında olmalı ve denetim karakteri içermemeli.`;
  if (s.template !== undefined) {
    const p = httpProblem('Karo şablonu', s.template);
    if (p) return p;
  }
  if (s.srid === 0) return 'İstenen sistem bir EPSG kodu olmalı.';
  if (s.grid !== undefined) {
    const p = gridProblem(s.grid);
    if (p) return p;
  }
  if (s.tileSize !== undefined && !(s.tileSize >= 64 && s.tileSize <= 4096)) return 'Karonun boyu 64 ile 4096 piksel arasında olmalı.';
  const lo = s.minZoom ?? 0;
  const hi = s.maxZoom ?? MAX_ZOOM;
  if (lo > hi || hi > MAX_ZOOM) return `Katlar 0 ile ${MAX_ZOOM} arasında olmalı, en küçüğü en büyüğünden büyük olmamalı.`;
  if (subdomains.length > MAX_SUBDOMAINS || subdomains.some((x) => !tokenName(x)))
    return `Alt alanlar en çok ${MAX_SUBDOMAINS} tane; her biri harf, rakam ve tire olmalı.`;
  if (params.length > MAX_SERVICE_PARAMS) return `En çok ${MAX_SERVICE_PARAMS} ek parametre olabilir.`;
  for (const p of params)
    if (!tokenName(p.name) || !plain(p.value, 1024))
      return `Ek parametre “${p.name}”: adı harf, rakam, tire ve alt çizgi; değeri en çok 1024 harf olmalı.`;
  const o = s.opacity;
  if (o !== undefined && !(Number.isFinite(o) && o >= MIN_SERVICE_OPACITY && o <= MAX_SERVICE_OPACITY))
    return `Servis katmanının donukluğu ${rustFloat(MIN_SERVICE_OPACITY)} ile ${rustFloat(MAX_SERVICE_OPACITY)} arasında olmalı; ${rustFloat(o)} verildi.`;
  const b = s.bbox;
  if (b !== undefined) {
    const [w, so, e, n] = b;
    const ok = b.every((v) => Number.isFinite(v)) && w >= -180 && e <= 180 && so >= -90 && n <= 90 && w <= e && so <= n;
    if (!ok)
      return 'Servisin kapsamı WGS 84 derecesinde batı, güney, doğu ve kuzey olmalı (boylam −180 ile 180, enlem −90 ile 90 arasında; batı doğudan, güney kuzeyden büyük değil).';
  }
  return null;
}

/** Why the fields do not make a feed, in the commands' words; null when they do. */
export function feedProblem(f: FeatureFeed): string | null {
  const p = httpProblem('Verinin adresi', f.url);
  if (p) return p;
  if (f.name === undefined) {
    if (f.kind === 'wfs' || f.kind === 'ogcFeatures' || f.kind === 'arcgis')
      return `${FEED_KIND_LABELS[f.kind]} kaynağının tür, koleksiyon ya da katman adı (name) olmalı.`;
  } else if (f.name === '' || !plain(f.name, 256)) return 'Tür adı 1 ile 256 harf arasında olmalı.';
  if (f.version !== undefined) {
    if (f.kind !== 'wfs') return 'Sürüm yalnız WFS kaynağının olur.';
    if (!(WFS_VERSIONS as readonly string[]).includes(f.version)) return `WFS sürümü ${WFS_VERSIONS.join(', ')} olmalı; “${f.version}” değil.`;
  }
  if (f.srid === 0) return 'İstenen sistem bir EPSG kodu olmalı.';
  const b = f.bbox;
  if (b !== undefined && !(b.every((v) => Number.isFinite(v)) && b[0] <= b[2] && b[1] <= b[3]))
    return 'İstenen alan sonlu dört sayı olmalı, en küçükler en büyüklerden büyük olmamalı.';
  if (f.limit !== undefined && !(f.limit >= 1 && f.limit <= MAX_FEED_LIMIT)) return `En çok nesne 1 ile ${MAX_FEED_LIMIT} arasında olmalı.`;
  const texts: [string, string | undefined, number][] = [
    ['Süzgeç', f.filter, 8192],
    ['Anahtar alan', f.key, 128],
    ['Bağlantı', f.connection, 64],
    ['Son alınış', f.fetched, 40],
  ];
  for (const [what, t, most] of texts)
    if (t !== undefined && (t === '' || !plain(t, most))) return `${what} 1 ile ${most} harf arasında olmalı ve denetim karakteri içermemeli.`;
  return null;
}

/** An authority's host and port; null when an IPv6 host's brackets do not close. */
function splitAuthority(a: string): [string, string | null] | null {
  if (a.startsWith('[')) {
    const close = a.indexOf(']', 1);
    if (close < 0) return null;
    const host = a.slice(0, close + 1);
    const after = a.slice(close + 1);
    if (after.startsWith(':')) return [host, after.slice(1)];
    return after === '' ? [host, null] : null;
  }
  const at = a.lastIndexOf(':');
  return at < 0 ? [a, null] : [a.slice(0, at), a.slice(at + 1)];
}

/** Rust's `str::parse::<u16>`: digits only (one leading `+` allowed), 0 to 65535. */
function port(p: string): number | null {
  if (!/^\+?[0-9]+$/.test(p)) return null;
  const n = Number(p.replace('+', ''));
  return n <= 65535 ? n : null;
}

/** Why `origin` is not `http(s)://host[:port]` in lower case without a path; null when it is. */
export function originProblem(origin: string): string | null {
  const rest = origin.startsWith('https://') ? origin.slice(8) : origin.startsWith('http://') ? origin.slice(7) : null;
  let ok = false;
  if (rest !== null && !/[/?#@]/.test(rest)) {
    const split = splitAuthority(rest);
    if (split) {
      const [host, p] = split;
      const name = host.startsWith('[')
        ? host.length > 2 && /^[0-9a-f:.]+$/.test(host.slice(1, -1))
        : host.length > 0 && host.length <= 253 && /^[a-z0-9.-]+$/.test(host);
      const n = p === null ? null : port(p);
      ok = name && (p === null || (n !== null && n > 0));
    }
  }
  return ok ? null : `Bağlantının kökeni küçük harfle şema ve makine (https://ornek.gov.tr, isteğe bağlı :kapı) olmalı, yolu olmamalı; “${origin}” değil.`;
}

/** The origin of an address: scheme, host and port in lower case, the scheme's own port left out; null for none. */
export function originOf(url: string): string | null {
  const text = url.trim();
  const colon = text.indexOf('://');
  if (colon < 0) return null;
  const scheme = asciiLower(text.slice(0, colon));
  if (scheme !== 'http' && scheme !== 'https') return null;
  const rest = text.slice(colon + 3);
  const end = rest.search(/[/?#]/);
  let authority = end < 0 ? rest : rest.slice(0, end);
  const at = authority.lastIndexOf('@');
  if (at >= 0) authority = authority.slice(at + 1);
  const split = splitAuthority(authority);
  if (!split || split[0] === '') return null;
  const host = asciiLower(split[0]);
  const p = split[1];
  const fallback = scheme === 'https' ? '443' : '80';
  return p !== null && p !== fallback && p !== '' ? `${scheme}://${host}:${p}` : `${scheme}://${host}`;
}

/** Why the fields do not make a connection, in the commands' words; null when they do. */
export function connectionProblem(c: ServiceConnection): string | null {
  const names = c.names ?? [];
  if (!(c.id.length > 0 && c.id.length <= 64 && /^[A-Za-z0-9_-]+$/.test(c.id))) return 'Bağlantının kimliği 1 ile 64 harf, rakam, tire ya da alt çizgi olmalı.';
  if (c.name.trim() === '' || !plain(c.name, 128)) return 'Bağlantının adı 1 ile 128 harf arasında olmalı.';
  const o = originProblem(c.origin);
  if (o) return o;
  const named = c.auth === 'query' || c.auth === 'header';
  if (named && (names.length === 0 || names.length > MAX_AUTH_NAMES)) return `${AUTH_KIND_LABELS[c.auth]} için 1 ile ${MAX_AUTH_NAMES} arasında ad olmalı.`;
  if (!named && names.length > 0) return `${AUTH_KIND_LABELS[c.auth]} ad taşımaz.`;
  if (names.some((n) => !tokenName(n))) return 'Parametre ve başlık adları harf, rakam, tire, nokta ve alt çizgiden olmalı.';
  if (c.tokenUrl === undefined) {
    if (c.auth === 'oauth2') return 'OAuth 2 bağlantısının belirteç adresi olmalı.';
  } else {
    if (c.auth !== 'arcgis' && c.auth !== 'oauth2') return 'Belirteç adresi yalnız ArcGIS ve OAuth 2 bağlantısının olur.';
    const p = httpProblem('Belirteç adresi', c.tokenUrl);
    if (p) return p;
  }
  if (c.scope !== undefined) {
    if (c.auth !== 'oauth2') return 'Kapsam (scope) yalnız OAuth 2 bağlantısının olur.';
    if (c.scope === '' || !plain(c.scope, 256)) return 'Kapsam 1 ile 256 harf arasında olmalı.';
  }
  return null;
}

/** Why the project's connections are not ones; null when they are. */
export function connectionsProblem(list: readonly ServiceConnection[]): string | null {
  if (list.length > MAX_CONNECTIONS) return `Projenin en çok ${MAX_CONNECTIONS} bağlantısı olabilir.`;
  for (let i = 0; i < list.length; i++) {
    const c = list[i];
    const p = connectionProblem(c);
    if (p) return `“${c.name}” bağlantısı: ${p}`;
    if (list.slice(0, i).some((o) => o.id === c.id)) return `“${c.id}” kimlikli bağlantı iki kez var.`;
  }
  return null;
}

/** A layer node as the links read it. */
export interface LinkNode {
  id?: string;
  name: string;
  service?: ServiceLayer;
  feed?: FeatureFeed;
  children?: readonly LinkNode[];
}

/** How a drawing's services break their links (docs/adr/0208 §2). */
export type ServiceLinkFault =
  | { kind: 'connection'; layer: string; connection: string }
  | { kind: 'both'; layer: string }
  | { kind: 'object'; index: number };

/** What is wrong, in the readers' and commands' words. */
export function linkFaultWords(f: ServiceLinkFault): string {
  switch (f.kind) {
    case 'connection':
      return `“${f.layer}” katmanının bağlantısı “${f.connection}” projenin bağlantıları arasında yok`;
    case 'both':
      return `“${f.layer}” katmanı hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz`;
    case 'object':
      return 'nesne bir servis katmanında; servis katmanı nesne tutmaz';
  }
}

/** The first broken link of a drawing's services; null when all hold. */
export function serviceLinks(
  layers: readonly LinkNode[],
  connections: readonly ServiceConnection[],
  entities: readonly { layerId: string }[],
): ServiceLinkFault | null {
  const all: LinkNode[] = [];
  const walk = (nodes: readonly LinkNode[]) => {
    for (const n of nodes) {
      all.push(n);
      walk(n.children ?? []);
    }
  };
  walk(layers);
  const known = new Set(connections.map((c) => c.id));
  const served = new Set<string>();
  for (const n of all) {
    if (n.service !== undefined && n.feed !== undefined) return { kind: 'both', layer: n.name };
    for (const c of [n.service?.connection, n.feed?.connection])
      if (c !== undefined && !known.has(c)) return { kind: 'connection', layer: n.name, connection: c };
    if (n.service !== undefined && n.id !== undefined) served.add(n.id);
  }
  if (served.size === 0) return null;
  const index = entities.findIndex((e) => served.has(e.layerId));
  return index < 0 ? null : { kind: 'object', index };
}

/** A copy with every object's keys in one order: two services are the same whatever order their fields came in. */
export function canonical<T>(value: T): T {
  if (Array.isArray(value)) return value.map(canonical) as T;
  if (value === null || typeof value !== 'object') return value;
  const out: Record<string, unknown> = {};
  for (const k of Object.keys(value).sort()) {
    const v = (value as Record<string, unknown>)[k];
    if (v !== undefined) out[k] = canonical(v);
  }
  return out as T;
}
