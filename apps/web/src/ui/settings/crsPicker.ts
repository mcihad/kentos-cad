import { crsBySrid, DATUM_LABEL, searchCrs, type CrsDef, type Datum } from '../../geo/crs';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';

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
}

const SRID_RE = /^\s*(epsg:?)?\d{4,5}\s*$/i;
const sridOf = (text: string) => parseInt(text.replace(/\D/g, ''), 10);

/** Searchable EPSG list + parameter card. Used by project and app settings. */
export function crsPicker(o: CrsPickerOptions): Child {
  let value = o.value;
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
    replaceChildren(current, currentCard(c, value !== o.initial, o.mode));
    replaceChildren(details, detailsCard(c));
    replaceChildren(notes, o.mode === 'assign' ? assignNotes(c, crsBySrid(o.initial)!) : o.mode === 'new' ? unitNotes(c) : []);
  };

  const renderList = () => {
    const results = searchCrs(o.state.query);
    const groups = new Map<Datum, CrsDef[]>();
    for (const c of results) groups.set(c.datum, [...(groups.get(c.datum) ?? []), c]);
    replaceChildren(
      list,
      results.length
        ? [...groups].map(([datum, items]) => [
            h('div', { class: 'crs-list__group' }, DATUM_LABEL[datum]),
            items.map((c) => {
              const row = h(
                'button',
                { class: 'crs-row', type: 'button', role: 'option', 'aria-selected': String(c.srid === value) },
                h('span', { class: 'crs-row__code num' }, String(c.srid)),
                h('span', { class: 'crs-row__name' }, c.name, c.srid === o.defaultSrid ? h('span', { class: 'crs-row__tag' }, 'varsayılan') : null),
                h('span', { class: 'crs-row__area' }, c.area),
              );
              row.addEventListener('click', () => o.onChange(c.srid, true));
              return row;
            }),
          ])
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
    if (SRID_RE.test(search.value) && crsBySrid(code) && code !== value) {
      value = code;
      o.onChange(code, false);
      patch();
    }
    renderList();
  });

  patch();
  renderList();
  return [
    current,
    h(
      'div',
      { class: 'crs-browser' },
      h('div', { class: 'crs-browser__list' }, h('div', { class: 'crs-search' }, h('span', { class: 'crs-search__icon' }, icon('search', 15)), search), status, list),
      details,
    ),
    notes,
  ];
}

function currentCard(c: CrsDef, changed: boolean, mode: CrsPickerOptions['mode']): Child {
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
  return h(
    'div',
    { class: 'crs-current', 'data-changed': changed && mode !== 'new' ? '' : null },
    h('div', { class: 'crs-current__label' }, label),
    h('div', { class: 'crs-current__row' }, h('span', { class: 'crs-current__name' }, c.name), h('span', { class: 'crs-chip num' }, `EPSG:${c.srid}`)),
  );
}

function detailsCard(c: CrsDef): Child {
  const rows: [string, string][] = [
    ['Tür', c.kind === 'projected' ? 'Projeksiyonlu (metre)' : 'Coğrafi (derece)'],
    ['Datum', DATUM_LABEL[c.datum]],
    ['Elipsoid', c.ellipsoid],
  ];
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

function assignNotes(c: CrsDef, initial: CrsDef): Child[] {
  const out: Child[] = [];
  if (c.srid !== initial.srid) {
    out.push(
      note(
        'warn',
        h('strong', null, 'Koordinatlar dönüştürülmez. '),
        `Kaydettiğinizde proje yalnızca ${c.name} olarak etiketlenir; mevcut Y/X değerleri aynı kalır.`,
        c.datum !== initial.datum ? ` ${DATUM_LABEL[initial.datum]} → ${DATUM_LABEL[c.datum]} geçişi için datum dönüşümü gerekir (geliştirme aşamasında).` : '',
      ),
    );
  }
  out.push(...unitNotes(c));
  return out;
}

function unitNotes(c: CrsDef): Child[] {
  return c.kind === 'geographic' ? [note('info', 'Coğrafi sistemlerde birim derecedir. Çizim ve ölçüm araçları metre cinsinden projeksiyonlu bir sistem bekler.')] : [];
}
