/// <reference lib="webworker" />
import { crsBySrid } from '../../geo/crs';
import { systemOf } from '../../model/geom/crsSystem';
import { perHost, preset } from '../../model/servicePresets';
import { getWire, serviceFetch, serviceText, type Proof, type Wire } from './fetch';
import { loadServices, type ServicesModule } from './module';
import type { FromWorker, Register, ToWorker } from './protocol';
import type { GoogleSession, LabelStore, VectorStyle, View } from './pkg/kentos_services_wasm';

/**
 * The page's services worker (docs/adr/0208 §3, §8, §9), twin of the desktop's `services/net.rs` and `resolve.rs`:
 * a service is resolved here (a vector service's style and TileJSON, Google's session, a token), its tiles fetched in
 * the order the frames ask for them (the view's centre first; at most two at a time from OpenStreetMap's servers, six
 * from another, 24 in all; a tile the frames no longer ask for is dropped and its request aborted), a picture decoded
 * by the browser and cut into the raster atlas's slots by the services core, a vector tile read once and built by its
 * style at the zoom step asked for. The browser's own cache keeps the answers by HTTP's rules. The labels of the tiles
 * built are placed here too, for the screen the page gives.
 */

declare const self: DedicatedWorkerGlobalScope;

interface Entry {
  reg: Register;
  serviceJson: string;
  state: 'resolving' | 'ready' | 'failed';
  template: string | null;
  style: VectorStyle | null;
  view: View | null;
  session: GoogleSession | null;
  tileW: number;
  tileH: number;
  vector: boolean;
  perHost: number;
  host: string;
}

interface Job {
  key: string;
  level: number;
  col: number;
  row: number;
  step: number | null;
  abort: AbortController | null;
}

/** Requests in flight in all. */
const TOTAL = 24;
/** Vector tiles kept read (their bytes). */
const KEPT_TILES = 384;

const entries = new Map<string, Entry>();
const queue: Job[] = [];
const running = new Map<string, Job>();
const perHostRunning = new Map<string, number>();
const kept = new Map<string, Uint8Array>();
let labels: LabelStore | null = null;
let nextLabel = 1;
let proxyOn = false;

const post = (m: FromWorker, transfer: Transferable[] = []) => self.postMessage(m, transfer);
const tileKey = (key: string, level: number, col: number, row: number) => `${key}|${level}|${col}|${row}`;
const hostOf = (url: string): string => {
  try {
    return new URL(url.replace(/\{[^}]*\}/g, '0')).host;
  } catch {
    return '';
  }
};

function proofOf(e: Entry): Proof | null {
  return e.reg.connection ? { connection: e.reg.connection, secret: e.reg.secret } : null;
}

/** Services whose answers are each asked once past the browser's cache (Önbelleği temizle): the addresses asked since. */
const fresh = new Map<string, Set<string>>();

function opts(e: Entry, signal?: AbortSignal, url?: string) {
  const asked = fresh.get(e.reg.key);
  let cache: RequestCache | undefined;
  if (asked && url !== undefined && !asked.has(url)) {
    asked.add(url);
    cache = 'reload';
  }
  return { proxy: proxyOn, referer: e.reg.service.url || 'https://tile.googleapis.com', signal, cache };
}

/** The system of an EPSG code in the core's JSON; null for one the register does not have. */
function systemJson(srid: number): string | null {
  const crs = crsBySrid(srid);
  const s = crs ? systemOf(crs) : null;
  return s ? JSON.stringify(s) : null;
}

/** Reads what a service needs before its tiles. */
async function resolve(e: Entry): Promise<void> {
  const m = await loadServices();
  labels ??= new m.LabelStore();
  const s = e.reg.service;
  const proof = proofOf(e);
  let credit = s.attribution ?? '';
  let notes: string[] = [];
  let tileSize = 0;
  if (s.kind === 'vector') {
    if (s.url.includes('{z}')) {
      e.style = m.VectorStyle.basic(s.url, s.minZoom ?? 0, s.maxZoom ?? 14, []);
      e.template = s.url;
    } else {
      const text = await serviceText(m, getWire(s.url), proof, opts(e, undefined, s.url));
      let isStyle = false;
      try {
        const v = JSON.parse(text) as { layers?: unknown; sources?: unknown };
        isStyle = Array.isArray(v.layers) && typeof v.sources === 'object' && v.sources !== null;
      } catch {
        throw new Error('Yanıt JSON değil: adres bir MapLibre stili ya da TileJSON olmalı.');
      }
      if (isStyle) {
        const style = m.VectorStyle.parse(text, s.url);
        const src = JSON.parse(style.source() ?? 'null') as { tiles: string[]; url: string | null; attribution: string | null } | null;
        if (!src) throw new Error('Stilin vektör kaynağı yok.');
        let tiles = src.tiles;
        if (!tiles.length) {
          if (!src.url) throw new Error('Stilin vektör kaynağının adresi yok.');
          const tj = JSON.parse(m.readTileJson(await serviceText(m, getWire(src.url), proof, opts(e, undefined, src.url)), src.url)) as { tiles: string[]; attribution?: string };
          tiles = tj.tiles;
          credit ||= tj.attribution ?? '';
        } else credit ||= src.attribution ?? '';
        e.style = style;
        e.template = tiles[0] ?? null;
        notes = style.notes();
      } else {
        const tj = JSON.parse(m.readTileJson(text, s.url)) as { tiles: string[]; minZoom: number; maxZoom: number; attribution?: string; layers: string[] };
        e.style = m.VectorStyle.basic(tj.tiles[0], tj.minZoom, tj.maxZoom, tj.layers);
        e.template = tj.tiles[0] ?? null;
        credit ||= tj.attribution ?? '';
      }
      if (!e.template) throw new Error('Vektör karoların adresi yok.');
    }
  } else if (s.kind === 'google') {
    if (!proof) throw new Error('Google katmanının API anahtarı bir bağlantıda olmalı.');
    const req = JSON.parse(m.googleSessionRequest(s.style ?? 'roadmap', e.reg.hidpi)) as Wire;
    const body = await serviceText(m, req, proof, opts(e));
    e.session = m.GoogleSession.read(body);
    tileSize = e.session.tileSize();
    credit ||= 'Google Maps';
  }
  const src = JSON.parse(m.sourceOf(e.serviceJson, e.template ?? undefined) ?? 'null') as { srid: number; vector: boolean; tileWidth: number; tileHeight: number } | null;
  if (!src) throw new Error('Servisin karo ızgarası ya da istenecek sistemi yok; servisi yeniden ekleyin.');
  const same = src.srid === e.reg.projectSrid && !e.reg.custom;
  const theirs = systemJson(src.srid);
  if (!same && !theirs) throw new Error(`Servisin koordinat sistemi (EPSG:${src.srid}) KentOS'un kaydında yok; servisi başka bir sistemde isteyin.`);
  if (!same && !e.reg.ours) throw new Error('Projenin koordinat sistemi yok: harita servisi gösterilemez. Proje ayarlarında projeye bir koordinat sistemi verin.');
  e.view = new m.View(e.serviceJson, e.template ?? undefined, e.reg.ours ?? undefined, theirs ?? undefined, e.reg.choices, same, tileSize);
  e.vector = src.vector;
  e.tileW = tileSize || src.tileWidth;
  e.tileH = tileSize || src.tileHeight;
  e.host = hostOf(e.template ?? (s.kind === 'google' ? 'https://tile.googleapis.com' : s.url));
  e.state = 'ready';
  const c = JSON.parse(m.credit(credit)) as { text: string; links: [string, string][] };
  // A ready basemap's credit without a link takes its preset's page (the desktop's `credit_of`).
  const page = s.preset !== undefined ? preset(s.preset)?.attributionUrl : undefined;
  if (!c.links.length && c.text && page) c.links.push([page.replace(/^[a-z]+:\/\//i, '').replace(/\/+$/, ''), page]);
  post({ type: 'ready', key: e.reg.key, srid: src.srid, template: e.template, tileSize, credit: c, vector: e.vector, notes });
  pump();
}

/** A tile's address with no proof. */
function urlOf(m: ServicesModule, e: Entry, j: Job): string | null {
  if (e.session) return e.session.tileUrl(j.level, j.col, j.row);
  return m.tileUrl(e.serviceJson, e.template ?? undefined, j.level, j.col, j.row, e.reg.hidpi && e.reg.service.url.includes('{r}')) ?? null;
}

/** A picture's bytes decoded by the browser and cut into slots. */
async function cut(m: ServicesModule, e: Entry, bytes: Uint8Array): Promise<{ across: number; down: number; bytes: Uint8Array } | null> {
  const bitmap = await createImageBitmap(new Blob([bytes as Uint8Array<ArrayBuffer>]), { premultiplyAlpha: 'none', colorSpaceConversion: 'none' });
  const { width, height } = bitmap;
  const canvas = new OffscreenCanvas(width, height);
  const g = canvas.getContext('2d', { willReadFrequently: true });
  if (!g) throw new Error('Tarayıcı karoyu çizemedi.');
  g.drawImage(bitmap, 0, 0);
  bitmap.close();
  const rgba = new Uint8Array(g.getImageData(0, 0, width, height).data.buffer);
  const c = m.cutPicture(width, height, rgba, e.tileW, e.tileH);
  if (!c) return null;
  const out = { across: c.across(), down: c.down(), bytes: c.bytes() };
  c.free();
  return out;
}

/** A vector tile's bytes built at its step. */
function buildVector(e: Entry, j: Job, bytes: Uint8Array): void {
  if (!e.style || !e.view || !labels || j.step === null) return;
  const id = nextLabel++;
  const built = e.style.build(e.view, bytes, j.level, j.col, j.row, j.step / 4, e.reg.origin[0], e.reg.origin[1], id, labels);
  const data = built.data();
  const json = built.json();
  built.free();
  post({ type: 'vector', key: e.reg.key, level: j.level, col: j.col, row: j.row, step: j.step, id, json, data }, [data.buffer]);
}

async function run(j: Job): Promise<void> {
  const e = entries.get(j.key);
  if (!e || e.state !== 'ready') return;
  const m = await loadServices();
  const k = tileKey(j.key, j.level, j.col, j.row);
  if (e.vector) {
    const have = kept.get(k);
    if (have) {
      buildVector(e, j, have);
      return;
    }
  }
  const url = urlOf(m, e, j);
  if (!url) {
    post({ type: 'tileFailed', key: j.key, level: j.level, col: j.col, row: j.row, message: 'Karonun adresi kurulamadı.', quiet: false });
    return;
  }
  j.abort = new AbortController();
  try {
    const a = await serviceFetch(m, getWire(url), proofOf(e), opts(e, j.abort.signal, url));
    if (a.status === 204 || a.status === 404) {
      post({ type: 'picture', key: j.key, level: j.level, col: j.col, row: j.row, across: 0, down: 0, bytes: null });
      return;
    }
    if (a.status < 200 || a.status > 299) {
      const quiet = a.status === 429 || a.status >= 500;
      const words =
        a.status === 401 || a.status === 403
          ? `sunucu erişimi reddetti (${a.status}). Bağlantının bilgilerini Bağlantılar penceresinden denetleyin.`
          : `sunucu ${a.status} dedi.`;
      post({ type: 'tileFailed', key: j.key, level: j.level, col: j.col, row: j.row, message: words, quiet });
      return;
    }
    if (e.vector) {
      kept.set(k, a.bytes);
      if (kept.size > KEPT_TILES) kept.delete(kept.keys().next().value!);
      buildVector(e, j, a.bytes);
      return;
    }
    const c = await cut(m, e, a.bytes);
    post(
      { type: 'picture', key: j.key, level: j.level, col: j.col, row: j.row, across: c?.across ?? 0, down: c?.down ?? 0, bytes: c?.bytes ?? null },
      c ? [c.bytes.buffer] : [],
    );
  } catch (err) {
    if (j.abort.signal.aborted) return;
    const message = err instanceof Error ? err.message : String(err);
    post({ type: 'tileFailed', key: j.key, level: j.level, col: j.col, row: j.row, message, quiet: false });
  }
}

/** Starts what the limits let, the oldest asked first. */
function pump(): void {
  for (let i = 0; i < queue.length && running.size < TOTAL; ) {
    const j = queue[i];
    const e = entries.get(j.key);
    if (!e) {
      queue.splice(i, 1);
      continue;
    }
    if (e.state !== 'ready') {
      i++;
      continue;
    }
    const busy = perHostRunning.get(e.host) ?? 0;
    if (busy >= e.perHost) {
      i++;
      continue;
    }
    queue.splice(i, 1);
    const k = tileKey(j.key, j.level, j.col, j.row) + `|${j.step ?? ''}`;
    running.set(k, j);
    perHostRunning.set(e.host, busy + 1);
    void run(j).finally(() => {
      running.delete(k);
      perHostRunning.set(e.host, (perHostRunning.get(e.host) ?? 1) - 1);
      pump();
    });
  }
}

async function credit(key: string, zoom: number, south: number, west: number, north: number, east: number): Promise<void> {
  const e = entries.get(key);
  if (!e?.session) return;
  const m = await loadServices();
  try {
    const text = await serviceText(m, getWire(e.session.viewportUrl(zoom, south, west, north, east)), proofOf(e), opts(e));
    post({ type: 'credit', key, text: m.googleCopyright(text) });
  } catch {
    post({ type: 'credit', key, text: 'Google Maps' });
  }
}

self.onmessage = (ev: MessageEvent<ToWorker>) => {
  const msg = ev.data;
  switch (msg.type) {
    case 'register': {
      const old = entries.get(msg.key);
      if (old && old.state !== 'failed' && JSON.stringify(old.reg) === JSON.stringify(msg)) return;
      const e: Entry = {
        reg: msg,
        serviceJson: JSON.stringify(msg.service),
        state: 'resolving',
        template: null,
        style: null,
        view: null,
        session: null,
        tileW: 256,
        tileH: 256,
        vector: false,
        perHost: perHost(msg.service),
        host: '',
      };
      entries.set(msg.key, e);
      proxyOn = msg.proxy;
      resolve(e).catch((err: unknown) => {
        e.state = 'failed';
        post({ type: 'failed', key: msg.key, message: err instanceof Error ? err.message : String(err) });
      });
      return;
    }
    case 'keep': {
      for (const k of [...entries.keys()]) if (!msg.keys.includes(k)) entries.delete(k);
      for (let i = queue.length - 1; i >= 0; i--) if (!entries.has(queue[i].key)) queue.splice(i, 1);
      for (const j of running.values()) if (!entries.has(j.key)) j.abort?.abort();
      return;
    }
    case 'tile':
      queue.push({ key: msg.key, level: msg.level, col: msg.col, row: msg.row, step: msg.step, abort: null });
      pump();
      return;
    case 'drop': {
      const gone = new Set(msg.tiles.map(([l, c, r]) => tileKey(msg.key, l, c, r)));
      for (let i = queue.length - 1; i >= 0; i--) {
        const j = queue[i];
        if (gone.has(tileKey(j.key, j.level, j.col, j.row))) queue.splice(i, 1);
      }
      for (const j of running.values()) if (gone.has(tileKey(j.key, j.level, j.col, j.row))) j.abort?.abort();
      return;
    }
    case 'place': {
      const json = labels ? labels.place(new Uint32Array(msg.ids), msg.x0, msg.y0, msg.pxPerUnit, msg.width, msg.height) : '[]';
      post({ type: 'placed', seq: msg.seq, json });
      return;
    }
    case 'unlabel':
      for (const id of msg.ids) labels?.remove(id);
      return;
    case 'credit':
      void credit(msg.key, msg.zoom, msg.south, msg.west, msg.north, msg.east);
      return;
    case 'proxy':
      proxyOn = msg.on;
      return;
    case 'forget': {
      entries.delete(msg.key);
      for (let i = queue.length - 1; i >= 0; i--) if (queue[i].key === msg.key) queue.splice(i, 1);
      for (const j of running.values()) if (j.key === msg.key) j.abort?.abort();
      for (const k of [...kept.keys()]) if (k.startsWith(`${msg.key}|`)) kept.delete(k);
      if (msg.fresh) fresh.set(msg.key, new Set());
      return;
    }
  }
};
