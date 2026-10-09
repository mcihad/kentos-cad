import type { ConnectionSecret } from '../contracts/generated/ConnectionSecret';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';
import { crsBySrid } from '../geo/crs';
import { systemOf } from '../model/geom/crsSystem';
import { loadServices, type ServicesModule } from '../io/services/module';
import type { FromWorker, Register, ToWorker } from '../io/services/protocol';
import type { View } from '../io/services/pkg/kentos_services_wasm';
import { SLOT } from './rasterPass';
import type { ServiceGrid, ServicePicture, ServiceSource, ServiceVector } from './servicePass';
import { styledBatches, type BatchOptions } from './styledBatches';

/**
 * The page's map services (docs/adr/0208 §3–§9), twin of the desktop's `services/hub.rs`: what the scene shows, by
 * key (its layer's service and connection, hashed); for each what is known of it (resolving, ready, failed) and its
 * tiles in memory. The drawing passes ask for tiles each frame (`tile`, `vector`); one not here is asked of the worker
 * in the order the frame asks (the view's centre first) and given back by a later frame; one not asked for again in
 * two frames is let go (its request dropped). Picture tiles are kept within 64 MB, the least recently drawn let go
 * first; vector tiles are built by the worker at a quarter zoom step and decoded here with the view's colours. Each
 * arrival tells the views (a frame is drawn). The labels of the vector tiles in view are placed by the worker for the
 * view's screen; the credits of the services shown come with their resolution (Google's for the view once it rests).
 */

export interface Credit {
  text: string;
  links: [string, string][];
}

/** A service as the scene shows it. */
export interface ServiceShown {
  key: string;
  service: ServiceLayer;
  connection: ServiceConnection | null;
  /** The layer's name, for what is said. */
  name: string;
}

/** What the services are drawn in: the project's system and the drawing's anchor. */
export interface ServiceProject {
  /** The project's system in the core's JSON; null for a project without one. */
  ours: string | null;
  choices: string;
  srid: number;
  custom: boolean;
  origin: [number, number];
}

/** A label as the worker placed it (crates/wasm/services-wasm `place`). */
export interface PlacedLabel {
  lines: { text: string; x: number; y: number }[];
  glyphs: { ch: string; x: number; y: number; angle: number }[];
  size: number;
  color: [number, number, number, number];
  halo?: [number, number, number, number, number];
  bold: boolean;
  italic: boolean;
}

/** Bytes the picture tiles in memory may take. */
const PICTURE_BUDGET = 64 * 1024 * 1024;
/** Vector tiles kept decoded. */
const VECTORS_KEPT = 512;
/** A tile that failed this many times shows nothing until the drawing asks again later. */
const TRIES = 3;

interface Entry {
  shown: ServiceShown;
  reg: Register;
  state: 'resolving' | 'ready' | 'failed';
  failure: string | null;
  view: View | null;
  grid: ServiceGrid | null;
  credit: Credit | null;
  vector: boolean;
  notes: string[];
}

interface Picture {
  pic: ServicePicture | 'empty';
  bytes: number;
}

interface Built {
  vec: ServiceVector;
  step: number;
  label: number;
}

const tileId = (key: string, level: number, col: number, row: number) => `${key}|${level}|${col}|${row}`;

/** A service's key: its layer's service and its connection (without secrets), hashed (the desktop's `key_of`). */
export function serviceKey(service: ServiceLayer, connection: ServiceConnection | null): string {
  const text = JSON.stringify(service) + '|' + JSON.stringify(connection);
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193;
  for (let i = 0; i < text.length; i++) {
    const c = text.charCodeAt(i);
    h1 = Math.imul(h1 ^ c, 16777619);
    h2 = Math.imul(h2 ^ c, 2246822519);
  }
  return `service:${(h1 >>> 0).toString(16).padStart(8, '0')}${(h2 >>> 0).toString(16).padStart(8, '0')}`;
}

export class ServiceHub implements ServiceSource {
  private worker: Worker | null = null;
  private module: ServicesModule | null = null;
  private readonly entries = new Map<string, Entry>();
  private readonly pictures = new Map<string, Picture>();
  private pictureBytes = 0;
  /** Vector tiles decoded, by tile and step. */
  private readonly built = new Map<string, Built>();
  /** Tiles asked of the worker, and the frame they were last asked in. */
  private readonly pending = new Map<string, { key: string; level: number; col: number; row: number; wanted: number }>();
  private readonly failed = new Map<string, { at: number; tries: number }>();
  private generation = 0;
  private readonly listeners = new Set<() => void>();
  /** What the worker said went wrong, each once (the log hears it). */
  private readonly said = new Set<string>();
  private noticeSink: ((text: string) => void) | null = null;
  private secrets: (c: ServiceConnection) => ConnectionSecret | null = () => null;
  private proxy = false;
  private hidpi = false;
  private look: BatchOptions | null = null;
  private lookKey = '';
  /** The labels placed last, and what they were placed for. */
  private placed: PlacedLabel[] = [];
  private placedFor = '';
  private placeSeq = 0;
  /** Label ids of the vector tiles drawn this frame. */
  private frameLabels: number[] = [];
  /** Google's credits by service, for the view last rested. */
  private readonly googleCredits = new Map<string, string>();
  private creditTimer: ReturnType<typeof setTimeout> | null = null;
  private creditFor = '';

  /** Told when tiles arrive or a service changes (a view redraws). */
  listen(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private changed(): void {
    for (const fn of this.listeners) fn();
  }

  /** Where a failed service's words go (the log), each once. */
  onNotice(sink: ((text: string) => void) | null): void {
    this.noticeSink = sink;
  }

  /** This device's secrets for the connections (app/connectionSecrets.ts). */
  useSecrets(get: (c: ServiceConnection) => ConnectionSecret | null): void {
    this.secrets = get;
  }

  /** Whether the proxy may be used (signed in to KentOS). */
  setProxy(on: boolean): void {
    if (this.proxy === on) return;
    this.proxy = on;
    this.worker?.postMessage({ type: 'proxy', on } satisfies ToWorker);
  }

  setHidpi(on: boolean): void {
    this.hidpi = on;
  }

  /** The colours vector tiles are decoded with: a new look decodes them again. */
  setLook(look: BatchOptions, key: string): void {
    if (key === this.lookKey) return;
    this.look = look;
    this.lookKey = key;
    this.built.clear();
  }

  private post(m: ToWorker): void {
    if (!this.worker) {
      this.worker = new Worker(new URL('../io/services/worker.ts', import.meta.url), { type: 'module', name: 'KentOS harita servisleri' });
      this.worker.onmessage = (e: MessageEvent<FromWorker>) => this.heard(e.data);
    }
    this.worker.postMessage(m);
  }

  /** The services the scene shows now, bottom first: new ones resolved, the others let go. */
  show(list: readonly ServiceShown[], project: ServiceProject): void {
    const keys = new Set(list.map((s) => s.key));
    for (const s of list) {
      const secret = s.connection ? this.secrets(s.connection) : null;
      const reg: Register = {
        type: 'register',
        key: s.key,
        service: s.service,
        connection: s.connection,
        secret,
        ours: project.ours,
        choices: project.choices,
        projectSrid: project.srid,
        custom: project.custom,
        origin: project.origin,
        proxy: this.proxy,
        hidpi: this.hidpi,
      };
      const old = this.entries.get(s.key);
      const again = !old || JSON.stringify(old.reg) !== JSON.stringify(reg);
      if (!again) {
        old.shown = s;
        continue;
      }
      old?.view?.free();
      this.entries.set(s.key, { shown: s, reg, state: 'resolving', failure: null, view: null, grid: null, credit: null, vector: false, notes: [] });
      this.post(reg);
    }
    let gone = false;
    for (const [k, e] of this.entries)
      if (!keys.has(k)) {
        e.view?.free();
        this.entries.delete(k);
        gone = true;
      }
    if (gone) {
      this.post({ type: 'keep', keys: [...keys] });
      for (const k of [...this.pictures.keys()]) if (!keys.has(k.slice(0, k.indexOf('|')))) this.dropPicture(k);
      for (const k of [...this.built.keys()]) if (!keys.has(k.slice(0, k.indexOf('|')))) this.dropBuilt(k);
    }
  }

  /** Whether nothing is on its way: no service resolving, no tile asked and not come (pictures and tests). */
  idle(): boolean {
    for (const e of this.entries.values()) if (e.state === 'resolving') return false;
    return this.pending.size === 0;
  }

  /** Whether the scene has shown a service (the views then listen for tiles). */
  get inUse(): boolean {
    return this.entries.size > 0;
  }

  /** Why a service shows nothing, when it failed. */
  failure(key: string): string | null {
    return this.entries.get(key)?.failure ?? null;
  }

  /** A service's credits, Google's for the view. */
  credit(key: string): Credit | null {
    const e = this.entries.get(key);
    if (!e?.credit?.text) return null;
    const g = this.googleCredits.get(key);
    return g ? { text: g, links: [["Google Haritalar'ın koşulları", 'https://www.google.com/intl/tr_tr/help/terms_maps/']] } : e.credit;
  }

  private async heard(m: FromWorker): Promise<void> {
    switch (m.type) {
      case 'ready': {
        const e = this.entries.get(m.key);
        if (!e) return;
        const mod = (this.module ??= await loadServices());
        const theirs = (() => {
          const crs = crsBySrid(m.srid);
          const s = crs ? systemOf(crs) : null;
          return s ? JSON.stringify(s) : undefined;
        })();
        const same = m.srid === e.reg.projectSrid && !e.reg.custom;
        try {
          const view = new mod.View(JSON.stringify(e.reg.service), m.template ?? undefined, e.reg.ours ?? undefined, theirs, e.reg.choices, same, m.tileSize);
          e.view = view;
          e.grid = {
            tiles: (x1, y1, x2, y2, px) => view.tiles(x1, y1, x2, y2, px),
            mesh: (l, c, r) => view.mesh(l, c, r),
            cells: (l) => view.cells(l),
            displayZoom: (u) => view.displayZoom(u),
            across: view.across(),
            down: view.down(),
            quadtree: view.quadtree(),
            vector: view.vector(),
          };
          e.state = 'ready';
          e.credit = m.credit;
          e.vector = m.vector;
          e.notes = m.notes;
        } catch (err) {
          this.fail(e, err instanceof Error ? err.message : String(err));
        }
        this.changed();
        return;
      }
      case 'failed': {
        const e = this.entries.get(m.key);
        if (e) this.fail(e, m.message);
        this.changed();
        return;
      }
      case 'picture': {
        const id = tileId(m.key, m.level, m.col, m.row);
        this.pending.delete(id);
        if (!this.entries.has(m.key)) return;
        this.failed.delete(id);
        const pic: ServicePicture | 'empty' =
          m.bytes === null
            ? 'empty'
            : {
                across: m.across,
                down: m.down,
                slots: Array.from({ length: m.across * m.down }, (_, i) => m.bytes!.subarray(i * SLOT * SLOT * 4, (i + 1) * SLOT * SLOT * 4)),
              };
        this.keepPicture(id, { pic, bytes: m.bytes?.byteLength ?? 0 });
        this.changed();
        return;
      }
      case 'vector': {
        const id = tileId(m.key, m.level, m.col, m.row);
        this.pending.delete(`${id}|${m.step}`);
        this.pending.delete(id);
        if (!this.entries.has(m.key) || !this.look) {
          this.post({ type: 'unlabel', ids: [m.id] });
          return;
        }
        const batches = styledBatches(m.json, m.data, this.look);
        this.keepBuilt(`${id}|${m.step}`, { vec: { id: `${id}|${m.step}|${m.id}`, batches }, step: m.step, label: m.id });
        this.changed();
        return;
      }
      case 'tileFailed': {
        const id = tileId(m.key, m.level, m.col, m.row);
        for (const k of [...this.pending.keys()]) if (k === id || k.startsWith(`${id}|`)) this.pending.delete(k);
        const f = this.failed.get(id);
        this.failed.set(id, { at: performance.now(), tries: (f?.tries ?? 0) + 1 });
        const e = this.entries.get(m.key);
        if (e && !m.quiet) this.notice(`“${e.shown.name}”: ${m.message}`);
        this.changed();
        return;
      }
      case 'placed': {
        if (m.seq !== this.placeSeq) return;
        this.placed = JSON.parse(m.json) as PlacedLabel[];
        this.changed();
        return;
      }
      case 'credit': {
        this.googleCredits.set(m.key, m.text);
        this.changed();
        return;
      }
    }
  }

  private fail(e: Entry, message: string): void {
    e.state = 'failed';
    e.failure = message;
    this.notice(`“${e.shown.name}”: ${message}`);
  }

  private notice(text: string): void {
    if (this.said.has(text)) return;
    this.said.add(text);
    this.noticeSink?.(text);
  }

  private keepPicture(id: string, p: Picture): void {
    this.dropPicture(id);
    this.pictures.set(id, p);
    this.pictureBytes += p.bytes;
    for (const [k, v] of this.pictures) {
      if (this.pictureBytes <= PICTURE_BUDGET) break;
      this.pictures.delete(k);
      this.pictureBytes -= v.bytes;
    }
  }

  private dropPicture(id: string): void {
    const old = this.pictures.get(id);
    if (!old) return;
    this.pictures.delete(id);
    this.pictureBytes -= old.bytes;
  }

  private keepBuilt(id: string, b: Built): void {
    this.dropBuilt(id);
    this.built.set(id, b);
    if (this.built.size > VECTORS_KEPT) {
      const first = this.built.keys().next().value!;
      this.dropBuilt(first);
    }
  }

  private dropBuilt(id: string): void {
    const old = this.built.get(id);
    if (!old) return;
    this.built.delete(id);
    this.post({ type: 'unlabel', ids: [old.label] });
  }

  // ── ServiceSource ────────────────────────────────────────────────────

  frame(): void {
    this.generation++;
    const drop = new Map<string, [number, number, number][]>();
    for (const [k, p] of this.pending)
      if (p.wanted + 2 < this.generation) {
        this.pending.delete(k);
        const list = drop.get(p.key) ?? [];
        list.push([p.level, p.col, p.row]);
        drop.set(p.key, list);
      }
    for (const [key, tiles] of drop) this.post({ type: 'drop', key, tiles });
    this.frameLabels = [];
  }

  grid(key: string): ServiceGrid | null {
    const e = this.entries.get(key);
    return e?.state === 'ready' ? e.grid : null;
  }

  private failing(id: string): boolean {
    const f = this.failed.get(id);
    return !!f && (f.tries >= TRIES || performance.now() - f.at < 2000 * f.tries);
  }

  tile(key: string, level: number, col: number, row: number): ServicePicture | 'empty' | null {
    const id = tileId(key, level, col, row);
    const kept = this.pictures.get(id);
    if (kept) {
      // Most recently used last.
      this.pictures.delete(id);
      this.pictures.set(id, kept);
      return kept.pic;
    }
    if (this.failing(id)) return 'empty';
    const p = this.pending.get(id);
    if (p) p.wanted = this.generation;
    else {
      this.pending.set(id, { key, level, col, row, wanted: this.generation });
      this.post({ type: 'tile', key, level, col, row, step: null });
    }
    return null;
  }

  vector(key: string, level: number, col: number, row: number, zoom: number): ServiceVector | null {
    if (!this.look) return null;
    const id = tileId(key, level, col, row);
    const step = Math.round(zoom * 4);
    const exact = this.built.get(`${id}|${step}`);
    // The nearest step built, shown while this one is built.
    let near: Built | null = exact ?? null;
    if (!exact)
      for (const [k, b] of this.built)
        if (k.startsWith(`${id}|`) && (!near || Math.abs(b.step - step) < Math.abs(near.step - step))) near = b;
    if (!exact && !this.failing(id)) {
      const pk = `${id}|${step}`;
      const p = this.pending.get(pk);
      if (p) p.wanted = this.generation;
      else {
        this.pending.set(pk, { key, level, col, row, wanted: this.generation });
        this.post({ type: 'tile', key, level, col, row, step });
      }
    }
    if (near) this.frameLabels.push(near.label);
    return near?.vec ?? null;
  }

  // ── Labels and credits ───────────────────────────────────────────────

  /**
   * The labels for the view whose top left is the project's (`x0`, `y0`), `pxPerUnit` CSS pixels a unit, `width` ×
   * `height` CSS pixels: the last placed; placed again (an answer later) when the view or the tiles drawn changed.
   */
  labels(x0: number, y0: number, pxPerUnit: number, width: number, height: number): readonly PlacedLabel[] {
    const ids = [...new Set(this.frameLabels)].sort((a, b) => a - b);
    if (!ids.length) {
      this.placed = [];
      this.placedFor = '';
      return this.placed;
    }
    const want = `${x0}|${y0}|${pxPerUnit}|${width}|${height}|${ids.join(',')}`;
    if (want !== this.placedFor) {
      this.placedFor = want;
      this.placeSeq++;
      this.post({ type: 'place', seq: this.placeSeq, ids, x0, y0, pxPerUnit, width, height });
    }
    return this.placed;
  }

  /** The credits of the services shown, top first: Google's asked for the view once it rests 400 ms. */
  credits(keys: readonly string[], googleView: ((key: string) => { zoom: number; south: number; west: number; north: number; east: number } | null) | null): Credit[] {
    const out: Credit[] = [];
    for (const k of keys) {
      const c = this.credit(k);
      if (c) out.push(c);
    }
    if (googleView) {
      const google = keys.filter((k) => this.entries.get(k)?.reg.service.kind === 'google' && this.entries.get(k)?.state === 'ready');
      const views = google.map((k) => [k, googleView(k)] as const).filter((v): v is readonly [string, NonNullable<ReturnType<typeof googleView>>] => !!v[1]);
      const want = JSON.stringify(views.map(([k, v]) => [k, v.zoom, ...[v.south, v.west, v.north, v.east].map((x) => Math.round(x * 1e4))]));
      if (views.length && want !== this.creditFor) {
        this.creditFor = want;
        if (this.creditTimer) clearTimeout(this.creditTimer);
        this.creditTimer = setTimeout(() => {
          for (const [k, v] of views) this.post({ type: 'credit', key: k, ...v });
        }, 400);
      }
    }
    return out;
  }

  /**
   * Yeniden yükle: a service read again from its address; its tiles in memory let go. With `fresh` (Önbelleği temizle)
   * each of its answers is asked once past the browser's cache, which then keeps the new one.
   */
  reload(key: string, fresh = false): void {
    const e = this.entries.get(key);
    if (!e) return;
    this.post({ type: 'forget', key, fresh });
    for (const k of [...this.pictures.keys()]) if (k.startsWith(`${key}|`)) this.dropPicture(k);
    for (const k of [...this.built.keys()]) if (k.startsWith(`${key}|`)) this.dropBuilt(k);
    for (const k of [...this.failed.keys()]) if (k.startsWith(`${key}|`)) this.failed.delete(k);
    e.view?.free();
    this.entries.delete(key);
    this.googleCredits.delete(key);
    this.changed();
  }
}

let shared: ServiceHub | null = null;

/** The page's service hub. */
export function serviceHub(): ServiceHub {
  return (shared ??= new ServiceHub());
}
