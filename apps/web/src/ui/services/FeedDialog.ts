import '../../styles/services.css';
import type { AppContext } from '../../app/context';
import type { Entity } from '../../contracts/generated/Entity';
import type { FeatureFeed } from '../../contracts/generated/FeatureFeed';
import type { FeedKind } from '../../contracts/generated/FeedKind';
import type { ReportItem } from '../../contracts/generated/ReportItem';
import { crsBySrid } from '../../geo/crs';
import { serviceText, type Wire } from '../../io/services/fetch';
import { loadServices, type ServicesModule } from '../../io/services/module';
import type { FeedConnecting } from '../../io/services/pkg/kentos_services_wasm';
import type { TakeAnswer, TakeAsk } from '../../io/services/protocol';
import type { CadDocument } from '../../model/document';
import type { NewEntity } from '../../model/entities';
import type { Bounds } from '../../model/geometry';
import { fold, inferFields } from '../../model/layerFields';
import { crsSettings } from '../../model/projectCrs';
import { AUTH_KIND_LABELS } from '../../model/serviceRules';
import { boxIn, servicePair, toService } from '../../model/serviceSystems';
import { readEntityList } from '../../model/snapshot';
import { layersService } from '../../product/layersService';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';

/** The window's title, which a trace names it by. */
export const FEED_TITLE = 'Servisten veri al';

/**
 * Servisten veri al (docs/adr/0208 §10, §14; the desktop's `services/feed_window.rs`): the kinds on the left (WFS, OGC
 * API Features, ArcGIS REST, GeoJSON adresi), on the right the address, the connection and Bağlan (the services core's
 * `FeedConnecting`), the searchable list of types, the system asked, the area (the view, the drawing's extent, the
 * selection's, everything), a filter, the most objects, the key that matches objects on Yenile and the target layer.
 * Al takes the objects page by page in a worker of its own (io/services/feedWorker.ts; Durdur ends it), moved into the
 * project's system, and writes them as one undo step (“Servisten veri al”): a new layer remembers its feed
 * (`cad.layers.service`'s `addFeed`, its fields from the values) and Yenile takes it again.
 */

export const FEED_KINDS: readonly { kind: FeedKind; label: string; icon: string; example: string }[] = [
  { kind: 'wfs', label: 'WFS', icon: 'serviceFeed', example: 'https://cbs.ornek.gov.tr/geoserver/wfs' },
  { kind: 'ogcFeatures', label: 'OGC API Features', icon: 'serviceExtent', example: 'https://demo.pygeoapi.io/master' },
  { kind: 'arcgis', label: 'ArcGIS REST', icon: 'serviceInfo', example: 'https://services.arcgis.com/…/FeatureServer' },
  { kind: 'geojson', label: 'GeoJSON adresi', icon: 'basemapVector', example: 'https://ornek.org/veri.geojson' },
];

/** Where the objects are asked from. */
export const AREAS = ['Görünüm', 'Çizimin kapsamı', 'Seçimin kapsamı', 'Hepsi'] as const;

/** The most objects taken when the window says nothing (`feed::DEFAULT_MOST`). */
const DEFAULT_MOST = 50_000;
/** At most this many types are listed; the search finds the rest. */
const MOST_LISTED = 400;

/** A type of objects the service offers (`feed::FeedItem`). */
interface FeedItem {
  id: string;
  title: string;
  summary?: string;
  srids: number[];
  wgs84?: [number, number, number, number];
  geometry?: string;
}

interface FeedOffer {
  kind: FeedKind;
  title: string;
  items: FeedItem[];
}

type Said = { kind: 'ok' | 'error' | 'info'; text: string };

/** What a take brought. */
interface Taken {
  feed: FeatureFeed;
  entities: Entity[];
  dropped: number;
  matched: number | null;
  capped: boolean;
  /** What the pages' readers left out, summed. */
  skipped: ReportItem[];
}

/** The time as the feed keeps it: `2026-10-09T12:00:00Z`. */
const stamp = (t: Date) => t.toISOString().replace(/\.\d{3}Z$/, 'Z');

/** What a take asks of the worker for `feed`, or why it cannot go. */
function askFor(ctx: AppContext, feed: FeatureFeed, most: number, geojson: boolean): TakeAsk | { error: string } {
  const pair = servicePair(crsSettings(ctx.doc.settings), feed.srid ?? 4326);
  if ('error' in pair) return pair;
  let area: [number, number, number, number] | null = null;
  if (feed.bbox) {
    area = boxIn(feed.bbox, (p) => toService(pair, p));
    if (!area) return { error: 'İstenen alan servisin sisteminde gösterilemiyor.' };
  }
  const conn = feed.connection ? (ctx.doc.settings.connections.value.find((c) => c.id === feed.connection) ?? null) : null;
  return {
    type: 'take',
    feed,
    area,
    most,
    geojson,
    connection: conn,
    secret: conn ? ctx.secrets.get(conn.origin, conn.id) : null,
    proxy: ctx.cloud.auth.value === 'signedIn',
    move: pair.same ? null : { from: JSON.stringify(pair.theirs), to: JSON.stringify(pair.ours), choices: JSON.stringify(pair.choices) },
  };
}

/**
 * Yenile's objects (the desktop's `replace_objects`): with a key, the old object of the same key keeps its identity,
 * colour, symbol and label and takes the new one's geometry and attributes, the new ones go in and the ones the service
 * no longer gives go; without a key the layer's objects are replaced.
 */
function replaceObjects(doc: CadDocument, layer: string, feed: FeatureFeed, fresh: readonly Entity[], label: string): void {
  const old = doc.byLayer(layer);
  const plain = (e: Entity): NewEntity => {
    const copy: Partial<Entity> = { ...e, layerId: layer };
    delete copy.id;
    return copy as NewEntity;
  };
  const key = feed.key;
  if (!key) {
    doc.remove(old.map((e) => e.id));
    doc.addMany(fresh.map(plain), label);
    return;
  }
  const first = new Map<string, number>();
  fresh.forEach((e, i) => {
    const v = e.attrs[key];
    if (v !== undefined && !first.has(v)) first.set(v, i);
  });
  const kept = new Set<number>();
  const gone: number[] = [];
  for (const e of old) {
    const v = e.attrs[key];
    const i = v === undefined ? undefined : first.get(v);
    if (i === undefined) {
      gone.push(e.id);
      continue;
    }
    const f = fresh[i];
    const next = { ...plain(f), layerId: e.layerId, color: e.color, label: e.label, symbol: e.symbol, lineWeight: e.lineWeight, attrs: { ...f.attrs } } as NewEntity;
    for (const k of ['color', 'label', 'symbol', 'lineWeight'] as const) if ((next as Partial<Entity>)[k] === undefined) delete (next as Partial<Entity>)[k];
    doc.replace(e.id, next, label);
    kept.add(i);
  }
  doc.remove(gone);
  doc.addMany(
    fresh.filter((_, i) => !kept.has(i)).map(plain),
    label,
  );
}

/** Opens Servisten veri al. */
export function openFeedDialog(ctx: AppContext): void {
  feedWindow(ctx, null);
}

/** Yenile: a feed layer's objects taken again. */
export function refreshFeed(ctx: AppContext, layerId: string): void {
  if (!ctx.doc.layers.get(layerId)?.feed) return;
  feedWindow(ctx, layerId);
}

function feedWindow(ctx: AppContext, refresh: string | null): void {
  const refreshed = refresh ? ctx.doc.layers.get(refresh) : undefined;
  let kind = refreshed?.feed ? Math.max(0, FEED_KINDS.findIndex((k) => k.kind === refreshed.feed!.kind)) : 0;
  let url = '';
  let connection: string | null = refreshed?.feed?.connection ?? null;
  let mod: ServicesModule | null = null;
  let connecting: FeedConnecting | null = null;
  let offer: FeedOffer | null = null;
  let busy = false;
  let said: Said | null = null;
  let search = '';
  let item: string | null = null;
  let srid: number | null = null;
  let area = 0;
  let filter = '';
  let most = String(DEFAULT_MOST);
  let key = '';
  let target: string | null = refresh;
  let name = '';
  let worker: Worker | null = null;
  let taken = 0;

  const kinds = h('div', { class: 'svc-kinds', role: 'listbox', 'aria-label': 'Veri türü' });
  const right = h('div', { class: 'svc-right' });
  const summary = h('div', { class: 'io-summary' });
  const take = h('button', { class: 'btn btn--primary', type: 'button' }, 'Al') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;
  const project = ctx.doc.settings.crs.value.srid;
  let itemsBox: HTMLElement | null = null;

  const items = (): FeedItem[] => offer?.items ?? [];
  /** The systems listed for the type chosen (the core's `systems`), those KentOS knows. */
  const sridsOf = (): number[] => {
    if (!connecting || item === null || !mod) return [];
    const m = mod;
    return [...connecting.systems(item, project)].filter((s) => m.knownSrid(s));
  };
  /** The system the objects will be asked in: what the list shows chosen. */
  const askedOf = (): number | null => (connecting && item !== null ? (connecting.askedSrid(item, srid ?? undefined, project) ?? null) : null);
  const forget = () => {
    connecting?.free();
    connecting = null;
    offer = null;
    item = null;
  };
  const stop = () => {
    worker?.terminate();
    worker = null;
  };

  function renderKinds(): void {
    replaceChildren(
      kinds,
      ...FEED_KINDS.map((k, i) => {
        const b = h(
          'button',
          { class: `svc-kind${i === kind ? ' is-selected' : ''}`, type: 'button', role: 'option', 'aria-selected': String(i === kind), disabled: !!refresh && i !== kind },
          icon(k.icon, 16),
          h('span', null, k.label),
        );
        b.addEventListener('click', () => {
          if (i === kind || refresh) return;
          kind = i;
          forget();
          said = null;
          url = '';
          render();
        });
        return b;
      }),
    );
  }

  const field = (label: string, control: HTMLElement) => h('div', { class: 'conn-row' }, h('span', { class: 'conn-row__label' }, label), control);
  const text = (value: string, placeholder: string, on: (v: string) => void, label: string) => {
    const el = h('input', { class: 'field', type: 'text', value, placeholder, 'aria-label': label, spellcheck: 'false', autocomplete: 'off' }) as HTMLInputElement;
    el.addEventListener('input', () => on(el.value));
    return el;
  };
  const choose = (labels: readonly string[], current: number, set: (i: number) => void, label: string) => {
    const sel = h('select', { class: 'field', 'aria-label': label }, labels.map((t, i) => h('option', { value: String(i), selected: i === current }, t))) as HTMLSelectElement;
    sel.addEventListener('change', () => set(Number(sel.value)));
    return sel;
  };

  function renderItems(): void {
    if (!itemsBox) return;
    const needle = fold(search.trim());
    const rows: HTMLElement[] = [];
    for (const it of items()) {
      if (needle && !fold(it.title).includes(needle) && !fold(it.id).includes(needle)) continue;
      if (rows.length >= MOST_LISTED) break;
      const on = it.id === item;
      const b = h(
        'button',
        { class: `svc-item${on ? ' is-selected' : ''}`, type: 'button', role: 'option', 'aria-selected': String(on), title: it.summary ?? null },
        h('span', null, it.title || it.id),
        it.id && it.id !== it.title ? h('span', { class: 'svc-item__id' }, it.id) : null,
        it.geometry ? h('span', { class: 'svc-item__id' }, it.geometry.replace(/^esriGeometry/, '')) : null,
      );
      b.addEventListener('click', () => {
        item = it.id;
        srid = null;
        name = it.title;
        const top = itemsBox?.scrollTop ?? 0;
        renderRight();
        if (itemsBox) itemsBox.scrollTop = top;
        renderSummary();
      });
      rows.push(b);
    }
    replaceChildren(itemsBox, ...(rows.length ? rows : [h('p', { class: 'svc-empty' }, needle ? 'Aranan tür yok.' : 'Servis tür vermedi.')]));
  }

  function form(): HTMLElement {
    const k = FEED_KINDS[kind].kind;
    const parts: HTMLElement[] = [];
    if (refresh) {
      const f = refreshed?.feed;
      parts.push(
        h('p', { class: 'io-field__hint' }, `“${refreshed?.name ?? ''}” katmanının kaynağı yeniden alınıyor: ${f?.url ?? ''}${f?.name ? ` (${f.name})` : ''}.`),
        h('p', { class: 'io-field__hint' }, f?.key ? `Nesneler “${f.key}” alanıyla eşlenir: eşleşenler kimliklerini korur.` : 'Anahtar alan yok: katmanın nesneleri yenileriyle değişir.'),
      );
      if (worker) parts.push(h('div', { class: 'feed-progress' }, h('progress', { 'aria-label': 'Alınan nesneler' }), h('span', null, `${taken} nesne`)));
      return h('div', { class: 'svc-form' }, parts);
    }
    const go = h('button', { class: 'btn btn--primary', type: 'button', disabled: busy || !url.trim() || !!worker }, 'Bağlan') as HTMLButtonElement;
    const address = text(
      url,
      FEED_KINDS[kind].example,
      (v) => {
        url = v;
        go.disabled = busy || !url.trim() || !!worker;
        if (connecting) {
          forget();
          renderRight();
          renderSummary();
          const again = right.querySelector<HTMLInputElement>('input[aria-label="Adres"]');
          again?.focus();
          again?.setSelectionRange(again.value.length, again.value.length);
        }
      },
      'Adres',
    );
    address.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && url.trim() && !busy) void runConnect();
    });
    go.addEventListener('click', () => void runConnect());
    parts.push(field('Adres', h('div', { class: 'svc-address' }, address, go)));
    const list = ctx.doc.settings.connections.value;
    const sel = h(
      'select',
      { class: 'field', 'aria-label': 'Bağlantı' },
      h('option', { value: '', selected: connection === null }, 'Yok (kimlik istemez)'),
      list.map((c) => h('option', { value: c.id, selected: c.id === connection }, `${c.name} (${AUTH_KIND_LABELS[c.auth]})`)),
    ) as HTMLSelectElement;
    sel.addEventListener('change', () => (connection = sel.value || null));
    parts.push(field('Bağlantı', sel));
    if (offer) {
      const find = text(
        search,
        'Tür ara',
        (v) => {
          search = v;
          renderItems();
        },
        'Tür ara',
      );
      itemsBox = h('div', { class: 'svc-items', role: 'listbox', 'aria-label': 'Servisin türleri' });
      parts.push(find, itemsBox);
      const srids = sridsOf();
      if (srids.length) {
        const current = askedOf();
        const labels = srids.map((s) => {
          const c = crsBySrid(s);
          const name = c ? `${c.name} (EPSG:${s})` : `EPSG:${s}`;
          return s === project ? `${name} — projenin sistemi` : name;
        });
        parts.push(field('İstenen sistem', choose(labels, current === null ? -1 : srids.indexOf(current), (i) => (srid = srids[i] ?? null), 'İstenen sistem')));
      }
      parts.push(
        field('Alan', choose(AREAS, area, (i) => (area = i), 'Alan')),
        field('Süzgeç', text(filter, k === 'arcgis' ? 'ArcGIS where: ADA = 104' : "CQL: ada = '104'", (v) => (filter = v), 'Süzgeç')),
        field('En çok nesne', text(most, String(DEFAULT_MOST), (v) => (most = v), 'En çok nesne')),
        field('Anahtar alan', text(key, "Yenile'de nesneleri eşler (isteğe bağlı)", (v) => (key = v), 'Anahtar alan')),
      );
      const layers = ctx.doc.layers.leaves().filter((l) => !l.service);
      const targets = ['Yeni katman (kaynağını hatırlar)', ...layers.map((l) => ctx.doc.layers.path(l.id))];
      const chosen = target === null ? 0 : layers.findIndex((l) => l.id === target) + 1;
      parts.push(
        field(
          'Hedef katman',
          choose(targets, chosen, (i) => {
            target = i === 0 ? null : (layers[i - 1]?.id ?? null);
            renderRight();
          }, 'Hedef katman'),
        ),
      );
      if (target === null) parts.push(field('Katmanın adı', text(name, 'Yeni katmanın adı', (v) => (name = v), 'Katmanın adı')));
      if (worker) parts.push(h('div', { class: 'feed-progress' }, h('progress', { 'aria-label': 'Alınan nesneler' }), h('span', null, `${taken} nesne`)));
    } else itemsBox = null;
    return h('div', { class: 'svc-form' }, parts);
  }

  function renderRight(): void {
    replaceChildren(right, form());
    renderItems();
  }

  function renderSummary(): void {
    const words: Said = worker
      ? { kind: 'info', text: `Nesneler alınıyor… ${taken} nesne.` }
      : (said ?? { kind: 'info', text: "Adresi yazıp Bağlan'a basın: servisin türleri listelenir." });
    replaceChildren(summary, summaryLine(words.kind, words.text));
    take.textContent = worker ? 'Durdur' : 'Al';
    take.disabled = !worker && (busy || !connecting || item === null);
    // Yenile has nothing to choose: only Durdur while it takes.
    take.hidden = !!refresh && !worker;
  }

  function render(): void {
    renderKinds();
    renderRight();
    renderSummary();
  }

  async function runConnect(): Promise<void> {
    if (busy || worker) return;
    const k = FEED_KINDS[kind].kind;
    busy = true;
    said = { kind: 'info', text: 'Servis okunuyor…' };
    forget();
    render();
    let c: FeedConnecting | null = null;
    try {
      mod = await loadServices();
      c = new mod.FeedConnecting(k, url.trim());
      const conn = connection ? (ctx.doc.settings.connections.value.find((x) => x.id === connection) ?? null) : null;
      const proof = conn ? { connection: conn, secret: ctx.secrets.get(conn.origin, conn.id) } : null;
      let next = c.first() ?? null;
      for (let steps = 1; next; steps++) {
        if (steps > 8) throw new Error('Servis okunurken çok fazla istek gerekti; adresi denetleyin.');
        const body = await serviceText(mod, JSON.parse(next) as Wire, proof, { proxy: ctx.cloud.auth.value === 'signedIn', referer: url.trim() });
        next = c.answer(body) ?? null;
      }
      connecting = c;
      c = null;
      offer = JSON.parse(connecting.offer() ?? 'null') as FeedOffer | null;
      if (offer) {
        said = { kind: 'ok', text: `${offer.title || 'Servis okundu'}: ${offer.items.length} tür.` };
        if (offer.items.length === 1) {
          item = offer.items[0].id;
          if (!name) name = offer.items[0].title;
        }
      }
    } catch (e) {
      c?.free();
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
    }
    busy = false;
    if (dialog.el.isConnected) render();
    else forget();
  }

  /** The area chosen, in the project's system. */
  function areaOf(i: number): [number, number, number, number] | null {
    const b: Bounds | null =
      i === 0 ? ctx.view.camera.visibleBounds() : i === 1 ? ctx.doc.bounds() : i === 2 ? ctx.doc.bounds(ctx.selection.ids.value) : null;
    return b ? [b.minX, b.minY, b.maxX, b.maxY] : null;
  }

  /** Al: the feed the choice makes, taken. */
  function startTake(): void {
    if (!connecting) {
      said = { kind: 'error', text: 'Önce Bağlan ile servisi okuyun.' };
      renderSummary();
      return;
    }
    if (area === 2 && !ctx.selection.ids.value.size) {
      said = { kind: 'error', text: 'Seçim yok: önce çizimde bir alan seçin ya da başka bir alan kullanın.' };
      renderSummary();
      return;
    }
    const limit = Number(most.trim());
    const choice = {
      item: item ?? '',
      srid,
      filter: filter.trim() ? filter : null,
      bbox: areaOf(area),
      limit: Number.isInteger(limit) && limit > 0 ? limit : null,
      key: key.trim() ? key.trim() : null,
      connection,
    };
    let feed: FeatureFeed;
    try {
      feed = JSON.parse(connecting.feed(JSON.stringify(choice), project)) as FeatureFeed;
    } catch (e) {
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
      renderSummary();
      return;
    }
    if (target === null && !name.trim()) {
      said = { kind: 'error', text: 'Yeni katmana bir ad verin.' };
      renderSummary();
      return;
    }
    run(feed, connecting.geojson());
  }

  /** Takes `feed`'s objects page by page in the worker. */
  function run(feed: FeatureFeed, geojson: boolean): void {
    const limit = Number(most.trim());
    const ask = askFor(ctx, feed, Number.isInteger(limit) && limit > 0 ? limit : DEFAULT_MOST, geojson);
    if ('error' in ask) {
      said = { kind: 'error', text: ask.error };
      render();
      return;
    }
    stop();
    taken = 0;
    const w = new Worker(new URL('../../io/services/feedWorker.ts', import.meta.url), { type: 'module', name: 'KentOS servisten veri' });
    worker = w;
    w.onmessage = (e: MessageEvent<TakeAnswer>) => {
      if (worker !== w) return;
      const a = e.data;
      if (a.type === 'progress') {
        taken = a.taken;
        renderSummary();
        const count = right.querySelector('.feed-progress span');
        if (count) count.textContent = `${taken} nesne`;
        return;
      }
      stop();
      if (a.type === 'failed') said = { kind: 'error', text: a.message };
      else writeTaken({ feed, entities: a.entities, dropped: a.dropped, matched: a.matched, capped: a.capped, skipped: a.skipped });
      if (dialog.el.isConnected) render();
    };
    w.onerror = (e) => {
      if (worker !== w) return;
      stop();
      said = { kind: 'error', text: `Servisten veri alınamadı: ${e.message || 'işçi başlatılamadı'}.` };
      if (dialog.el.isConnected) render();
    };
    w.postMessage(ask);
    said = null;
    render();
  }

  /** The objects taken written as one undo step. */
  function writeTaken(t: Taken): void {
    const label = refresh ? 'Servisi yenile' : 'Servisten veri al';
    const doc = ctx.doc;
    if (doc.busy) {
      said = { kind: 'error', text: 'Bir işlem aracı ya da model hâlâ çalışıyor. Bitmesini bekleyip Al düğmesine yeniden basın.' };
      return;
    }
    const into = refresh ?? target;
    if (into && doc.layers.isLocked(into)) {
      said = { kind: 'error', text: `“${doc.layers.get(into)?.name ?? into}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin.` };
      return;
    }
    const feed: FeatureFeed = { ...t.feed, fetched: stamp(new Date()) };
    try {
      doc.transact(label, () => {
        let layer: string;
        if (refresh) {
          // Yenile: the layer's objects replaced (by key when given), its feed's time renewed.
          const r = layersService.execute({ doc }, { operation: 'update', layer: refresh, feed });
          if (r.status !== 'completed') throw new Error('error' in r ? r.error.message : 'Kaynak yazılamadı.');
          layer = refresh;
        } else if (target) layer = target;
        else {
          const fields = inferFields(t.entities.map((e) => e.attrs));
          const r = layersService.execute({ doc }, { operation: 'addFeed', name: name.trim(), index: 0, feed, fields });
          if (r.status !== 'completed') throw new Error('error' in r ? r.error.message : 'Katman eklenemedi.');
          layer = r.output.layer;
          // A colour of its own, seen on any basemap.
          if (mod) {
            const [line, fill] = mod.layerColors(doc.layers.leaves().length);
            doc.setLayerStyle(layer, { color: line, fill }, label);
          }
        }
        const valid = new Set(doc.layers.leaves().map((l) => l.id));
        const checked = readEntityList(
          t.entities.map((e, i) => ({ ...e, layerId: layer, id: i + 1 })),
          valid,
          'Servisten alınan nesne',
        );
        if (!checked.ok) throw new Error(`Servisin nesneleri çizime uymuyor (${checked.error}). Hiçbir nesne eklenmedi.`);
        if (refresh) replaceObjects(doc, layer, feed, checked.entities, label);
        else
          doc.addMany(
            checked.entities.map((e) => {
              const copy: Partial<Entity> = { ...e };
              delete copy.id;
              return copy as NewEntity;
            }),
            label,
          );
      });
    } catch (e) {
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
      return;
    }
    const n = t.entities.length;
    const words = mod
      ? mod.takenWords(n, t.matched ?? undefined, JSON.stringify(t.skipped), t.dropped, t.capped, t.feed.srid ?? 4326)
      : `${n} nesne alındı.`;
    // What was left out is a warning: the user may want to take it again otherwise.
    if (t.skipped.length || t.dropped) ctx.log.warn(words);
    else ctx.log.success(words);
    dialog.close();
  }

  take.addEventListener('click', () => {
    if (worker) {
      stop();
      said = { kind: 'info', text: 'Alma durduruldu; çizim değişmedi.' };
      render();
      return;
    }
    startTake();
  });
  cancel.addEventListener('click', () => dialog.close());
  const dialog = new Dialog({
    title: refresh ? 'Servisi yenile' : FEED_TITLE,
    width: 900,
    className: 'dialog--feed',
    content: [h('div', { class: 'svc-panes' }, kinds, right), summary],
    footer: [cancel, take],
    onClose: () => {
      stop();
      forget();
    },
  });
  render();
  if (refresh && refreshed?.feed) {
    const f = refreshed.feed;
    void loadServices()
      .then((m) => {
        mod = m;
        run(f, false);
      })
      .catch((e: unknown) => {
        said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
        render();
      });
  }
}
