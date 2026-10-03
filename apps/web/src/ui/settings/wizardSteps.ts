import cadArt from './art/project-cad.svg?raw';
import gisArt from './art/project-gis.svg?raw';
import { DRAWING_FONTS } from '../../app/appearance';
import { fixed } from '../../core/displayNumber';
import { DATUM_LABEL } from '../../geo/crs';
import { searchProvinces } from '../../geo/provinces';
import { draftProvince, draftScale, draftScales, draftSrid, scaleText, summary, systemChoices, UNITS, type CadCoords, type WizardDraft } from '../../model/newProjectWizard';
import type { DrawingFont } from '../../model/projectSettings';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { workspacePicker } from './workspacePicker';
import { zoneStrip } from './zoneStrip';

/**
 * The Yeni proje wizard's pages (docs/adr/0165 §3): the project's type on cards with their pictures; its coordinates
 * (a CAD project local in its unit or with real coordinates; a province and a system, the zone suggested and shown on
 * a strip of Türkiye's longitudes); its scale, name, typeface and a summary. The rules are model/newProjectWizard.ts.
 */

/** What a page may do: change the draft (and draw the page again, or only the rail), go on. */
export interface StepApi {
  readonly draft: WizardDraft;
  set(patch: Partial<WizardDraft>, redraw?: boolean): void;
  next(): void;
  /** What a page keeps while the wizard is open (the province search). */
  readonly keep: { query: string };
}

/** A picture of the app's own (art/*.svg) in a card. */
function art(markup: string): Node {
  const span = document.createElement('span');
  span.className = 'wiz-art';
  span.innerHTML = markup;
  return span;
}

const head = (title: string, lead: string) => h('header', { class: 'wiz__head' }, h('h3', { class: 'wiz__title' }, title), h('p', { class: 'wiz__lead' }, lead));
const section = (label: string, ...content: Child[]) => h('section', { class: 'wiz__section' }, h('h4', { class: 'wiz__label' }, label), ...content);

/** Cards to choose one of (a radio group): ←/→ move, the chosen one outlined in the accent. */
function choices<T extends string>(items: readonly { id: T; mark: Child; name: string; note: string }[], value: T, onChange: (id: T) => void, label: string, kind: string): HTMLElement {
  const group = h('div', { class: `wiz__choices wiz__choices--${kind}`, role: 'radiogroup', 'aria-label': label });
  const buttons = items.map((it) => {
    const on = it.id === value;
    const b = h(
      'button',
      { class: 'wiz__choice', type: 'button', role: 'radio', 'aria-checked': String(on), tabindex: on ? '0' : '-1', dataset: { id: it.id } },
      h('span', { class: 'wiz__mark', 'aria-hidden': 'true' }, it.mark),
      h('span', { class: 'wiz__cname' }, it.name),
      h('span', { class: 'wiz__cnote' }, it.note),
      h('span', { class: 'wiz__check', 'aria-hidden': 'true' }, icon('check', 12)),
    );
    b.addEventListener('click', () => onChange(it.id));
    return b;
  });
  group.append(...buttons);
  group.addEventListener('keydown', (e) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const at = items.findIndex((it) => it.id === value);
    onChange(items[(at + step + items.length) % items.length].id);
  });
  return group;
}

export function typeStep(api: StepApi): Child[] {
  return [
    head('Ne tür bir proje?', 'Tür; sahneyi, eksenleri ve şeridi belirler. Proje ayarları’ndan sonra da değiştirilebilir.'),
    workspacePicker({
      value: api.draft.type,
      // Another type offers other scales: the chosen one goes back to the type's own.
      onChange: (id) => api.set({ type: id === 'cad' ? 'cad' : 'gis', plotScale: null }, false),
      art: (id) => (id === 'cad' ? art(cadArt) : id === 'gis' ? art(gisArt) : null),
      onChoose: () => api.next(),
    }),
  ];
}

export function coordsStep(api: StepApi): Child[] {
  const d = api.draft;
  if (d.type === 'gis') return [head('Konum ve koordinat sistemi', 'İli seçin: proje il merkezinde açılır ve ilin TM dilimi önerilir.'), ...place(api)];
  const coords = choices<CadCoords>(
    [
      { id: 'local', mark: icon('target', 22), name: 'Yerel', note: 'Koordinat sistemi yok; çizim 0,0’dan başlar, AutoCAD’deki gibi.' },
      { id: 'real', mark: icon('crs', 22), name: 'Gerçek koordinatlı', note: 'TM ya da UTM dilimi; ölçme, aplikasyon ve imar çizimleri.' },
    ],
    d.coords,
    (v) => api.set({ coords: v, plotScale: null }),
    'Koordinatlar',
    'coords',
  );
  const parts: Child[] = [head('Koordinatlar ve birim', 'Bir parça ya da yapı çiziyorsanız yerel çalışın; arazideki bir yeri çiziyorsanız gerçek koordinatlarla.'), section('Koordinatlar', coords)];
  if (d.coords === 'local') {
    const units = choices(
      UNITS.map((u) => ({ id: u.id, mark: h('span', { class: 'wiz__unitmark' }, u.mark), name: u.name, note: u.note })),
      d.unit,
      (u) => api.set({ unit: u }),
      'Çizim birimi',
      'units',
    );
    parts.push(section('Çizim birimi', units, h('p', { class: 'wiz__hint' }, 'Uzunluklar, koordinatlar ve alanlar bu birimle yazılır ve gösterilir; çizim kendi içinde metrede saklanır.')));
  } else parts.push(...place(api));
  return parts;
}

/** The province search and list, the systems with the suggested one first, and the zone strip. */
function place(api: StepApi): Child[] {
  const d = api.draft;
  const search = h('input', { class: 'field wiz__search', type: 'search', placeholder: 'İl adı ya da plaka kodu', 'aria-label': 'İl ara', value: api.keep.query, spellcheck: 'false' });
  const list = h('div', { class: 'wiz__list', role: 'listbox', 'aria-label': 'İller' });
  const fill = () => {
    const found = searchProvinces(api.keep.query);
    replaceChildren(
      list,
      found.length
        ? found.map((p) => {
            const row = h(
              'button',
              { class: 'wiz__row', type: 'button', role: 'option', 'aria-selected': String(p.code === d.province), dataset: { code: String(p.code) } },
              h('span', { class: 'wiz__code' }, String(p.code).padStart(2, '0')),
              h('span', { class: 'wiz__rowname' }, p.name),
            );
            // Another province suggests its own zone: a system chosen before follows it again.
            row.addEventListener('click', () => api.set({ province: p.code === d.province ? null : p.code, srid: null }));
            return row;
          })
        : [h('p', { class: 'wiz__empty' }, 'Bu adla il yok.')],
    );
  };
  fill();
  search.addEventListener('input', () => {
    api.keep.query = search.value;
    fill();
  });
  search.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const first = searchProvinces(api.keep.query)[0];
    if (first) api.set({ province: first.code, srid: null });
  });
  const chosen = draftSrid(d);
  const systems = h('div', { class: 'wiz__list wiz__list--systems', role: 'listbox', 'aria-label': 'Koordinat sistemleri' });
  let datum = '';
  for (const { crs, suggested } of systemChoices(d)) {
    if (!suggested && crs.datum !== datum) {
      datum = crs.datum;
      systems.append(h('div', { class: 'wiz__group' }, DATUM_LABEL[crs.datum]));
    }
    const row = h(
      'button',
      { class: 'wiz__row', type: 'button', role: 'option', 'aria-selected': String(crs.srid === chosen), dataset: { srid: String(crs.srid) } },
      h('span', { class: 'wiz__code' }, String(crs.srid)),
      h('span', { class: 'wiz__rowname' }, crs.name),
      suggested ? h('span', { class: 'wiz__badge' }, 'Önerilen') : h('span', { class: 'wiz__area' }, crs.area),
    );
    row.addEventListener('click', () => api.set({ srid: crs.srid }));
    systems.append(row);
  }
  const p = draftProvince(d);
  return [
    h(
      'div',
      { class: 'wiz__place' },
      section('İl', search, list),
      section('Koordinat sistemi', systems),
    ),
    section(
      p ? `${p.name}, ${fixed(p.lon, 2)}° D` : 'TM3 dilimleri',
      h('div', { class: 'wiz__strip' }, zoneStrip({ srid: chosen, province: p, onPick: (srid) => api.set({ srid }) })),
    ),
  ];
}

export function detailsStep(api: StepApi, current: Child): Child[] {
  const d = api.draft;
  const name = h('input', { class: 'field wiz__name', type: 'text', value: d.name, 'aria-label': 'Proje adı', spellcheck: 'false' });
  name.addEventListener('input', () => api.set({ name: name.value }, false));
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      api.next();
    }
  });
  const scale = draftScale(d);
  const listed = draftScales(d);
  const chips = h('div', { class: 'wiz__scales', role: 'radiogroup', 'aria-label': 'Ölçek' });
  for (const n of listed) {
    const c = h('button', { class: 'wiz__chip', type: 'button', role: 'radio', 'aria-checked': String(n === scale) }, scaleText(n));
    c.addEventListener('click', () => api.set({ plotScale: n }));
    chips.append(c);
  }
  // Any other scale, typed: 1:N with N a whole number over 0.
  const own = h('input', { class: 'field wiz__own', type: 'text', inputmode: 'numeric', value: listed.includes(scale) ? '' : String(scale), placeholder: 'başka', 'aria-label': 'Başka ölçek (1:N)' });
  own.addEventListener('change', () => {
    const n = Number(own.value.replace(/[.\s]/g, ''));
    if (Number.isInteger(n) && n > 0) api.set({ plotScale: n });
    else own.value = '';
  });
  chips.append(h('label', { class: 'wiz__ownfield' }, h('span', null, '1:'), own));
  const font = h('select', { class: 'field wiz__font', 'aria-label': 'Çizim yazı tipi' }, ...DRAWING_FONTS.map((f) => h('option', { value: f.id, selected: f.id === d.font }, f.label)));
  font.addEventListener('change', () => api.set({ font: font.value as DrawingFont }, false));
  const lines = summary(d);
  return [
    head('Ölçek ve ayrıntılar', d.type === 'cad' ? 'Çizim ölçeği yazıları, ölçüleri ve kalemleri kâğıda göre boyutlandırır.' : 'Harita ölçeği yazıları, sembolleri ve paftayı boyutlandırır.'),
    section('Proje adı', name),
    section(d.type === 'cad' ? 'Çizim ölçeği' : 'Harita ölçeği', chips),
    section('Çizim yazı tipi', font),
    section('Özet', h('dl', { class: 'wiz__summary' }, ...lines.flatMap(([k, v]) => [h('dt', null, k), h('dd', null, v)]))),
    current,
  ];
}
