import type { CrsDefinition } from '../../contracts/generated/CrsDefinition';
import { CRS_REGISTRY, crsBySrid, crsCode, crsTitle, DATUM_LABEL, searchCrs, type CrsDef, type Datum } from '../../geo/crs';
import { DEFINITION_CODE, definitionTitle } from '../../model/projectCrs';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';

/**
 * The project's own definition in the picker (docs/adr/0168 §6; Proje ayarları only): its row at the head of the list,
 * chosen or not; Özel sistem… under the list defines one, Düzenle on the chosen one's card edits it.
 */
export interface DefinedOptions {
  /** The definition the window knows; null: the project has none yet. */
  readonly definition: CrsDefinition | null;
  /** It is the draft's system. */
  readonly chosen: boolean;
  /** The project's system was a definition when the window opened. */
  readonly was: boolean;
  /** The draft's system is another than the project's (an edited definition too). */
  readonly changed: boolean;
  onPick(): void;
  onNew(): void;
  onEdit(): void;
}

export interface CrsPickerOptions {
  /** Currently chosen SRID in the draft. */
  value: number;
  /** SRID before the dialog opened (for "will change" state and warnings). */
  initial: number;
  /** SRID to tag as "varsayılan" in the list. */
  defaultSrid?: number;
  /**
   * `assign`: changes the open project's CRS (warns that coordinates are
   * not reprojected). `default`: picks the CRS for future projects. `new`:
   * picks the CRS of a new, empty project (nothing to reproject).
   */
  mode: 'assign' | 'default' | 'new';
  /** Search text survives section re-renders through this holder. */
  state: { query: string };
  /** `rerender: false` means the picker already patched itself. */
  onChange(srid: number, rerender: boolean): void;
  /** The project's own definition (docs/adr/0168 §6). */
  defined?: DefinedOptions;
}

const SRID_RE = /^\s*(epsg:?)?\d{4,5}\s*$/i;
const sridOf = (text: string) => parseInt(text.replace(/\D/g, ''), 10);

/** Searchable EPSG list + parameter card. Used by project and app settings. */
export function crsPicker(o: CrsPickerOptions): Child {
  let value = o.value;
  // The definition while it is the draft's system; a system of the registry picked here takes its place.
  let own = o.defined?.chosen ? o.defined.definition : null;
  const list = h('div', { class: 'crs-list', role: 'listbox', 'aria-label': 'Koordinat sistemleri' });
  const status = h('div', { class: 'crs-search__status' });
  const search = h('input', {
    class: 'field crs-search__input',
    type: 'search',
    value: o.state.query,
    placeholder: 'SRID ya da ad yazın, ör. 5256 veya TM36',
    'aria-label': 'Koordinat sistemi ara',
    spellcheck: 'false',
  });
  const current = h('div', null);
  const details = h('div', { class: 'crs-details' });
  const notes = h('div', { class: 'crs-notes' });

  const patch = () => {
    const c = crsBySrid(value)!;
    const changed = own ? !!o.defined?.changed : value !== o.initial || !!o.defined?.was;
    replaceChildren(current, currentCard(c, changed, o.mode, own, o.defined));
    replaceChildren(details, own ? definitionCard(own) : detailsCard(c));
    replaceChildren(notes, o.mode === 'assign' ? assignNotes(c, crsBySrid(o.initial)!, changed, own, !!o.defined?.was) : o.mode === 'new' ? unitNotes(c) : []);
  };

  const renderList = () => {
    const results = searchCrs(o.state.query);
    const groups = new Map<Datum, CrsDef[]>();
    for (const c of results) groups.set(c.datum, [...(groups.get(c.datum) ?? []), c]);
    const wanted = o.state.query.trim().toLocaleLowerCase('tr-TR');
    const definition = o.defined?.definition;
    const shown = definition && (!wanted || definition.name.toLocaleLowerCase('tr-TR').includes(wanted)) ? definition : null;
    const definedRow = shown
      ? [
          h('div', { class: 'crs-list__group' }, 'Projenin tanımı'),
          (() => {
            const row = h(
              'button',
              { class: 'crs-row', type: 'button', role: 'option', 'aria-selected': String(own !== null) },
              h('span', { class: 'crs-row__code crs-row__code--icon' }, icon('crs', 15)),
              h('span', { class: 'crs-row__name' }, shown.name),
              h('span', { class: 'crs-row__area' }, DEFINITION_CODE),
            );
            row.addEventListener('click', () => o.defined?.onPick());
            return row;
          })(),
        ]
      : [];
    replaceChildren(
      list,
      definedRow,
      results.length
        ? [...groups].map(([datum, items]) => [
            h('div', { class: 'crs-list__group' }, DATUM_LABEL[datum]),
            items.map((c) => {
              const row = h(
                'button',
                { class: 'crs-row', type: 'button', role: 'option', 'aria-selected': String(own === null && c.srid === value) },
                h('span', { class: 'crs-row__code num' }, String(c.srid)),
                h('span', { class: 'crs-row__name' }, c.name, c.srid === o.defaultSrid ? h('span', { class: 'crs-row__tag' }, 'varsayılan') : null),
                h('span', { class: 'crs-row__area' }, c.area),
              );
              row.addEventListener('click', () => o.onChange(c.srid, true));
              return row;
            }),
          ])
        : definedRow.length
          ? null
          : h('div', { class: 'crs-list__empty' }, 'Eşleşen koordinat sistemi yok.'),
    );
    // Only the list scrolls to the chosen row: scrollIntoView would also scroll the dialog around it.
    queueMicrotask(() => {
      const row = list.querySelector<HTMLElement>('[aria-selected="true"]');
      if (!row) return;
      const r = row.getBoundingClientRect();
      const l = list.getBoundingClientRect();
      if (r.top < l.top) list.scrollTop -= l.top - r.top;
      else if (r.bottom > l.bottom) list.scrollTop += r.bottom - l.bottom;
    });
    const code = sridOf(o.state.query);
    status.textContent = SRID_RE.test(o.state.query) && !crsBySrid(code) ? `EPSG:${code} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin.` : '';
  };

  search.addEventListener('input', () => {
    o.state.query = search.value;
    const code = sridOf(search.value);
    // Typing an exact known SRID selects it without stealing focus.
    if (SRID_RE.test(search.value) && crsBySrid(code) && (code !== value || own)) {
      value = code;
      own = null;
      o.onChange(code, false);
      patch();
    }
    renderList();
  });

  patch();
  renderList();
  // Özel sistem… under the list, as a list's command is (the desktop's picker).
  let listbox: Child = list;
  if (o.defined) {
    const add = h('button', { class: 'crs-list__action', type: 'button' }, icon('plus', 14), 'Özel sistem…');
    add.addEventListener('click', () => o.defined?.onNew());
    listbox = h('div', { class: 'crs-listbox' }, list, add);
  }
  return [
    current,
    h(
      'div',
      { class: 'crs-browser' },
      h('div', { class: 'crs-browser__list' }, h('div', { class: 'crs-search' }, h('span', { class: 'crs-search__icon' }, icon('search', 15)), search), status, listbox),
      details,
    ),
    notes,
  ];
}

function currentCard(c: CrsDef, changed: boolean, mode: CrsPickerOptions['mode'], own: CrsDefinition | null, defined: DefinedOptions | undefined): Child {
  const label =
    mode === 'new'
      ? 'Yeni projenin koordinat sistemi'
      : mode === 'assign'
        ? changed
          ? 'Kaydedince projeye atanacak sistem'
          : 'Projenin koordinat sistemi'
        : changed
          ? 'Kaydedince yeni projelerde kullanılacak'
          : 'Yeni projelerde kullanılan sistem';
  // A new project has nothing to change: its system is chosen, not about to change (no amber "değişecek").
  let edit: HTMLElement | null = null;
  if (own) {
    edit = h('button', { class: 'btn btn--small crs-current__edit', type: 'button' }, 'Düzenle');
    edit.addEventListener('click', () => defined?.onEdit());
  }
  return h(
    'div',
    { class: 'crs-current', 'data-changed': changed && mode !== 'new' ? '' : null },
    h('div', { class: 'crs-current__label' }, label),
    h(
      'div',
      { class: 'crs-current__row' },
      h('span', { class: 'crs-current__name' }, own ? own.name : c.name),
      h('span', { class: 'crs-chip num' }, own ? DEFINITION_CODE : crsCode(c)),
      edit,
    ),
  );
}

const KIND_LABEL: Record<CrsDef['kind'], string> = {
  projected: 'Projeksiyonlu (metre)',
  geographic: 'Coğrafi (derece)',
  local: 'Yerel: koordinat sistemi yok (metre)',
};

function detailsCard(c: CrsDef): Child {
  const rows: [string, string][] = [['Tür', KIND_LABEL[c.kind]]];
  if (c.kind !== 'local') rows.push(['Datum', DATUM_LABEL[c.datum]]);
  if (c.ellipsoid) rows.push(['Elipsoid', c.ellipsoid]);
  if (c.projection) rows.push(['Projeksiyon', c.projection === 'UTM' ? 'UTM (Transverse Mercator)' : c.projection]);
  if (c.centralMeridian !== undefined) rows.push(['Orta meridyen', `${c.centralMeridian}° D`]);
  if (c.scaleFactor !== undefined) rows.push(['Ölçek faktörü', String(c.scaleFactor)]);
  if (c.falseEasting !== undefined) rows.push(['Sağa öteleme', `${c.falseEasting.toLocaleString('tr-TR')} m`]);
  rows.push(['Kapsam', c.area]);
  return [
    h('div', { class: 'crs-details__title' }, c.name),
    h('dl', { class: 'crs-details__grid' }, rows.map(([k, v]) => [h('dt', null, k), h('dd', null, v)])),
    c.kind === 'projected' ? h('p', { class: 'crs-details__axis' }, 'Eksen sırası: Y sağa değer, X yukarı değer.') : null,
  ];
}

function assignNotes(c: CrsDef, initial: CrsDef, changed: boolean, own: CrsDefinition | null, was: boolean): Child[] {
  const out: Child[] = [];
  if (changed) {
    // A datum's change is said between the registry's systems only.
    const datumNote = !own && !was && c.datum !== initial.datum ? ` ${DATUM_LABEL[initial.datum]} → ${DATUM_LABEL[c.datum]} geçişi için datum dönüşümü gerekir (geliştirme aşamasında).` : '';
    out.push(
      note('warn', h('strong', null, 'Koordinatlar dönüştürülmez. '), `Kaydettiğinizde proje yalnızca ${own ? own.name : c.name} olarak etiketlenir; mevcut Y/X değerleri aynı kalır.`, datumNote),
    );
  }
  if (own) {
    if (own.system.kind === 'geographic') out.push(note('info', 'Coğrafi sistemlerde birim derecedir. Çizim ve ölçüm araçları metre cinsinden projeksiyonlu bir sistem bekler.'));
    return out;
  }
  out.push(...unitNotes(c));
  return out;
}

/** The registry's ellipsoid of a datum, as its systems name it. */
const datumEllipsoid = (datum: Datum): string | undefined => CRS_REGISTRY.find((c) => c.datum === datum && c.ellipsoid)?.ellipsoid;

/**
 * A definition's card (docs/adr/0168 §6; the desktop's `definition_details`): its kind, its datum (a project's own with
 * its ellipsoid and its link to WGS 84), its projection's values or a local system's base and plane. Values as typed.
 */
function definitionCard(d: CrsDefinition): Child {
  const s = d.system;
  const rows: [string, string][] = [['Tür', s.kind === 'tm' ? 'TM izdüşümü (metre)' : s.kind === 'geographic' ? 'Coğrafi (derece)' : 'Yerel, taban sisteme bağlı (metre)']];
  if (s.kind !== 'local') {
    if (s.datum !== undefined) {
      rows.push(['Datum', DATUM_LABEL[s.datum]]);
      const e = datumEllipsoid(s.datum);
      if (e) rows.push(['Elipsoid', e]);
    } else if (s.customDatum) {
      const c = s.customDatum;
      const h7 = c.toWgs84;
      rows.push(['Datum', `${c.name} (projenin)`], ['Elipsoid', c.ellipsoid.name]);
      rows.push([
        "WGS 84'e",
        h7 ? `7 parametre, ${h7.convention === 'positionVector' ? 'konum vektörü' : 'koordinat çerçevesi'}${h7.accuracy === undefined ? '' : `, ±${h7.accuracy} m`}` : 'bağı yok',
      ]);
    }
  }
  if (s.kind === 'tm') {
    rows.push(['Projeksiyon', 'Transverse Mercator'], ['Orta meridyen', `${s.centralMeridian}° D`], ['Ölçek faktörü', String(s.scaleFactor)], ['Sağa öteleme', `${s.falseEasting} m`]);
    if (s.falseNorthing !== 0) rows.push(['Yukarı öteleme', `${s.falseNorthing} m`]);
    if (s.latitudeOfOrigin !== undefined) rows.push(['Başlangıç enlemi', `${s.latitudeOfOrigin}°`]);
  } else if (s.kind === 'local') {
    const base = s.base.definition ? definitionTitle(s.base.definition) : s.base.srid !== undefined ? (crsBySrid(s.base.srid) ? crsTitle(crsBySrid(s.base.srid)!) : `EPSG:${s.base.srid}`) : '';
    if (base) rows.push(['Taban', base]);
    rows.push(['Düzlem', s.plane.kind === 'similarity' ? 'Benzerlik' : 'Afin']);
  }
  return [
    h('div', { class: 'crs-details__title' }, d.name),
    h('dl', { class: 'crs-details__grid' }, rows.map(([k, v]) => [h('dt', null, k), h('dd', null, v)])),
    s.kind !== 'geographic' ? h('p', { class: 'crs-details__axis' }, 'Eksen sırası: Y sağa değer, X yukarı değer.') : null,
  ];
}

function unitNotes(c: CrsDef): Child[] {
  if (c.kind === 'local')
    return [
      note(
        'info',
        'Yerel: koordinatlar bir konuma bağlı değildir (teknik çizim, başlangıç 0,0). Koordinat sistemi taşıyan CBS verisi olduğu gibi gelir; dışa aktarılan CBS dosyalarında koordinat sistemi yazılmaz. Gerçek konum için bir sistem seçin.',
      ),
    ];
  return c.kind === 'geographic' ? [note('info', 'Coğrafi sistemlerde birim derecedir. Çizim ve ölçüm araçları metre cinsinden projeksiyonlu bir sistem bekler.')] : [];
}
