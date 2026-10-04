import type { Convention } from '../../contracts/generated/Convention';
import type { CrsDefinition } from '../../contracts/generated/CrsDefinition';
import { CRS_REGISTRY, crsTitle } from '../../geo/crs';
import {
  AFFINE,
  buildDefinition,
  definitionProj,
  definitionWkt,
  ELLIPSOIDS,
  emptyForm,
  formOf,
  PARAMETERS,
  readDefinition,
  sameSrid,
  type Coefficient,
  type DatumPick,
  type DefinitionForm,
  type Kind,
  type Parameter,
  type PlaneKind,
  type Problems,
} from '../../model/definitionForm';
import type { DatumChoice, System } from '../../model/geom/crsTransform';
import { definitionSystem } from '../../model/projectCrs';
import { convertPoint, errorText, type ConvertFormat } from '../calc/convert';
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
 * (model/definitionForm.ts, the shared cases'), and a definition the registry has is said, with Kayıttakini seç. A WKT
 * or PROJ text pasted, or a `.prj` file, fills the fields (docs/adr/0168 §5); the definition is copied as WKT or PROJ; a
 * Deneme noktası shows where a point typed in it is in WGS 84 and in the project's other system, with the project's
 * datum choices. It opens over Proje ayarları: Tamam puts the definition in its draft, Kaydet there assigns it; the
 * drawing is not transformed.
 */
export interface CustomCrsOptions {
  /** Whose definition: the project's own system or its second. */
  readonly target: 'own' | 'second';
  /** The definition there is; null: a new one. */
  readonly existing: CrsDefinition | null;
  /** Where a trial point is compared. */
  readonly trial: TrialContext;
  onDone(definition: CrsDefinition): void;
  /** Kayıttakini seç: the registry's system the definition is. */
  onRegistry(srid: number): void;
  /** What happened, for the log: a text copied, or not. */
  say(kind: 'success' | 'error', text: string): void;
}

/** Where a trial point is compared: the project's other system (named), its datum choices, how it writes points. */
export interface TrialContext {
  readonly reference: { readonly name: string; readonly system: System } | null;
  readonly choices: readonly DatumChoice[];
  readonly format: ConvertFormat;
}

/** The largest `.prj` file read (a definition is a few hundred bytes). */
const PRJ_LIMIT = 1 << 20;
const WGS84: System = { kind: 'geographic', datum: 'WGS84' };

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
  let same: number | null = null;
  // What reading a text said, and a grid the registry has on the text's own datum.
  let read: { readonly ok: boolean; readonly text: string } | null = null;
  let grid: string | null = null;
  const trial = ['', ''];
  const problemNodes: { key: string; el: HTMLElement }[] = [];
  const noteSlot = h('div', { class: 'custom-crs__notes' });
  const body = h('div', { class: 'custom-crs' });
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const wkt = h('button', { class: 'btn', type: 'button' }, 'WKT olarak kopyala');
  const proj = h('button', { class: 'btn', type: 'button' }, 'PROJ olarak kopyala');
  // The box keeps what is pasted across redraws: it is made once.
  const paste = h('textarea', {
    class: 'field custom-crs__paste',
    rows: 3,
    placeholder: 'WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj=… ile başlayan PROJ dizesi yapıştırın',
    'aria-label': 'WKT ya da PROJ metni',
    spellcheck: 'false',
  }) as HTMLTextAreaElement;
  const take = h('button', { class: 'btn btn--small', type: 'button' }, 'Al');
  const pick = h('button', { class: 'btn btn--small', type: 'button' }, '.prj dosyası…');
  const file = h('input', { type: 'file', accept: '.prj,.PRJ,.wkt,.WKT,.txt,.TXT', hidden: true }) as HTMLInputElement;
  const readSaid = h('div', { class: 'custom-crs__read' });
  const trialOut = h('div', { class: 'custom-crs__trial' });

  const check = () => {
    const got = buildDefinition(f);
    problems = 'problems' in got ? got.problems : {};
    said = 'problems' in got ? null : (got.note ?? null);
    same = 'problems' in got ? null : sameSrid(got.definition);
  };
  /** The definition as it stands: the one kept, or the form's when it builds. */
  const definition = (): CrsDefinition | null => {
    if (kept) return kept;
    const got = buildDefinition(f);
    return 'problems' in got ? null : got.definition;
  };
  const paintTrial = () => {
    if (trial.every((t) => !t.trim())) return replaceChildren(trialOut);
    const d = definition();
    const from = d ? definitionSystem(d) : null;
    if (!from) return replaceChildren(trialOut, h('p', { class: 'custom-crs__note' }, 'Önce tanımı tamamlayın; nokta onunla dönüştürülür.'));
    const targets: [string, System][] = [['WGS 84', WGS84], ...(o.trial.reference ? [[o.trial.reference.name, o.trial.reference.system] as [string, System]] : [])];
    replaceChildren(
      trialOut,
      targets.map(([name, to]) => {
        const c = convertPoint(from, to, o.trial.choices, trial[0]!, trial[1]!, o.trial.format);
        const said =
          typeof c === 'string'
            ? h('div', null, errorText(c, from, o.trial.format))
            : [h('div', { class: 'custom-crs__values num' }, c.values.map(([k, v]) => h('span', null, `${k} ${v}`))), h('div', { class: 'custom-crs__note' }, c.accuracy)];
        return [h('div', { class: 'custom-crs__trial-name' }, name), h('div', null, said)];
      }),
    );
  };
  const paint = () => {
    for (const p of problemNodes) p.el.textContent = problems[p.key] ?? '';
    const notes: Child[] = [];
    if (said) {
      const offer = same === null ? null : h('button', { class: 'btn btn--small', type: 'button' }, 'Kayıttakini seç');
      offer?.addEventListener('click', () => {
        if (same === null) return;
        o.onRegistry(same);
        dialog.close();
      });
      notes.push(note('info', h('span', { class: 'custom-crs__said' }, said), offer));
    }
    if (grid) notes.push(note('info', grid));
    replaceChildren(noteSlot, notes);
    ok.disabled = Object.keys(problems).length > 0;
    const whole = definition();
    const local = whole?.system.kind === 'local';
    wkt.disabled = !whole;
    proj.disabled = !whole || local;
    // PROJ cannot write a system derived from another (docs/adr/0168 §5): said where the button waits.
    proj.title = local ? 'Yerel sistem PROJ dizesiyle yazılamaz; WKT olarak kopyalayın.' : '';
    take.disabled = !paste.value.trim();
    replaceChildren(readSaid, read?.text ?? '');
    readSaid.toggleAttribute('data-error', read !== null && !read.ok);
    paintTrial();
  };
  // Something typed or chosen: a definition kept from a file and what a text's datum shares go, the form is checked; a
  // choice draws the form again.
  const changed = (redraw: boolean) => {
    kept = null;
    grid = null;
    check();
    if (redraw) render();
    else paint();
  };
  /** A text read into the fields, or why not (the shared cases'). */
  const takeText = (text: string) => {
    const got = readDefinition(text);
    if ('problem' in got) {
      read = { ok: false, text: got.problem };
      return paint();
    }
    Object.assign(f, formOf(got.definition));
    const d = got.definition;
    kept = d.system.kind === 'local' && d.system.base.definition ? d : null;
    grid = got.note;
    read = { ok: true, text: `“${d.name}” okundu; alanlar metinden dolduruldu.` };
    problems = {};
    said = null;
    same = null;
    if (!kept) check();
    render();
  };
  paste.addEventListener('input', () => (take.disabled = !paste.value.trim()));
  take.addEventListener('click', () => takeText(paste.value));
  pick.addEventListener('click', () => file.click());
  file.addEventListener('change', async () => {
    const chosen = file.files?.[0];
    file.value = '';
    if (!chosen) return;
    if (chosen.size > PRJ_LIMIT) {
      read = { ok: false, text: 'Dosya bir tanım için çok büyük (en çok 1 MiB); .prj dosyasını seçin.' };
      return paint();
    }
    const text = await chosen.text();
    paste.value = text.trim();
    takeText(text);
  });
  const copy = (what: 'WKT' | 'PROJ dizesi', text: string | null) => {
    if (!text) return;
    void navigator.clipboard.writeText(text).then(
      () => o.say('success', `${what} panoya kopyalandı.`),
      () => o.say('error', `${what} panoya yazılamadı: tarayıcı izin vermedi. Pencereyi tıklayıp yeniden deneyin.`),
    );
  };
  wkt.addEventListener('click', () => {
    const d = definition();
    copy('WKT', d && definitionWkt(d));
  });
  proj.addEventListener('click', () => {
    const d = definition();
    copy('PROJ dizesi', d && definitionProj(d));
  });

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

  /** Deneme noktası: a point typed in this system, where it is in WGS 84 and in the project's other system, how sure. */
  const trialPart = (): Child[] => {
    // Named by the kind chosen, not by a whole definition: the names stay as the fields are typed.
    const [a, b] = f.kind === 'geographic' ? ['Enlem', 'Boylam'] : [o.trial.format.east, o.trial.format.north];
    const input = (i: 0 | 1, label: string) => {
      const el = textField({ label: `Deneme noktası ${label}`, value: trial[i]!, onChange: (v) => ((trial[i] = v), paintTrial()) });
      el.classList.add('datum-field__input');
      return h('label', { class: 'datum-field datum-field--scale' }, caption(label), el);
    };
    const where = o.trial.reference ? `WGS 84'teki ve ${o.trial.reference.name} sistemindeki yeri` : "WGS 84'teki yeri";
    return [heading('Deneme noktası'), words(`Bu sistemde bir nokta yazın: ${where}, projenin datum dönüşümleriyle gösterilir.`), row(input(0, a), input(1, b)), trialOut];
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
      heading("WKT ya da PROJ'dan al"),
      paste,
      h('div', { class: 'custom-crs__actions' }, take, pick, file),
      readSaid,
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
    parts.push(noteSlot, ...trialPart());
    replaceChildren(body, parts);
    paint();
    if (focusKey) {
      const target = body.querySelector<HTMLElement>(`[aria-label="${CSS.escape(focusKey)}"]`);
      (target?.getAttribute('role') === 'radiogroup' ? target.querySelector<HTMLElement>('[aria-checked="true"]') : target)?.focus();
    }
  };

  const dialog = new Dialog({ title: 'Özel koordinat sistemi', width: 760, className: 'dialog--custom-crs', stack: true, content: [body], footer: [wkt, proj, h('div', { class: 'dialog__spacer' }), cancel, ok] });
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
