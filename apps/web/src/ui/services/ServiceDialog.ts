import '../../styles/services.css';
import type { AppContext } from '../../app/context';
import { addBasemap } from '../../app/services';
import type { ServiceKind } from '../../contracts/generated/ServiceKind';
import type { ServiceLayer } from '../../contracts/generated/ServiceLayer';
import { crsBySrid } from '../../geo/crs';
import { serviceText, type Wire } from '../../io/services/fetch';
import { loadServices, type ServicesModule } from '../../io/services/module';
import type { Connecting } from '../../io/services/pkg/kentos_services_wasm';
import { fold } from '../../model/layerFields';
import { PRESET_GROUPS, PRESETS } from '../../model/servicePresets';
import { AUTH_KIND_LABELS } from '../../model/serviceRules';
import { layersService } from '../../product/layersService';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { summaryLine } from '../io/common';
import { Dialog } from '../widgets/Dialog';
import { openConnections } from './ConnectionsDialog';

/** The window's title, which a trace names it by. */
export const SERVICE_TITLE = 'Harita servisi';

/**
 * Harita servisi (docs/adr/0208 §14; the desktop's `services/window.rs`): the kinds on the left (Hazır altlıklar, XYZ /
 * TMS, WMS, WMTS, OGC API Tiles, Vektör karo, ArcGIS REST, Google), on the right the address, the connection and Bağlan,
 * which reads what the service has (the services core's `Connecting`, every request with the connection's proof), then
 * the searchable list of its layers and what to ask for: style, format, system (the project's when offered), clear
 * ground, one picture of the view, the levels; the layer's name and opacity. Ekle writes it by `cad.layers.service` as
 * one undo step: an opaque service goes to the bottom, a clear one over the basemaps. Opened from a service layer's
 * menu (Servis ayarları) it changes that layer.
 */

/** The kinds on the left: none is Hazır altlıklar; each its icon and an address to show the form. */
export const KINDS: readonly { kind: ServiceKind | null; label: string; icon: string; example: string }[] = [
  { kind: null, label: 'Hazır altlıklar', icon: 'basemap', example: '' },
  { kind: 'xyz', label: 'XYZ / TMS', icon: 'basemapOsm', example: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png' },
  { kind: 'wms', label: 'WMS', icon: 'serviceAdd', example: 'https://cbs.ornek.gov.tr/geoserver/wms' },
  { kind: 'wmts', label: 'WMTS', icon: 'serviceSettings', example: 'https://atlas.harita.gov.tr/wmts/1.0.0/WMTSCapabilities.xml' },
  { kind: 'ogcTiles', label: 'OGC API Tiles', icon: 'serviceExtent', example: 'https://maps.ornek.org/ogcapi/collections/orto/map/tiles' },
  { kind: 'vector', label: 'Vektör karo', icon: 'basemapVector', example: 'https://tiles.openfreemap.org/styles/liberty' },
  { kind: 'arcgis', label: 'ArcGIS REST', icon: 'serviceInfo', example: 'https://services.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer' },
  { kind: 'google', label: 'Google', icon: 'basemapGoogleRoad', example: '' },
];

const OPACITIES = [1, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4];

/** At most this many of a service's layers are listed; the search finds the rest. */
const MOST_LISTED = 400;

/** A layer the service offers (`connect::Item`). */
interface Item {
  id: string;
  title: string;
  depth: number;
  summary?: string;
  pickable: boolean;
  queryable: boolean;
  styles: { id: string; title: string }[];
  formats: string[];
  srids: number[];
  wgs84?: [number, number, number, number];
  vector: boolean;
}

/** What Bağlan read (`connect::Offer`). */
interface Offer {
  kind: ServiceKind;
  title: string;
  items: Item[];
  formats: string[];
  attribution?: string;
  pickable: number;
}

type Said = { kind: 'ok' | 'error' | 'info'; text: string };

/** Bağlan's requests in turn, each with the connection's proof (the desktop's `connect_now`). */
async function connect(m: ServicesModule, ctx: AppContext, kind: ServiceKind, url: string, connectionId: string | null): Promise<Connecting> {
  const c = new m.Connecting(kind, url);
  try {
    const conn = connectionId ? (ctx.doc.settings.connections.value.find((x) => x.id === connectionId) ?? null) : null;
    const proof = conn ? { connection: conn, secret: ctx.secrets.get(conn.origin, conn.id) } : null;
    let next = c.first() ?? null;
    for (let steps = 1; next; steps++) {
      if (steps > 16) throw new Error('Servis okunurken çok fazla istek gerekti; adresi denetleyin.');
      const body = await serviceText(m, JSON.parse(next) as Wire, proof, { proxy: ctx.cloud.auth.value === 'signedIn', referer: url });
      next = c.answer(body) ?? null;
    }
    return c;
  } catch (e) {
    c.free();
    throw e;
  }
}

/** Where a new service goes in the top of the tree: an opaque one under everything, a clear one over the services at the bottom. */
export function placeFor(ctx: AppContext, clear: boolean): number | undefined {
  if (!clear) return undefined;
  const roots = ctx.doc.layers.tree;
  let basemaps = 0;
  for (let i = roots.length - 1; i >= 0 && roots[i].type === 'layer' && roots[i].service; i--) basemaps++;
  return roots.length - basemaps;
}

/** A whole number for a level, or none. */
const levelOf = (t: string): number | null => {
  const z = Number(t.trim());
  return t.trim() && Number.isInteger(z) && z >= 0 && z <= 30 ? z : null;
};

/** Opens Harita servisi; on a layer drawn from a service (Servis ayarları), to change it. */
export function openServiceDialog(ctx: AppContext, layerId?: string): void {
  const node = layerId ? ctx.doc.layers.get(layerId) : undefined;
  const before: ServiceLayer | null = node?.service ? structuredClone(node.service) : null;
  const changing = !!before;
  let kind = before ? Math.max(1, KINDS.findIndex((k) => k.kind === before.kind)) : 0;
  let url = before?.url ?? '';
  let connection: string | null = before?.connection ?? null;
  let mod: ServicesModule | null = null;
  let connecting: Connecting | null = null;
  let offer: Offer | null = null;
  let busy = false;
  let said: Said | null = null;
  let search = '';
  let picked: string[] = [];
  let style: string | null = null;
  let format: string | null = null;
  let srid: number | null = null;
  let opacity = before?.opacity ?? 1;
  let transparent = before?.transparent ?? true;
  let dynamic = before?.dynamic ?? false;
  let name = node?.name ?? '';
  let named = changing;
  let tileSize = before?.tileSize ?? 256;
  let subdomains = (before?.subdomains ?? []).join(',');
  let yFlip = before?.yFlip ?? false;
  let maxZoom = before?.maxZoom !== undefined ? String(before.maxZoom) : '';
  let attribution = before?.attribution ?? '';

  const kinds = h('div', { class: 'svc-kinds', role: 'listbox', 'aria-label': 'Servis türü' });
  const right = h('div', { class: 'svc-right' });
  const summary = h('div', { class: 'io-summary' });
  const add = h('button', { class: 'btn btn--primary', type: 'button' }, changing ? 'Kaydet' : 'Ekle') as HTMLButtonElement;
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç') as HTMLButtonElement;
  const project = ctx.doc.settings.crs.value.srid;
  let itemsBox: HTMLElement | null = null;

  const kindNow = (): ServiceKind | null => KINDS[kind].kind;
  const items = (): Item[] => offer?.items ?? [];
  const pickedItems = (): Item[] => picked.map((id) => items().find((i) => i.id === id)).filter((i): i is Item => !!i);
  /** The systems the picked items are all offered in, those KentOS knows. */
  const commonSrids = (): number[] => {
    const list = pickedItems();
    const m = mod;
    if (!list.length || !m) return [];
    let out = [...list[0].srids];
    for (const i of list.slice(1)) out = out.filter((s) => i.srids.includes(s));
    return out.filter((s) => m.knownSrid(s));
  };
  const formatsOf = (): string[] => (kindNow() === 'wmts' ? (pickedItems()[0]?.formats ?? []) : kindNow() === 'wms' ? (offer?.formats ?? []) : []);

  /** Forgets what Bağlan read. */
  const forget = () => {
    connecting?.free();
    connecting = null;
    offer = null;
    picked = [];
  };

  function renderKinds(): void {
    replaceChildren(
      kinds,
      ...KINDS.map((k, i) => {
        const b = h(
          'button',
          { class: `svc-kind${i === kind ? ' is-selected' : ''}`, type: 'button', role: 'option', 'aria-selected': String(i === kind), disabled: changing && i !== kind },
          icon(k.icon, 16),
          h('span', null, k.label),
        );
        b.addEventListener('click', () => {
          if (i === kind) return;
          kind = i;
          forget();
          said = null;
          url = '';
          style = null;
          format = null;
          srid = null;
          if (!named || !changing) {
            name = '';
            named = false;
          }
          render();
          if (KINDS[i].kind === 'google') void runConnect();
        });
        return b;
      }),
    );
  }

  function presets(): HTMLElement {
    return h(
      'div',
      { class: 'svc-presets' },
      PRESET_GROUPS.map((g) =>
        h(
          'section',
          null,
          h('h3', { class: 'svc-presets__group' }, g.name),
          h(
            'div',
            { class: 'svc-presets__tiles' },
            PRESETS.filter((p) => p.group === g.id).map((p) => {
              const b = h('button', { class: 'svc-preset', type: 'button', title: p.note }, icon(p.icon, 28), h('span', null, p.name));
              b.addEventListener('click', () => {
                dialog.close();
                addBasemap(ctx, p);
              });
              return b;
            }),
          ),
        ),
      ),
    );
  }

  const field = (label: string, control: HTMLElement) => h('div', { class: 'conn-row' }, h('span', { class: 'conn-row__label' }, label), control);
  const text = (value: string, placeholder: string, on: (v: string) => void, label: string) => {
    const el = h('input', { class: 'field', type: 'text', value, placeholder, 'aria-label': label, spellcheck: 'false', autocomplete: 'off' }) as HTMLInputElement;
    el.addEventListener('input', () => on(el.value));
    return el;
  };
  const check = (on: boolean, words: string, set: (v: boolean) => void) => {
    const box = h('input', { type: 'checkbox', checked: on }) as HTMLInputElement;
    box.addEventListener('change', () => set(box.checked));
    return h('label', { class: 'io-check' }, box, words);
  };
  /** A list of `values`; with `placeholder`, its first entry chooses none. */
  const choose = <T,>(values: readonly T[], labelOf: (v: T) => string, current: T | null, set: (v: T | null) => void, label: string, placeholder?: string) => {
    const sel = h(
      'select',
      { class: 'field', 'aria-label': label },
      placeholder ? h('option', { value: '', selected: current === null }, placeholder) : null,
      values.map((v, i) => h('option', { value: String(i), selected: current !== null && v === current }, labelOf(v))),
    ) as HTMLSelectElement;
    sel.addEventListener('change', () => set(sel.value === '' ? null : values[Number(sel.value)]));
    return sel;
  };

  function renderItems(): void {
    if (!itemsBox) return;
    const k = kindNow();
    const many = k === 'wms' || k === 'arcgis';
    const needle = fold(search.trim());
    const rows: HTMLElement[] = [];
    for (const it of items()) {
      if (needle && !fold(it.title).includes(needle) && !fold(it.id).includes(needle)) continue;
      if (rows.length >= MOST_LISTED) break;
      const on = picked.includes(it.id);
      const mark = many
        ? (h('input', { type: 'checkbox', checked: on, disabled: !it.pickable, tabindex: -1, 'aria-hidden': 'true' }) as HTMLInputElement)
        : icon(it.vector ? 'basemapVector' : 'basemap', 14);
      const b = h(
        'button',
        {
          class: `svc-item${on ? ' is-selected' : ''}`,
          type: 'button',
          role: 'option',
          'aria-selected': String(on),
          disabled: !it.pickable,
          title: it.summary ?? null,
          style: `padding-left: ${8 + 14 * it.depth}px`,
        },
        mark,
        h('span', null, it.title || it.id),
        it.id && it.id !== it.title && k !== 'ogcTiles' ? h('span', { class: 'svc-item__id' }, it.id) : null,
        it.queryable && k === 'wms' ? h('span', { class: 'svc-item__id' }, 'sorgulanır') : null,
      );
      b.addEventListener('click', () => pick(it));
      rows.push(b);
    }
    replaceChildren(itemsBox, ...(rows.length ? rows : [h('p', { class: 'svc-empty' }, needle ? 'Aranan katman yok.' : 'Servis katman vermedi.')]));
  }

  function pick(it: Item): void {
    const k = kindNow();
    const many = k === 'wms' || (k === 'arcgis' && it.id !== '');
    if (many) {
      // The whole service and its layers do not go together.
      picked = picked.includes(it.id) ? picked.filter((p) => p !== it.id) : [...picked.filter((p) => p !== ''), it.id];
    } else picked = [it.id];
    style = null;
    format = null;
    if (!named) name = it.title;
    const top = itemsBox?.scrollTop ?? 0;
    renderRight();
    if (itemsBox) itemsBox.scrollTop = top;
    renderSummary();
  }

  function form(): HTMLElement {
    const k = kindNow()!;
    const parts: HTMLElement[] = [];
    if (k !== 'google') {
      const go = h('button', { class: 'btn btn--primary', type: 'button', disabled: busy || !url.trim() }, 'Bağlan') as HTMLButtonElement;
      const address = text(
        url,
        KINDS[kind].example,
        (v) => {
          url = v;
          go.disabled = busy || !url.trim();
          if (connecting) {
            // What Bağlan read was for the address as it was.
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
    }
    const list = ctx.doc.settings.connections.value;
    const sel = h(
      'select',
      { class: 'field', 'aria-label': 'Bağlantı' },
      h('option', { value: '', selected: connection === null }, 'Yok (kimlik istemez)'),
      list.map((c) => h('option', { value: c.id, selected: c.id === connection }, `${c.name} (${AUTH_KIND_LABELS[c.auth]})`)),
      h('option', { value: '__new' }, 'Yeni bağlantı…'),
    ) as HTMLSelectElement;
    sel.addEventListener('change', () => {
      if (sel.value !== '__new') {
        connection = sel.value || null;
        return;
      }
      // Yeni bağlantı…: Bağlantılar over this window, a new connection in it.
      openConnections(ctx, {
        add: true,
        done: (added) => {
          if (added) connection = added;
          renderRight();
        },
      });
    });
    parts.push(field('Bağlantı', sel));
    if (k === 'google') {
      const show = h('button', { class: 'btn', type: 'button', disabled: busy }, 'Harita türlerini göster') as HTMLButtonElement;
      show.addEventListener('click', () => void runConnect());
      parts.push(h('div', null, show));
    }
    if (offer) {
      const find = text(
        search,
        'Katman ara',
        (v) => {
          search = v;
          renderItems();
        },
        'Katman ara',
      );
      itemsBox = h('div', { class: 'svc-items', role: 'listbox', 'aria-label': 'Servisin katmanları', 'aria-multiselectable': String(k === 'wms' || k === 'arcgis') });
      parts.push(find, itemsBox, ...options(k));
    } else {
      itemsBox = null;
      if (k === 'xyz' && !changing)
        parts.push(h('p', { class: 'io-field__hint' }, 'Şablonun yer tutucuları: {z}, {x}, {y} ya da {-y} (TMS), {s} (alt alanlar), {r} (yüksek çözünürlük), {quadkey}.'));
    }
    parts.push(
      field(
        'Katmanın adı',
        text(
          name,
          'Katman ağacındaki adı',
          (v) => {
            name = v;
            named = true;
          },
          'Katmanın adı',
        ),
      ),
      field(
        'Saydamlık',
        choose(OPACITIES, (o) => `% ${Math.round(o * 100)}`, OPACITIES.find((o) => Math.abs(o - opacity) < 1e-9) ?? 1, (o) => (opacity = o ?? 1), 'Saydamlık'),
      ),
    );
    return h('div', { class: 'svc-form' }, parts);
  }

  /** What to ask for once a layer is picked, per kind. */
  function options(k: ServiceKind): HTMLElement[] {
    const out: HTMLElement[] = [];
    const list = pickedItems();
    if ((k === 'wms' || k === 'wmts') && list.length === 1 && list[0].styles.length) {
      const styles = list[0].styles;
      out.push(field('Stil', choose(styles, (s) => s.title || s.id, styles.find((s) => s.id === style) ?? styles[0], (s) => (style = s?.id ?? null), 'Stil')));
    }
    const formats = formatsOf();
    if ((k === 'wms' || k === 'wmts') && formats.length) out.push(field('Biçim', choose(formats, (f) => f, format, (f) => (format = f), 'Biçim', 'Önerilen')));
    const srids = commonSrids();
    if ((k === 'wms' || k === 'wmts') && srids.length && mod) {
      const suggested = mod.suggestedSrid(Uint32Array.from(srids), project) ?? null;
      const nameOf = (s: number) => {
        const c = crsBySrid(s);
        const label = c ? `${c.name} (EPSG:${s})` : `EPSG:${s}`;
        return s === suggested ? `${label} — ${s === project ? 'projenin sistemi' : 'önerilen'}` : label;
      };
      const current = srid !== null && srids.includes(srid) ? srid : suggested;
      out.push(field('Sistem', choose(srids, nameOf, current, (s) => (srid = s), 'Sistem')));
    }
    if (k === 'wms') {
      out.push(check(transparent, 'Saydam zemin (altındaki katmanlar görünür)', (v) => (transparent = v)));
      out.push(check(dynamic, 'Görünüm başına tek resim (karosuz)', (v) => (dynamic = v)));
    }
    if (k === 'arcgis') out.push(check(transparent, 'Saydam zemin', (v) => (transparent = v)));
    if (k === 'xyz') {
      out.push(
        field('Karo boyu', choose([256, 512], (t) => `${t} piksel`, tileSize, (t) => (tileSize = t ?? 256), 'Karo boyu')),
        field('Alt alanlar', text(subdomains, 'a,b,c ({s} için)', (v) => (subdomains = v), 'Alt alanlar')),
        check(yFlip, 'TMS: satırlar alttan sayılır', (v) => (yFlip = v)),
        field('En büyük kat', text(maxZoom, '19', (v) => (maxZoom = v), 'En büyük kat')),
        field('Atıf', text(attribution, '© Kaynağın adı', (v) => (attribution = v), 'Atıf')),
      );
    }
    return out;
  }

  function renderRight(): void {
    replaceChildren(right, kindNow() === null ? presets() : form());
    renderItems();
  }

  function renderSummary(): void {
    const k = kindNow();
    const words: Said =
      said ??
      (k === null
        ? { kind: 'info', text: 'Bir altlığa tıklayın: en alttaki hazır altlığın yerini alır ya da en alta eklenir.' }
        : k === 'google'
          ? { kind: 'info', text: "Google'ın karoları kullanıcının API anahtarıyla gelir (Map Tiles API); anahtarı bir Google bağlantısına girin." }
          : { kind: 'info', text: "Adresi yazıp Bağlan'a basın: servisin katmanları listelenir." });
    replaceChildren(summary, summaryLine(words.kind, words.text));
    add.hidden = k === null;
    add.disabled = busy || !((connecting && picked.length) || (!connecting && before));
  }

  function render(): void {
    renderKinds();
    renderRight();
    renderSummary();
  }

  async function runConnect(): Promise<void> {
    const k = kindNow();
    if (!k || busy) return;
    busy = true;
    said = { kind: 'info', text: 'Servis okunuyor…' };
    forget();
    render();
    try {
      mod = await loadServices();
      connecting = await connect(mod, ctx, k, k === 'google' ? '' : url.trim(), connection);
      offer = JSON.parse(connecting.offer() ?? 'null') as Offer | null;
      if (offer) {
        said =
          k === 'google' && !connection
            ? { kind: 'info', text: 'Bir harita türü seçin; anahtar bir Google bağlantısında olmalı.' }
            : { kind: 'ok', text: `${offer.title || 'Servis okundu'}: ${offer.pickable} katman.` };
        if (!named || !name) name = offer.title;
        // One to pick: picked.
        const pickable = offer.items.filter((i) => i.pickable);
        if (pickable.length === 1) picked = [pickable[0].id];
      }
    } catch (e) {
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
    }
    busy = false;
    if (dialog.el.isConnected) render();
    else forget();
  }

  /** Ekle (or Kaydet on a service layer): the service layer the choice makes, as one undo step. */
  function write(): void {
    const n = name.trim();
    if (!n) {
      said = { kind: 'error', text: 'Katmana bir ad verin.' };
      renderSummary();
      return;
    }
    let service: ServiceLayer;
    try {
      if (connecting) {
        const choice = {
          items: picked,
          style,
          format,
          srid,
          transparent,
          dynamic,
          opacity,
          connection,
          mapType: null,
          minZoom: null,
          maxZoom: levelOf(maxZoom),
          tileSize,
          subdomains: subdomains.split(/[,\s]+/).filter(Boolean),
          yFlip,
          attribution: attribution.trim() || null,
        };
        service = JSON.parse(connecting.layer(JSON.stringify(choice), project)) as ServiceLayer;
      } else if (before) {
        // Servis ayarları without Bağlan: the general fields only.
        service = structuredClone(before);
        if (opacity < 1) service.opacity = opacity;
        else delete service.opacity;
        if (attribution.trim()) service.attribution = attribution.trim();
        else delete service.attribution;
        const z = levelOf(maxZoom);
        if (z !== null) service.maxZoom = z;
        if (connection) service.connection = connection;
        else delete service.connection;
      } else {
        said = { kind: 'error', text: 'Önce Bağlan ile servisi okuyun.' };
        renderSummary();
        return;
      }
    } catch (e) {
      said = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
      renderSummary();
      return;
    }
    const clear = !!service.transparent || service.opacity !== undefined;
    const index = changing ? undefined : placeFor(ctx, clear);
    const result = layersService.execute(
      { doc: ctx.doc },
      changing && layerId ? { operation: 'update', layer: layerId, name: n, service } : { operation: 'add', name: n, service, ...(index !== undefined && { index }) },
    );
    if (result.status !== 'completed') {
      said = { kind: 'error', text: 'error' in result ? result.error.message : 'Servis katmanı yazılamadı.' };
      renderSummary();
      return;
    }
    for (const w of result.warnings) ctx.log.warn(w.message);
    ctx.log.success(changing ? `“${n}” servis katmanı değişti.` : `“${n}” servis katmanı eklendi.`);
    const c = service.connection ? ctx.doc.settings.connections.value.find((x) => x.id === service.connection) : undefined;
    if (c && mod) {
      const secret = ctx.secrets.get(c.origin, c.id);
      const why = mod.authMissing(JSON.stringify(c), secret ? JSON.stringify(secret) : undefined);
      if (why) ctx.log.warn(why);
    }
    dialog.close();
  }

  add.addEventListener('click', write);
  cancel.addEventListener('click', () => dialog.close());
  const dialog = new Dialog({
    title: changing ? 'Servis ayarları' : SERVICE_TITLE,
    width: 900,
    className: 'dialog--service',
    content: [h('div', { class: 'svc-panes' }, kinds, right), summary],
    footer: [cancel, add],
    onClose: () => forget(),
  });
  render();
  if (kindNow() === 'google' && !changing) void runConnect();
}
