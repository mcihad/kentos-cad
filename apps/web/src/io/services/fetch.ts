import type { ConnectionSecret } from '../../contracts/generated/ConnectionSecret';
import type { ServiceConnection } from '../../contracts/generated/ServiceConnection';
import type { ServicesModule } from './module';

/**
 * A map service's request from the browser (docs/adr/0208 §12, §13): the connection's proof added by the services
 * core (for ArcGIS and OAuth 2 a token asked for first and kept until a minute before its end), sent straight to the
 * service; when the browser refuses it (CORS, the network) and the page is signed in to KentOS, sent again through
 * `kentosd`'s proxy (`/v1/proxy?url=…`, the request's headers as `x-kentos-header-<name>`), which the origin then keeps
 * for the session. A secret never goes into a message the page logs.
 */

/** A request as the services module writes it. */
export interface Wire {
  url: string;
  headers: [string, string][];
  body: string | null;
  media: string | null;
}

/** A connection and this device's secret for it. */
export interface Proof {
  connection: ServiceConnection;
  secret: ConnectionSecret | null;
}

export interface Answer {
  status: number;
  url: string;
  headers: Headers;
  bytes: Uint8Array;
}

export interface FetchOptions {
  /** The proxy may be used: the page is signed in to KentOS. */
  proxy: boolean;
  /** Where the request is from (ArcGIS binds its tokens to it). */
  referer: string;
  signal?: AbortSignal;
  /** How the browser's cache is used; its default when absent (Önbelleği temizle asks past it). */
  cache?: RequestCache;
}

/** Origins the browser cannot reach straight: through the proxy for the session. */
const viaProxy = new Set<string>();
/** Tokens by connection. */
const tokens = new Map<string, { value: string; expiresMs: number }>();

const originOf = (url: string): string => {
  try {
    return new URL(url).origin;
  } catch {
    return '';
  }
};

async function send(w: Wire, o: FetchOptions, proxied: boolean): Promise<Response> {
  const init: RequestInit = {
    method: w.body === null ? 'GET' : 'POST',
    body: w.body ?? undefined,
    signal: o.signal,
    credentials: proxied ? 'same-origin' : 'omit',
    cache: o.cache ?? 'default',
  };
  if (!proxied) {
    const headers = new Headers(w.headers);
    if (w.media) headers.set('content-type', w.media);
    init.headers = headers;
    return fetch(w.url, init);
  }
  const headers = new Headers({ 'x-kentos-client': 'web' });
  for (const [name, value] of w.headers) headers.set(`x-kentos-header-${name.toLowerCase()}`, value);
  if (w.media) headers.set('content-type', w.media);
  init.headers = headers;
  return fetch(`/v1/proxy?url=${encodeURIComponent(w.url)}`, init);
}

/** Sends `w` straight, or through the proxy where the browser cannot (and may). */
async function reach(w: Wire, o: FetchOptions): Promise<Response> {
  const origin = originOf(w.url);
  if (o.proxy && viaProxy.has(origin)) return send(w, o, true);
  try {
    return await send(w, o, false);
  } catch (e) {
    if (o.signal?.aborted || !o.proxy) throw e;
    viaProxy.add(origin);
    return send(w, o, true);
  }
}

/** A token for an ArcGIS or OAuth 2 connection; none for the others. */
async function tokenFor(m: ServicesModule, p: Proof, o: FetchOptions): Promise<string | undefined> {
  if (!p.secret || (p.connection.auth !== 'arcgis' && p.connection.auth !== 'oauth2')) return undefined;
  const key = `${p.connection.origin}|${p.connection.id}`;
  const kept = tokens.get(key);
  if (kept && kept.expiresMs > Date.now() + 60_000) return JSON.stringify(kept);
  const req = m.tokenRequest(JSON.stringify(p.connection), JSON.stringify(p.secret), o.referer);
  if (!req) return undefined;
  const res = await reach(JSON.parse(req) as Wire, o);
  const t = JSON.parse(m.readToken(JSON.stringify(p.connection), await res.text(), Date.now())) as { value: string; expiresMs: number };
  tokens.set(key, t);
  return JSON.stringify(t);
}

/**
 * `w` sent with `proof` and answered: its status, address, headers and bytes; why not in the windows' words (a
 * missing secret, the network, the browser's refusal while signed out).
 */
export async function serviceFetch(m: ServicesModule, w: Wire, proof: Proof | null, o: FetchOptions): Promise<Answer> {
  let wire = w;
  if (proof) {
    const missing = m.authMissing(JSON.stringify(proof.connection), proof.secret ? JSON.stringify(proof.secret) : undefined);
    if (missing) throw new Error(missing);
    if (proof.secret) {
      const token = await tokenFor(m, proof, o);
      wire = JSON.parse(m.authApply(JSON.stringify(proof.connection), JSON.stringify(proof.secret), token, JSON.stringify(w))) as Wire;
    }
  }
  let res: Response;
  try {
    res = await reach(wire, o);
  } catch (e) {
    if (o.signal?.aborted) throw e;
    throw new Error(
      o.proxy
        ? 'Servise ulaşılamadı: adresi ve ağı denetleyin.'
        : 'Tarayıcı servise ulaşamadı (CORS ya da ağ). KentOS hesabınızla oturum açın: istek sunucu üzerinden gider.',
    );
  }
  return { status: res.status, url: res.url || wire.url, headers: res.headers, bytes: new Uint8Array(await res.arrayBuffer()) };
}

/** `w` sent and its answer as text when it succeeded; why not in the windows' words. */
export async function serviceText(m: ServicesModule, w: Wire, proof: Proof | null, o: FetchOptions): Promise<string> {
  const a = await serviceFetch(m, w, proof, o);
  if (a.status === 401 || a.status === 403) throw new Error(`Sunucu erişimi reddetti (${a.status}): bağlantının değerlerini Bağlantılar'dan denetleyin.`);
  if (a.status === 404) throw new Error(`Adres bulunamadı (404): ${a.url}`);
  if (a.status < 200 || a.status > 299) throw new Error(`Sunucu ${a.status} dedi.`);
  return new TextDecoder().decode(a.bytes);
}

/** A plain GET's request. */
export const getWire = (url: string): Wire => ({ url, headers: [], body: null, media: null });
