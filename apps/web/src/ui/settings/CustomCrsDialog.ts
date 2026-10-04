import type { Convention } from '../../contracts/generated/Convention';
import type { CrsDefinition } from '../../contracts/generated/CrsDefinition';
import { CRS_REGISTRY, crsTitle } from '../../geo/crs';
import {
  AFFINE,
  buildDefinition,
  ELLIPSOIDS,
  emptyForm,
  formOf,
  PARAMETERS,
  type Coefficient,
  type DatumPick,
  type DefinitionForm,
  type Kind,
  type Parameter,
  type PlaneKind,
  type Problems,
} from '../../model/definitionForm';
import { h, replaceChildren, type Child } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { note, segmented, textField } from '../widgets/controls';

/**
 * Özel koordinat sistemi (docs/adr/0168 §1–§2, §6; the desktop's `project/custom_crs.rs`): the project's own system or
 * its second system typed as a definition: its name, its kind (a transverse Mercator, a geographic system, a local
 * system bound to a base), the kind's values, its datum (the registry's, or the project's: an ellipsoid and seven
 * parameters to WGS 84), a local system's base and plane. What is typed is checked field by field
 * (model/definitionForm.ts, the shared cases'), and a definition the registry has is said. It opens over Proje
 * ayarları: Tamam puts the definition in its draft, Kaydet there assigns it; the drawing is not transformed.
 */
export interface CustomCrsOptions {
  /** Whose definition: the project's own system or its second. */
  readonly target: 'own' | 'second';
  /** The definition there is; null: a new one. */
  readonly existing: CrsDefinition | null;
  onDone(definition: CrsDefinition): void;
}

/** The seven parameters' captions (Datum dönüşümleri's too). */
export const PARAMETER_CAPTIONS: Record<Parameter, string> = { tx: 'ΔX (m)', ty: 'ΔY (m)', tz: 'ΔZ (m)', rx: 'rX (″)', ry: 'rY (″)', rz: 'rZ (″)', ds: 'Ölçek farkı (ppm)' };

/** A rotation convention's name (Datum dönüşümleri's too). */
export const CONVENTIONS: { value: Convention; label: string }[] = [
  { value: 'positionVector', label: 'Konum vektörü (9606)' },
  { value: 'coordinateFrame', label: 'Koordinat çerçevesi (9607)' },
];

const LEADS = {
  own: "Kayıtta olmayan bir sistemi projenin sistemi olarak tanımlayın: tanım projeyle saklanır ve paylaşılır. Tamam tanımı Proje ayarları'na yazar, Kaydet onu atar; koordinatlar dönüştürülmez.",
  second:
    "Kayıtta olmayan bir sistemi ikinci sistem olarak tanımlayın: değerleri durum çubuğunda ve Koordinat oku'da projeninkilerin yanında gösterilir. Tamam tanımı Proje ayarları'na yazar, Kaydet onu atar; çizim dönüştürülmez.",
} as const;

export function openCustomCrs(o: CustomCrsOptions): void {
  const f: DefinitionForm = o.existing ? formOf(o.existing) : emptyForm();
  // A definition from a file whose base is itself a definition: the window cannot show that base, so the definition
  // stays as it is until something is typed.
  let kept: CrsDefinition | null = o.existing?.system.kind === 'local' && o.existing.system.base.definition ? o.existing : null;
  let problems: Problems = {};
  let said: string | null = null;
  const problemNodes: { key: string; el: HTMLElement }[] = [];
  const noteSlot = h('div', null);
  const body = h('div', { class: 'custom-crs' });
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');

  const check = () => {
    const got = buildDefinition(f);
    problems = 'problems' in got ? got.problems : {};
    said = 'problems' in got ? null : (got.note ?? null);
  };
  const paint = () => {
    for (const p of problemNodes) p.el.textContent = problems[p.key] ?? '';
    replaceChildren(noteSlot, said ? note('info', said) : null);
    ok.disabled = Object.keys(problems).length > 0;
  };
  // Something typed or chosen: a definition kept from a file goes, the form is checked; a choice draws the form again.
  const changed = (redraw: boolean) => {
    kept = null;
    check();
    if (redraw) render();
    else paint();
  };

  const caption = (text: string) => h('span', { class: 'datum-field__caption' }, text);
  const field = (key: string, label: string, value: string, set: (v: string) => void, size: 'wide' | 'number' | 'scale' | 'coefficient' = 'number') => {
    const input = textField({ label, value, onChange: (v) => (set(v), changed(false)) });
    input.classList.add('datum-field__input');
    const problem = h('div', { class: 'datum-field__problem' });
    problemNodes.push({ key, el: problem });
    return h('label', { class: `datum-field datum-field--${size}` }, caption(label), input, problem);
  };
  const control = (label: string, el: Child, problemKey?: string, size?: 'base') => {
    const problem = problemKey ? h('div', { class: 'datum-field__problem' }) : null;
    if (problem && problemKey) problemNodes.push({ key: problemKey, el: problem });
    return h('div', { class: `datum-field${size ? ` datum-field--${size}` : ''}` }, caption(label), el, problem);
  };
  const heading = (text: string) => h('div', { class: 'custom-crs__heading' }, text);
  const words = (text: string) => h('p', { class: 'custom-crs__note' }, text);
  const row = (...children: Child[]) => h('div', { class: 'datum-fields' }, children);

  const datumPart = (): Child[] => {
    const datums = segmented<DatumPick>({
      label: 'Datum',
      value: f.datum,
      options: [
        { value: 'TUREF', label: 'TUREF' },
        { value: 'ED50', label: 'ED50' },
        { value: 'WGS84', label: 'WGS 84' },
        { value: 'custom', label: 'Projenin datumu' },
      ],
      onChange: (d) => ((f.datum = d), changed(true)),
    });
    if (f.datum !== 'custom') return [heading('Datum'), datums, words("Kayıttaki datum: öbür datumlara EPSG'nin yollarıyla ya da projenin Datum dönüşümleri'yle varılır.")];
    const ellipsoid = new Dropdown({
      ariaLabel: 'Elipsoid',
      width: 220,
      items: (): MenuItem[] => [
        ...ELLIPSOIDS.map(([name, a, rf]): MenuItem => ({ label: name, hint: `a ${a}, 1/f ${rf}`, radio: true, checked: f.ellipsoid === name, run: () => ((f.ellipsoid = name), changed(true)) })),
        {
          label: 'Değerleri yazılan',
          radio: true,
          checked: f.ellipsoid === 'custom',
          run: () => {
            // The typed values start from the ellipsoid chosen before.
            const before = ELLIPSOIDS.find(([name]) => name === f.ellipsoid);
            if (before) [f.semiMajor, f.inverseFlattening] = [String(before[1]), String(before[2])];
            f.ellipsoid = 'custom';
            changed(true);
          },
        },
      ],
    });
    ellipsoid.set(f.ellipsoid === 'custom' ? 'Değerleri yazılan' : f.ellipsoid);
    const fields: Child[] = [field('datumName', 'Datumun adı', f.datumName, (v) => (f.datumName = v), 'wide'), control('Elipsoid', ellipsoid.el)];
    if (f.ellipsoid === 'custom')
      fields.push(
        field('semiMajor', 'a (m)', f.semiMajor, (v) => (f.semiMajor = v), 'scale'),
        field('inverseFlattening', '1/f', f.inverseFlattening, (v) => (f.inverseFlattening = v)),
      );
    const box = h('input', { type: 'checkbox', checked: f.linked, 'aria-label': "WGS 84'e yedi parametreyle bağlı" });
    box.addEventListener('change', () => ((f.linked = box.checked), changed(true)));
    const linked = h('label', { class: 'custom-crs__check' }, box, "WGS 84'e yedi parametreyle bağlı");
    const out: Child[] = [heading('Datum'), datums, row(fields), linked];
    if (!f.linked) return [...out, words('Bağsız datum kendi içinde kalır: başka datumdaki sistemlere değer verilmez.')];
    const parameter = (k: Parameter) => field(k, PARAMETER_CAPTIONS[k], f.parameters[k], (v) => (f.parameters[k] = v), k === 'ds' ? 'scale' : 'number');
    const rule = segmented<Convention>({ label: 'Dönüklüklerin kuralı', value: f.convention, options: CONVENTIONS, onChange: (c) => ((f.convention = c), changed(true)) });
    return [
      ...out,
      row(PARAMETERS.slice(0, 3).map(parameter)),
      row(PARAMETERS.slice(3).map(parameter)),
      row(control('Dönüklüklerin kuralı', rule), field('accuracy', 'Doğruluk (m)', f.accuracy, (v) => (f.accuracy = v))),
      words("Boş dönüklük ve ölçek farkı 0'dır (üç parametre). Öbür datumlara WGS 84 üstünden varılır; doğruluk boşsa bilinmiyor sayılır."),
    ];
  };

  const localPart = (): Child[] => {
    const bases = CRS_REGISTRY.filter((c) => c.kind === 'projected');
    const chosen = bases.find((c) => String(c.srid) === f.base.trim());
    const base = new Dropdown({
      ariaLabel: 'Taban sistem',
      width: 320,
      items: (): MenuItem[] => bases.map((c) => ({ label: c.name, hint: `EPSG:${c.srid}`, radio: true, checked: c === chosen, run: () => ((f.base = String(c.srid)), changed(true)) })),
    });
    base.set(chosen ? crsTitle(chosen) : 'Taban sistemi seçin…');
    const planes = segmented<PlaneKind>({
      label: 'Düzlem dönüşümü',
      value: f.plane,
      options: [
        { value: 'similarity', label: 'Benzerlik' },
        { value: 'affine', label: 'Afin' },
      ],
      onChange: (p) => ((f.plane = p), changed(true)),
    });
    const out: Child[] = [heading('Taban ve düzlem'), row(control('Taban sistem', base.el, 'base', 'base'), control('Düzlem dönüşümü', planes))];
    if (f.plane === 'similarity')
      return [
        ...out,
        row(
          field('east', 'Sağa öteleme (m)', f.east, (v) => (f.east = v), 'scale'),
          field('north', 'Yukarı öteleme (m)', f.north, (v) => (f.north = v), 'scale'),
          field('rotation', 'Dönüklük (°)', f.rotation, (v) => (f.rotation = v)),
          field('scale', 'Ölçek', f.scale, (v) => (f.scale = v)),
        ),
        words("Bu sistemin noktası saat yönünün tersine döndürülür, ölçeklenir, sonra ötelenir: tabandaki yeri çıkar. Boş dönüklük 0, boş ölçek 1'dir."),
      ];
    const coefficient = (k: Coefficient) => field(k, k, f[k], (v) => (f[k] = v), 'coefficient');
    return [
      ...out,
      row(AFFINE.slice(0, 3).map(coefficient)),
      row(AFFINE.slice(3).map(coefficient)),
      words('Tabanda sağa = a·sağa + b·yukarı + c, tabanda yukarı = d·sağa + e·yukarı + f; sağa ve yukarı bu sistemin koordinatlarıdır.'),
    ];
  };

  const render = () => {
    // Keep the keyboard's place across a redraw: the nearest labelled control.
    const active = document.activeElement as HTMLElement | null;
    const focusKey = active && body.contains(active) ? active.closest('[aria-label]')?.getAttribute('aria-label') : null;
    problemNodes.length = 0;
    const kinds = segmented<Kind>({
      label: 'Tür',
      value: f.kind,
      options: [
        { value: 'tm', label: 'TM izdüşümü' },
        { value: 'geographic', label: 'Coğrafi' },
        { value: 'local', label: 'Yerel (taban sisteme bağlı)' },
      ],
      onChange: (k) => ((f.kind = k), changed(true)),
    });
    const parts: Child[] = [
      words(LEADS[o.target]),
      kept ? note('info', 'Bu tanımın tabanı da bir tanım (dosyadan geldi); bu pencere onu gösteremez. Bir alanı değiştirmezseniz tanım olduğu gibi kalır.') : null,
      row(field('name', 'Ad', f.name, (v) => (f.name = v), 'wide'), control('Tür', kinds)),
    ];
    if (f.kind === 'tm')
      parts.push(
        heading('İzdüşüm'),
        row(
          field('centralMeridian', 'Orta meridyen (°)', f.centralMeridian, (v) => (f.centralMeridian = v), 'scale'),
          field('scaleFactor', 'Ölçek', f.scaleFactor, (v) => (f.scaleFactor = v)),
          field('latitudeOfOrigin', 'Başlangıç enlemi (°)', f.latitudeOfOrigin, (v) => (f.latitudeOfOrigin = v), 'scale'),
        ),
        row(
          field('falseEasting', 'Sağa öteleme (m)', f.falseEasting, (v) => (f.falseEasting = v), 'scale'),
          field('falseNorthing', 'Yukarı öteleme (m)', f.falseNorthing, (v) => (f.falseNorthing = v), 'scale'),
        ),
        words("Boş ölçek 1, boş başlangıç enlemi 0'dır. Orta meridyen ve ötelemeler yazılmalı."),
        ...datumPart(),
      );
    else if (f.kind === 'geographic') parts.push(...datumPart());
    else parts.push(...localPart());
    parts.push(noteSlot);
    replaceChildren(body, parts);
    paint();
    if (focusKey) {
      const target = body.querySelector<HTMLElement>(`[aria-label="${CSS.escape(focusKey)}"]`);
      (target?.getAttribute('role') === 'radiogroup' ? target.querySelector<HTMLElement>('[aria-checked="true"]') : target)?.focus();
    }
  };

  const dialog = new Dialog({ title: 'Özel koordinat sistemi', width: 760, className: 'dialog--custom-crs', stack: true, content: [body], footer: [h('div', { class: 'dialog__spacer' }), cancel, ok] });
  cancel.addEventListener('click', () => dialog.close());
  ok.addEventListener('click', () => {
    if (kept) {
      o.onDone(kept);
      dialog.close();
      return;
    }
    const got = buildDefinition(f);
    if ('problems' in got) {
      problems = got.problems;
      paint();
      return;
    }
    o.onDone(got.definition);
    dialog.close();
  });
  // A new window says nothing before anything is typed; a definition is checked as it is.
  if (o.existing && !kept) check();
  render();
}
